//! Synchronous and asynchronous ASCII Modbus clients.
//!
//! This module is available when the `ascii` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps request PDUs in the
//! ASCII ADU format, validates responses, and exposes high-level methods for
//! reading and writing coils and registers.

#![cfg(all(feature = "ascii", any(feature = "sync", feature = "async")))]

use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use crate::client::ClientConfig;
use crate::macros::impl_adu_adapter;

#[cfg(feature = "sync")]
use crate::client::ClientCore;
#[cfg(feature = "sync")]
use crate::transport::Transport;

#[cfg(feature = "async")]
use crate::client::AsyncClientCore;
#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// Configuration for a synchronous and asynchronous ASCII client.
pub type AsciiClientConfig = crate::client::ClientConfig;

/// Errors that can occur while using the ASCII client.
pub type AsciiClientError = crate::client::ClientError;

#[cfg(feature = "sync")]
impl_adu_adapter! {
    [] [],
    /// Synchronous ASCII ADU adapter.
    AsciiAduAdapter,
    crate::ascii::AsciiAdu,
    no_transaction
}

/// A synchronous ASCII Modbus client.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct AsciiClient<T: Transport>(ClientCore<AsciiAduAdapter<T>>);

#[cfg(feature = "sync")]
impl<T: Transport> AsciiClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: AsciiClientConfig) -> Self {
        Self(ClientCore::with_config(
            AsciiAduAdapter::with_config(transport, config),
            config,
        ))
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> Deref for AsciiClient<T> {
    type Target = ClientCore<AsciiAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> DerefMut for AsciiClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(feature = "ascii", feature = "sync-serial"))]
impl
    AsciiClient<
        crate::ascii_transport::AsciiTransport<
            crate::serial_transport::SerialTransport<Box<dyn serialport::SerialPort>>,
        >,
    >
{
    /// Open a local serial port and return an ASCII client.
    ///
    /// `path` is the platform-specific serial device name (e.g. `/dev/ttyUSB0`
    /// on Linux or `COM3` on Windows). The port is configured for 8 data bits,
    /// no parity, 1 stop bit, and a 100 ms read timeout.
    pub fn connect_serial_ascii(
        path: impl AsRef<std::path::Path>,
        baud_rate: u32,
        config: AsciiClientConfig,
    ) -> Result<Self, AsciiClientError> {
        let mut serial = crate::serial_transport::open_serial_port(path, baud_rate).map_err(|e| {
            AsciiClientError::Transport(crate::transport::TransportError::Io(e.into()))
        })?;
        let serial_timeout = config.idle_timeout.unwrap_or(config.timeout);
        serial
            .inner_mut()
            .set_timeout(serial_timeout)
            .map_err(|e| AsciiClientError::Transport(crate::transport::TransportError::Io(e.into())))?;
        let transport = crate::ascii_transport::AsciiTransport::new(serial);
        Ok(Self::with_config(transport, config))
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::ascii::AsciiAdu;
    use crate::client::AduAdapter;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::{Transport, TransportError};
    use crate::{ExceptionResponse, ReadCoilsRequest, ReadCoilsResponse};
    use core::time::Duration;

    struct LoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl LoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for LoopbackTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = AsciiAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[test]
    fn read_coils_over_ascii() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let mut client = AsciiClient::new(LoopbackTransport::new(server));
        let coils = client.read_coils(0x01, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[test]
    fn write_and_read_holding_register_over_ascii() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = AsciiClient::new(LoopbackTransport::new(server));
        client.write_register(0x01, 1, 0x1234).unwrap();
        let bytes = client.read_holding_registers(0x01, 1, 1).unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[test]
    fn dispatch_returns_exception() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let exception_pdu = {
            let exc =
                ExceptionResponse::new(0x01, crate::exception::ExceptionCode::IllegalDataAddress);
            let mut buf = [0u8; 2];
            let n = exc.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = AsciiAdu::new(0x01, exception_pdu);
            let mut buf = [0u8; 32];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsciiClient::new(MockTransport::new(vec![response_adu]));
        let err = client.dispatch(0x01, &request_pdu).unwrap_err();
        assert!(matches!(err, AsciiClientError::Exception(_)));
    }

    #[test]
    fn dispatch_rejects_wrong_slave() {
        let response_adu = {
            let pdu = {
                let resp = ReadCoilsResponse {
                    coil_status: vec![0x01],
                };
                let mut buf = [0u8; 3];
                let n = resp.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };
            let adu = AsciiAdu::new(0x02, pdu);
            let mut buf = [0u8; 32];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsciiClient::new(MockTransport::new(vec![response_adu]));
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01])
            .unwrap_err();
        assert!(matches!(err, AsciiClientError::InvalidResponse));
    }

    #[test]
    fn broadcast_skips_recv_and_returns_empty_pdu() {
        use super::AsciiAduAdapter;

        let request_pdu = {
            let req = ReadCoilsRequest::new(0, 8).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut adapter =
            AsciiAduAdapter::with_config(MockTransport::new(vec![]), AsciiClientConfig::default());
        let response = adapter.send_receive(0, &request_pdu).unwrap();

        assert!(response.is_empty());
        assert_eq!(adapter.transport.sent.len(), 1);

        let sent = AsciiAdu::decode(&adapter.transport.sent[0]).unwrap();
        assert_eq!(sent.address, 0);
        assert_eq!(sent.pdu, request_pdu);
    }

    struct MockTransport {
        sent: Vec<Vec<u8>>,
        responses: alloc::collections::VecDeque<Vec<u8>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                sent: Vec::new(),
                responses: responses.into(),
            }
        }
    }

    impl Transport for MockTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }

        fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
            let resp = self
                .responses
                .pop_front()
                .ok_or(TransportError::Disconnected)?;
            let n = resp.len().min(buf.len());
            buf[..n].copy_from_slice(&resp[..n]);
            Ok(n)
        }
    }
}

#[cfg(feature = "async")]
impl_adu_adapter! {
    [async] [.await],
    /// Asynchronous ASCII ADU adapter.
    AsyncAsciiAduAdapter,
    crate::ascii::AsciiAdu,
    no_transaction
}

/// An asynchronous ASCII Modbus client.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncAsciiClient<T: AsyncTransport>(AsyncClientCore<AsyncAsciiAduAdapter<T>>);

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncAsciiClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: AsciiClientConfig) -> Self {
        Self(AsyncClientCore::with_config(
            AsyncAsciiAduAdapter::with_config(transport, config),
            config,
        ))
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> Deref for AsyncAsciiClient<T> {
    type Target = AsyncClientCore<AsyncAsciiAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> DerefMut for AsyncAsciiClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::ascii::AsciiAdu;
    use crate::client::AsyncAduAdapter;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::{AsyncTransport, TransportError};
    use crate::{ExceptionResponse, ReadCoilsRequest, ReadCoilsResponse};
    use alloc::collections::VecDeque;
    use core::time::Duration;

    struct AsyncLoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncLoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncLoopbackTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = AsciiAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[tokio::test]
    async fn read_coils_over_ascii() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let mut client = AsyncAsciiClient::new(AsyncLoopbackTransport::new(server));
        let coils = client.read_coils(0x01, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn write_and_read_holding_register_over_ascii() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = AsyncAsciiClient::new(AsyncLoopbackTransport::new(server));
        client.write_register(0x01, 1, 0x1234).await.unwrap();
        let bytes = client.read_holding_registers(0x01, 1, 1).await.unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[tokio::test]
    async fn dispatch_returns_exception() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let exception_pdu = {
            let exc =
                ExceptionResponse::new(0x01, crate::exception::ExceptionCode::IllegalDataAddress);
            let mut buf = [0u8; 2];
            let n = exc.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = AsciiAdu::new(0x01, exception_pdu);
            let mut buf = [0u8; 32];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncAsciiClient::new(MockAsyncTransport::new(vec![response_adu]));
        let err = client.dispatch(0x01, &request_pdu).await.unwrap_err();
        assert!(matches!(err, AsciiClientError::Exception(_)));
    }

    #[tokio::test]
    async fn dispatch_rejects_wrong_slave() {
        let response_adu = {
            let pdu = {
                let resp = ReadCoilsResponse {
                    coil_status: vec![0x01],
                };
                let mut buf = [0u8; 3];
                let n = resp.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };
            let adu = AsciiAdu::new(0x02, pdu);
            let mut buf = [0u8; 32];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncAsciiClient::new(MockAsyncTransport::new(vec![response_adu]));
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01])
            .await
            .unwrap_err();
        assert!(matches!(err, AsciiClientError::InvalidResponse));
    }

    #[tokio::test]
    async fn broadcast_skips_recv_and_returns_empty_pdu() {
        use super::AsyncAsciiAduAdapter;

        let request_pdu = {
            let req = ReadCoilsRequest::new(0, 8).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut adapter = AsyncAsciiAduAdapter::with_config(
            MockAsyncTransport::new(vec![]),
            AsciiClientConfig::default(),
        );
        let response = adapter.send_receive(0, &request_pdu).await.unwrap();

        assert!(response.is_empty());
        assert_eq!(adapter.transport.sent.len(), 1);

        let sent = AsciiAdu::decode(&adapter.transport.sent[0]).unwrap();
        assert_eq!(sent.address, 0);
        assert_eq!(sent.pdu, request_pdu);
    }

    struct MockAsyncTransport {
        sent: Vec<Vec<u8>>,
        responses: VecDeque<Vec<u8>>,
    }

    impl MockAsyncTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                sent: Vec::new(),
                responses: responses.into(),
            }
        }
    }

    impl AsyncTransport for MockAsyncTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let resp = self
                .responses
                .pop_front()
                .ok_or(TransportError::Disconnected)?;
            let n = resp.len().min(buf.len());
            buf[..n].copy_from_slice(&resp[..n]);
            Ok(n)
        }
    }
}
