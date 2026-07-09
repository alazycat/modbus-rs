//! Shared integration-test harness: mock (loopback) transports and helpers.
//!
//! This module is only compiled when at least one of the sync/async runtime
//! features is enabled.

#![cfg(any(feature = "sync", feature = "async"))]
#![allow(dead_code)]

use modbus::server::MemoryStore;

/// A store pre-populated with enough holding/input registers for the standard
/// register-access integration scenario.
pub fn register_store() -> MemoryStore {
    let mut store = MemoryStore::new(8, 0, 4, 4);
    store
        .write_input_registers(0, &[0xAABB, 0xCCDD, 0xEEFF])
        .unwrap();
    store
}

/// Assert the results returned by the register-access scenario.
pub fn assert_register_results(holding: &[u8], inputs: &[u8]) {
    assert_eq!(
        holding,
        &[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC],
        "holding registers should reflect writes"
    );
    assert_eq!(
        inputs,
        &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        "input registers should match pre-populated values"
    );
}

#[cfg(all(feature = "sync", feature = "rtu"))]
pub mod sync_rtu {
    use core::time::Duration;

    use modbus::rtu::RtuAdu;
    use modbus::server::Server;
    use modbus::transport::{Transport, TransportError};

    use super::MemoryStore;

    /// A synchronous in-memory RTU loopback transport.
    #[derive(Debug)]
    pub struct RtuLoopback {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl RtuLoopback {
        pub fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for RtuLoopback {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = RtuAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "sync", feature = "tcp"))]
pub mod sync_tcp {
    use core::time::Duration;

    use modbus::server::Server;
    use modbus::tcp::TcpAdu;
    use modbus::transport::{Transport, TransportError};

    use super::MemoryStore;

    /// A synchronous in-memory TCP loopback transport.
    #[derive(Debug)]
    pub struct TcpLoopback {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl TcpLoopback {
        pub fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for TcpLoopback {
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
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "sync", feature = "udp"))]
pub mod sync_udp {
    use core::time::Duration;

    use modbus::server::Server;
    use modbus::transport::{Transport, TransportError};
    use modbus::udp::UdpAdu;

    use super::MemoryStore;

    /// A synchronous in-memory UDP loopback transport.
    #[derive(Debug)]
    pub struct UdpLoopback {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl UdpLoopback {
        pub fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for UdpLoopback {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = UdpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = UdpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "sync", feature = "ascii"))]
pub mod sync_ascii {
    use core::time::Duration;

    use modbus::ascii::AsciiAdu;
    use modbus::server::Server;
    use modbus::transport::{Transport, TransportError};

    use super::MemoryStore;

    /// A synchronous in-memory ASCII loopback transport.
    #[derive(Debug)]
    pub struct AsciiLoopback {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsciiLoopback {
        pub fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for AsciiLoopback {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = AsciiAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "async", feature = "rtu"))]
pub mod async_rtu {
    use core::time::Duration;

    use modbus::rtu::RtuAdu;
    use modbus::transport::{AsyncTransport, TransportError};
    use modbus::AsyncServer;

    use super::MemoryStore;

    /// An asynchronous in-memory RTU loopback transport.
    #[derive(Debug)]
    pub struct AsyncRtuLoopback {
        server: AsyncServer<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncRtuLoopback {
        pub fn new(server: AsyncServer<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncRtuLoopback {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = RtuAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .server_mut()
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "async", feature = "tcp"))]
pub mod async_tcp {
    use core::time::Duration;

    use modbus::tcp::TcpAdu;
    use modbus::transport::{AsyncTransport, TransportError};
    use modbus::AsyncServer;

    use super::MemoryStore;

    /// An asynchronous in-memory TCP loopback transport.
    #[derive(Debug)]
    pub struct AsyncTcpLoopback {
        server: AsyncServer<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncTcpLoopback {
        pub fn new(server: AsyncServer<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncTcpLoopback {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = TcpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .server_mut()
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = TcpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "async", feature = "udp"))]
pub mod async_udp {
    use core::time::Duration;

    use modbus::transport::{AsyncTransport, TransportError};
    use modbus::udp::UdpAdu;
    use modbus::AsyncServer;

    use super::MemoryStore;

    /// An asynchronous in-memory UDP loopback transport.
    #[derive(Debug)]
    pub struct AsyncUdpLoopback {
        server: AsyncServer<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncUdpLoopback {
        pub fn new(server: AsyncServer<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncUdpLoopback {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = UdpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .server_mut()
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = UdpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}

#[cfg(all(feature = "async", feature = "ascii"))]
pub mod async_ascii {
    use core::time::Duration;

    use modbus::ascii::AsciiAdu;
    use modbus::transport::{AsyncTransport, TransportError};
    use modbus::AsyncServer;

    use super::MemoryStore;

    /// An asynchronous in-memory ASCII loopback transport.
    #[derive(Debug)]
    pub struct AsyncAsciiLoopback {
        server: AsyncServer<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncAsciiLoopback {
        pub fn new(server: AsyncServer<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncAsciiLoopback {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = AsciiAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .server_mut()
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
            let mut buf = [0u8; 512];
            let m = response
                .encode(&mut buf)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(buf[..m].to_vec());
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
}
