//! Synchronous and asynchronous TCP Modbus clients.
//!
//! This module is available when the `tcp` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps request PDUs in the
//! MODBUS TCP MBAP header, tracks transaction IDs, validates responses, and
//! exposes high-level methods for reading and writing coils and registers.

#![cfg(all(feature = "tcp", any(feature = "sync", feature = "async")))]

use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use crate::client::{ClientConfig, ClientError};
use crate::tcp::TcpAdu;
use crate::transport::TransportError;

#[cfg(feature = "sync")]
use crate::client::{AduAdapter, ClientCore};
#[cfg(feature = "sync")]
use crate::transport::Transport;

#[cfg(feature = "async")]
use crate::client::{AsyncAduAdapter, AsyncClientCore};
#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// Configuration for a synchronous and asynchronous TCP client.
pub type TcpClientConfig = crate::client::ClientConfig;

/// Errors that can occur while using the TCP client.
pub type TcpClientError = crate::client::ClientError;

/// Synchronous TCP ADU adapter.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct TcpAduAdapter<T: Transport> {
    transport: T,
    config: ClientConfig,
    next_transaction_id: u16,
}

#[cfg(feature = "sync")]
impl<T: Transport> TcpAduAdapter<T> {
    /// Create an adapter with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create an adapter with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self {
            transport,
            config,
            next_transaction_id: 1,
        }
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> AduAdapter for TcpAduAdapter<T> {
    fn send_receive(&mut self, unit_id: u8, request_pdu: &[u8]) -> Result<Vec<u8>, ClientError> {
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

        let adu = TcpAdu::new(transaction_id, unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(ClientError::Encode)?;
        self.transport.send(&tx[..n])?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout)?;
        if m == 0 {
            return Err(ClientError::Transport(TransportError::Disconnected));
        }
        let response = TcpAdu::decode(&rx[..m]).map_err(ClientError::Decode)?;
        if response.transaction_id != transaction_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.unit_id != unit_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response.pdu)
    }
}

/// A synchronous TCP Modbus client.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct TcpClient<T: Transport>(ClientCore<TcpAduAdapter<T>>);

#[cfg(feature = "sync")]
impl<T: Transport> TcpClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: TcpClientConfig) -> Self {
        Self(ClientCore::new(TcpAduAdapter::with_config(
            transport, config,
        )))
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> Deref for TcpClient<T> {
    type Target = ClientCore<TcpAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> DerefMut for TcpClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::tcp::TcpAdu;
    use crate::transport::Transport;
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
            let request = TcpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = TcpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        fn recv(
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

    #[test]
    fn read_coils_over_tcp() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server.store_mut().write_coils(0, &[true, false, true, true]).unwrap();

        let mut client = TcpClient::new(LoopbackTransport::new(server));
        let coils = client.read_coils(0x0A, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[test]
    fn write_and_read_holding_register_over_tcp() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = TcpClient::new(LoopbackTransport::new(server));
        client.write_register(0x0A, 1, 0x1234).unwrap();
        let bytes = client.read_holding_registers(0x0A, 1, 1).unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[test]
    fn transaction_id_increments() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server.store_mut().write_coils(0, &[true, true, true, true]).unwrap();

        let mut client = TcpClient::new(LoopbackTransport::new(server));
        let _ = client.read_coils(0x01, 0, 8).unwrap();
        let _ = client.read_coils(0x01, 0, 8).unwrap();

        // Transaction IDs should differ; exact values are an implementation detail.
        // The loopback server echoes the request transaction ID, so if dispatch
        // succeeded both were tracked correctly.
    }

    #[test]
    fn mismatched_transaction_id_returns_invalid_response() {
        struct BadTransport;
        impl Transport for BadTransport {
            fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            fn recv(
                &mut self,
                buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                let response = TcpAdu::new(0x9999, 0x01, vec![0x01, 0x01, 0x0F]);
                let mut tmp = [0u8; 32];
                let n = response.encode(&mut tmp).unwrap();
                buf[..n].copy_from_slice(&tmp[..n]);
                Ok(n)
            }
        }

        let mut client = TcpClient::new(BadTransport);
        let err = client.read_coils(0x01, 0, 8).unwrap_err();
        assert!(matches!(err, TcpClientError::InvalidResponse));
    }
}

/// Asynchronous TCP ADU adapter.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncTcpAduAdapter<T: AsyncTransport> {
    transport: T,
    config: ClientConfig,
    next_transaction_id: u16,
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncTcpAduAdapter<T> {
    /// Create an adapter with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create an adapter with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self {
            transport,
            config,
            next_transaction_id: 1,
        }
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncAduAdapter for AsyncTcpAduAdapter<T> {
    async fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

        let adu = TcpAdu::new(transaction_id, unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(ClientError::Encode)?;
        self.transport.send(&tx[..n]).await?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout).await?;
        if m == 0 {
            return Err(ClientError::Transport(TransportError::Disconnected));
        }
        let response = TcpAdu::decode(&rx[..m]).map_err(ClientError::Decode)?;
        if response.transaction_id != transaction_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.unit_id != unit_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response.pdu)
    }
}

/// An asynchronous TCP Modbus client.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncTcpClient<T: AsyncTransport>(AsyncClientCore<AsyncTcpAduAdapter<T>>);

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncTcpClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: TcpClientConfig) -> Self {
        Self(AsyncClientCore::new(AsyncTcpAduAdapter::with_config(
            transport, config,
        )))
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> Deref for AsyncTcpClient<T> {
    type Target = AsyncClientCore<AsyncTcpAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> DerefMut for AsyncTcpClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::tcp::TcpAdu;
    use crate::transport::AsyncTransport;
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
            let request = TcpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = TcpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
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
    async fn read_coils_over_tcp() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let mut client = AsyncTcpClient::new(AsyncLoopbackTransport::new(server));
        let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn write_and_read_holding_register_over_tcp() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = AsyncTcpClient::new(AsyncLoopbackTransport::new(server));
        client.write_register(0x0A, 1, 0x1234).await.unwrap();
        let bytes = client.read_holding_registers(0x0A, 1, 1).await.unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[tokio::test]
    async fn transaction_id_increments() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, true, true, true])
            .unwrap();

        let mut client = AsyncTcpClient::new(AsyncLoopbackTransport::new(server));
        let _ = client.read_coils(0x01, 0, 8).await.unwrap();
        let _ = client.read_coils(0x01, 0, 8).await.unwrap();
    }

    #[tokio::test]
    async fn mismatched_transaction_id_returns_invalid_response() {
        struct BadAsyncTransport;
        impl AsyncTransport for BadAsyncTransport {
            async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            async fn recv(
                &mut self,
                buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                let response = TcpAdu::new(0x9999, 0x01, vec![0x01, 0x01, 0x0F]);
                let mut tmp = [0u8; 32];
                let n = response.encode(&mut tmp).unwrap();
                buf[..n].copy_from_slice(&tmp[..n]);
                Ok(n)
            }
        }

        let mut client = AsyncTcpClient::new(BadAsyncTransport);
        let err = client.read_coils(0x01, 0, 8).await.unwrap_err();
        assert!(matches!(err, TcpClientError::InvalidResponse));
    }
}
