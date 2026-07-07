//! Synchronous and asynchronous UDP Modbus server listener.
//!
//! This module is available when the `udp` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps a
//! [`Server`](crate::server::Server) and the UDP MBAP framer so that
//! request/response ADUs can be served as datagrams over a UDP socket.

#![cfg(all(feature = "udp", any(feature = "sync", feature = "async")))]

use crate::error::{DecodeError, EncodeError};
use crate::server::{DataStore, Server};
use crate::udp::UdpAdu;

#[cfg(feature = "sync")]
use std::net::UdpSocket;

#[cfg(feature = "async")]
use tokio::net::UdpSocket as TokioUdpSocket;

/// Errors that can occur while running the UDP server.
#[derive(Debug)]
pub enum UdpServerError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// Failed to encode a response ADU.
    Encode(EncodeError),
    /// Failed to decode a request ADU.
    Decode(DecodeError),
}

impl core::fmt::Display for UdpServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "UDP server I/O error: {e}"),
            Self::Encode(e) => write!(f, "UDP server encode error: {e}"),
            Self::Decode(e) => write!(f, "UDP server decode error: {e}"),
        }
    }
}

impl std::error::Error for UdpServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for UdpServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<EncodeError> for UdpServerError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

impl From<DecodeError> for UdpServerError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

/// A synchronous UDP Modbus server.
///
/// The server listens on a UDP socket, decodes UDP ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into a UDP
/// ADU sent to the originating peer. Requests with a non-matching unit ID are
/// silently ignored.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct UdpServer<D: DataStore> {
    server: Server<D>,
}

#[cfg(feature = "sync")]
impl<D: DataStore> UdpServer<D> {
    /// Create a new UDP server backed by `store`.
    pub fn new(store: D) -> Self {
        Self {
            server: Server::new(store),
        }
    }

    /// Return an immutable reference to the inner [`Server`].
    pub fn server(&self) -> &Server<D> {
        &self.server
    }

    /// Return a mutable reference to the inner [`Server`].
    pub fn server_mut(&mut self) -> &mut Server<D> {
        &mut self.server
    }

    /// Serve a single request/response exchange on `socket`.
    ///
    /// Returns `Some(n)` if a response ADU of `n` bytes was sent back to the
    /// requester, or `None` if the request was filtered out because its unit
    /// ID did not match `unit_id`.
    pub fn serve_one(
        &mut self,
        socket: &UdpSocket,
        unit_id: u8,
    ) -> Result<Option<usize>, UdpServerError> {
        let mut rx = [0u8; 512];
        let (len, peer) = socket.recv_from(&mut rx)?;

        let request = UdpAdu::decode(&rx[..len])?;
        if request.unit_id != unit_id {
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch(&request.pdu, &mut pdu_response)?;

        let response = UdpAdu::new(
            request.transaction_id,
            request.unit_id,
            pdu_response[..n].to_vec(),
        );
        let mut tx = [0u8; 512];
        let m = response.encode(&mut tx)?;
        socket.send_to(&tx[..m], peer)?;

        Ok(Some(m))
    }

    /// Continuously serve datagrams on `socket`.
    ///
    /// The function loops indefinitely, handling one datagram per iteration.
    /// It only returns on an I/O or encode/decode error; malformed datagrams
    /// are reported as errors rather than silently dropped.
    pub fn serve(&mut self, socket: &UdpSocket, unit_id: u8) -> Result<(), UdpServerError> {
        loop {
            self.serve_one(socket, unit_id)?;
        }
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore};
    use crate::udp::UdpAdu;
    use std::thread;
    use std::time::Duration;

    fn make_read_coils_adu(
        unit_id: u8,
        transaction_id: u16,
        address: u16,
        quantity: u16,
    ) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsRequest;
        let req = ReadCoilsRequest::new(address, quantity).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = UdpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    fn make_read_coils_response_adu(
        unit_id: u8,
        transaction_id: u16,
        coil_status: Vec<u8>,
    ) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsResponse;
        let resp = ReadCoilsResponse { coil_status };
        let mut pdu = [0u8; 256];
        let n = resp.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 512];
        let m = UdpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_one_responds_to_matching_unit_id() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = UdpServer::new(store);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        let request = make_read_coils_adu(0x0A, 0x0001, 0, 8);
        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.send_to(&request, addr).unwrap();

        let n = server.serve_one(&socket, 0x0A).unwrap().unwrap();
        assert!(n > 0);

        let mut rx = [0u8; 512];
        let (m, _) = client.recv_from(&mut rx).unwrap();
        let response = UdpAdu::decode(&rx[..m]).unwrap();
        assert_eq!(response.unit_id, 0x0A);
        assert_eq!(response.transaction_id, 0x0001);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn serve_one_ignores_non_matching_unit_id() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = UdpServer::new(store);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        let request = make_read_coils_adu(0x02, 0x0001, 0, 8);
        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client
            .set_read_timeout(Some(Duration::from_millis(50)))
            .unwrap();
        client.send_to(&request, addr).unwrap();

        let result = server.serve_one(&socket, 0x0A).unwrap();
        assert!(result.is_none());

        let mut rx = [0u8; 512];
        assert!(client.recv_from(&mut rx).is_err());
    }

    #[test]
    fn serve_one_returns_decode_error_for_zero_length() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = UdpServer::new(store);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        // MBAP header with a zero length field is malformed.
        let mut frame = [0u8; UdpAdu::HEADER_SIZE];
        frame[0..2].copy_from_slice(&0x0001u16.to_be_bytes());
        frame[2..4].copy_from_slice(&crate::udp::MODBUS_PROTOCOL_ID.to_be_bytes());
        frame[6] = 0x0A;

        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.send_to(&frame, addr).unwrap();

        let err = server.serve_one(&socket, 0x0A).unwrap_err();
        assert!(matches!(
            err,
            UdpServerError::Decode(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn serve_loops_until_error() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = UdpServer::new(store);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        let expected = make_read_coils_response_adu(0x0A, 0x1234, vec![0b00001101]);

        thread::spawn(move || {
            let client = UdpSocket::bind("127.0.0.1:0").unwrap();
            let request = make_read_coils_adu(0x0A, 0x1234, 0, 8);
            client.send_to(&request, addr).unwrap();

            let mut rx = [0u8; 512];
            let (n, _) = client.recv_from(&mut rx).unwrap();
            assert_eq!(&rx[..n], expected.as_slice());

            // Send a malformed datagram to make serve() return an error.
            client.send_to(&[0u8; 2], addr).unwrap();
        });

        let result = server.serve(&socket, 0x0A);
        assert!(matches!(
            result.unwrap_err(),
            UdpServerError::Decode(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn serve_one_reads_holding_registers() {
        let mut store = MemoryStore::new(0, 0, 4, 0);
        store.write_registers(0, &[0x1234]).unwrap();

        let mut server = UdpServer::new(store);
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = socket.local_addr().unwrap();

        use crate::function_codes::read_holding_registers::ReadHoldingRegistersRequest;
        let req = ReadHoldingRegistersRequest::new(0, 1).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = UdpAdu::new(0xABCD, 0x05, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let client = UdpSocket::bind("127.0.0.1:0").unwrap();
        client.send_to(&adu[..m], addr).unwrap();

        let n = server.serve_one(&socket, 0x05).unwrap().unwrap();
        assert!(n > 0);

        let mut rx = [0u8; 512];
        let (m, _) = client.recv_from(&mut rx).unwrap();
        let response = UdpAdu::decode(&rx[..m]).unwrap();
        assert_eq!(response.transaction_id, 0xABCD);
        assert_eq!(response.pdu, vec![0x03, 0x02, 0x12, 0x34]);
    }
}

/// An asynchronous UDP Modbus server.
///
/// The server listens on a tokio UDP socket, decodes UDP ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into a UDP
/// ADU sent to the originating peer. Requests with a non-matching unit ID are
/// silently ignored.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncUdpServer<D: DataStore> {
    server: Server<D>,
}

#[cfg(feature = "async")]
impl<D: DataStore> AsyncUdpServer<D> {
    /// Create a new async UDP server backed by `store`.
    pub fn new(store: D) -> Self {
        Self {
            server: Server::new(store),
        }
    }

    /// Return an immutable reference to the inner [`Server`].
    pub fn server(&self) -> &Server<D> {
        &self.server
    }

    /// Return a mutable reference to the inner [`Server`].
    pub fn server_mut(&mut self) -> &mut Server<D> {
        &mut self.server
    }

    /// Serve a single request/response exchange on `socket`.
    ///
    /// Returns `Some(n)` if a response ADU of `n` bytes was sent back to the
    /// requester, or `None` if the request was filtered out because its unit
    /// ID did not match `unit_id`.
    pub async fn serve_one(
        &mut self,
        socket: &TokioUdpSocket,
        unit_id: u8,
    ) -> Result<Option<usize>, UdpServerError> {
        let mut rx = [0u8; 512];
        let (len, peer) = socket.recv_from(&mut rx).await?;

        let request = UdpAdu::decode(&rx[..len])?;
        if request.unit_id != unit_id {
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch(&request.pdu, &mut pdu_response)?;

        let response = UdpAdu::new(
            request.transaction_id,
            request.unit_id,
            pdu_response[..n].to_vec(),
        );
        let mut tx = [0u8; 512];
        let m = response.encode(&mut tx)?;
        socket.send_to(&tx[..m], peer).await?;

        Ok(Some(m))
    }

    /// Continuously serve datagrams on `socket`.
    ///
    /// The function loops indefinitely, handling one datagram per iteration.
    /// It only returns on an I/O or encode/decode error; malformed datagrams
    /// are reported as errors rather than silently dropped.
    pub async fn serve(
        &mut self,
        socket: &TokioUdpSocket,
        unit_id: u8,
    ) -> Result<(), UdpServerError> {
        loop {
            self.serve_one(socket, unit_id).await?;
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore};
    use crate::udp::UdpAdu;

    fn make_read_coils_adu(
        unit_id: u8,
        transaction_id: u16,
        address: u16,
        quantity: u16,
    ) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsRequest;
        let req = ReadCoilsRequest::new(address, quantity).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = UdpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    fn make_read_coils_response_adu(
        unit_id: u8,
        transaction_id: u16,
        coil_status: Vec<u8>,
    ) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsResponse;
        let resp = ReadCoilsResponse { coil_status };
        let mut pdu = [0u8; 256];
        let n = resp.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 512];
        let m = UdpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[tokio::test]
    async fn serve_one_responds_to_matching_unit_id() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncUdpServer::new(store);
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        let request = make_read_coils_adu(0x0A, 0x0001, 0, 8);
        let client = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(&request, addr).await.unwrap();

        let n = server.serve_one(&socket, 0x0A).await.unwrap().unwrap();
        assert!(n > 0);

        let mut rx = [0u8; 512];
        let (m, _) = client.recv_from(&mut rx).await.unwrap();
        let response = UdpAdu::decode(&rx[..m]).unwrap();
        assert_eq!(response.unit_id, 0x0A);
        assert_eq!(response.transaction_id, 0x0001);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[tokio::test]
    async fn serve_one_ignores_non_matching_unit_id() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsyncUdpServer::new(store);
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        let request = make_read_coils_adu(0x02, 0x0001, 0, 8);
        let client = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(&request, addr).await.unwrap();

        let result = server.serve_one(&socket, 0x0A).await.unwrap();
        assert!(result.is_none());

        // The request should have been consumed; the client should time out.
        let mut rx = [0u8; 512];
        let timeout_result = tokio::time::timeout(
            std::time::Duration::from_millis(50),
            client.recv_from(&mut rx),
        )
        .await;
        assert!(timeout_result.is_err());
    }

    #[tokio::test]
    async fn serve_one_returns_decode_error_for_zero_length() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = AsyncUdpServer::new(store);
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        // MBAP header with a zero length field is malformed.
        let mut frame = [0u8; UdpAdu::HEADER_SIZE];
        frame[0..2].copy_from_slice(&0x0001u16.to_be_bytes());
        frame[2..4].copy_from_slice(&crate::udp::MODBUS_PROTOCOL_ID.to_be_bytes());
        frame[6] = 0x0A;

        let client = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(&frame, addr).await.unwrap();

        let err = server.serve_one(&socket, 0x0A).await.unwrap_err();
        assert!(matches!(
            err,
            UdpServerError::Decode(DecodeError::InvalidValue)
        ));
    }

    #[tokio::test]
    async fn serve_loops_until_error() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncUdpServer::new(store);
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        let expected = make_read_coils_response_adu(0x0A, 0x1234, vec![0b00001101]);

        tokio::spawn(async move {
            let client = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
            let request = make_read_coils_adu(0x0A, 0x1234, 0, 8);
            client.send_to(&request, addr).await.unwrap();

            let mut rx = [0u8; 512];
            let (n, _) = client.recv_from(&mut rx).await.unwrap();
            assert_eq!(&rx[..n], expected.as_slice());

            // Send a malformed datagram to make serve() return an error.
            client.send_to(&[0u8; 2], addr).await.unwrap();
        });

        let result = server.serve(&socket, 0x0A).await;
        assert!(matches!(
            result.unwrap_err(),
            UdpServerError::Decode(DecodeError::InvalidLength)
        ));
    }

    #[tokio::test]
    async fn serve_one_reads_holding_registers() {
        let mut store = MemoryStore::new(0, 0, 4, 0);
        store.write_registers(0, &[0x1234]).unwrap();

        let mut server = AsyncUdpServer::new(store);
        let socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let addr = socket.local_addr().unwrap();

        use crate::function_codes::read_holding_registers::ReadHoldingRegistersRequest;
        let req = ReadHoldingRegistersRequest::new(0, 1).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = UdpAdu::new(0xABCD, 0x05, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let client = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        client.send_to(&adu[..m], addr).await.unwrap();

        let n = server.serve_one(&socket, 0x05).await.unwrap().unwrap();
        assert!(n > 0);

        let mut rx = [0u8; 512];
        let (m, _) = client.recv_from(&mut rx).await.unwrap();
        let response = UdpAdu::decode(&rx[..m]).unwrap();
        assert_eq!(response.transaction_id, 0xABCD);
        assert_eq!(response.pdu, vec![0x03, 0x02, 0x12, 0x34]);
    }
}
