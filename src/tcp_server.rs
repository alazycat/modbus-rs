//! Synchronous TCP server listener.
//!
//! This module is available when both the `tcp` and `sync` features are
//! enabled. It wraps a [`Server`](crate::server::Server) and the TCP MBAP
//! framer so that request/response ADUs can be served over any [`Read`] +
//! [`Write`] stream.

#![cfg(all(feature = "tcp", feature = "sync"))]

use std::io::{self, Read, Write};

use crate::error::{DecodeError, EncodeError};
use crate::server::{DataStore, Server};
use crate::tcp::TcpAdu;

/// Errors that can occur while running the TCP server.
#[derive(Debug)]
pub enum TcpServerError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// Failed to encode a response ADU.
    Encode(EncodeError),
    /// Failed to decode a request ADU.
    Decode(DecodeError),
    /// The peer disconnected or the stream reached end-of-file.
    Disconnected,
}

impl core::fmt::Display for TcpServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "TCP server I/O error: {e}"),
            Self::Encode(e) => write!(f, "TCP server encode error: {e}"),
            Self::Decode(e) => write!(f, "TCP server decode error: {e}"),
            Self::Disconnected => write!(f, "TCP server disconnected"),
        }
    }
}

impl std::error::Error for TcpServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TcpServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<EncodeError> for TcpServerError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

impl From<DecodeError> for TcpServerError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

/// A synchronous TCP Modbus server.
///
/// The server listens on a byte stream, decodes TCP ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into a TCP
/// ADU. Requests with a non-matching unit ID are ignored.
#[derive(Debug)]
pub struct TcpServer<D: DataStore> {
    server: Server<D>,
}

impl<D: DataStore> TcpServer<D> {
    /// Create a new TCP server backed by `store`.
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

    /// Serve a single request/response exchange on `stream`.
    ///
    /// Returns `Some(n)` if a response ADU of `n` bytes was written back to
    /// the stream, or `None` if the request was filtered out because its unit
    /// ID did not match `unit_id`.
    pub fn serve_one<T: Read + Write>(
        &mut self,
        stream: &mut T,
        unit_id: u8,
    ) -> Result<Option<usize>, TcpServerError> {
        let request = self.read_adu(stream)?;

        if request.unit_id != unit_id {
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch(&request.pdu, &mut pdu_response)?;

        let response = TcpAdu::new(
            request.transaction_id,
            request.unit_id,
            pdu_response[..n].to_vec(),
        );
        let mut tx = [0u8; 512];
        let m = response.encode(&mut tx)?;
        stream.write_all(&tx[..m])?;
        stream.flush()?;
        Ok(Some(m))
    }

    /// Continuously serve requests on `stream`.
    ///
    /// The function returns when the stream is disconnected or reaches EOF.
    pub fn serve<T: Read + Write>(
        &mut self,
        stream: &mut T,
        unit_id: u8,
    ) -> Result<(), TcpServerError> {
        loop {
            match self.serve_one(stream, unit_id) {
                Ok(_) => {}
                Err(TcpServerError::Disconnected) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    fn read_adu<T: Read>(&mut self, stream: &mut T) -> Result<TcpAdu, TcpServerError> {
        let mut header = [0u8; TcpAdu::HEADER_SIZE];
        read_all(stream, &mut header)?;

        let length = u16::from_be_bytes([header[4], header[5]]) as usize;
        if length == 0 {
            return Err(TcpServerError::Decode(DecodeError::InvalidValue));
        }
        let pdu_len = length - 1;

        let mut frame = vec![0u8; TcpAdu::HEADER_SIZE + pdu_len];
        frame[..TcpAdu::HEADER_SIZE].copy_from_slice(&header);
        read_all(stream, &mut frame[TcpAdu::HEADER_SIZE..])?;

        TcpAdu::decode(&frame).map_err(TcpServerError::Decode)
    }
}

fn read_all<T: Read>(stream: &mut T, buf: &mut [u8]) -> Result<(), TcpServerError> {
    let mut pos = 0;
    while pos < buf.len() {
        match stream.read(&mut buf[pos..]) {
            Ok(0) => return Err(TcpServerError::Disconnected),
            Ok(n) => pos += n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                return Err(TcpServerError::Disconnected);
            }
            Err(e) => return Err(TcpServerError::Io(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore};
    use crate::tcp::TcpAdu;

    /// A simple in-memory stream for tests. `read_buf` feeds the server and
    /// `write_buf` collects anything the server writes back.
    struct Duplex {
        read_buf: Vec<u8>,
        read_pos: usize,
        write_buf: Vec<u8>,
    }

    impl Duplex {
        fn new(read_buf: Vec<u8>) -> Self {
            Self {
                read_buf,
                read_pos: 0,
                write_buf: Vec::new(),
            }
        }
    }

    impl Read for Duplex {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let remaining = self.read_buf.len() - self.read_pos;
            if remaining == 0 {
                return Ok(0);
            }
            let n = buf.len().min(remaining);
            buf[..n].copy_from_slice(&self.read_buf[self.read_pos..self.read_pos + n]);
            self.read_pos += n;
            Ok(n)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.write_buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

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
        let m = TcpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_one_responds_to_matching_unit_id() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = TcpServer::new(store);
        let request = make_read_coils_adu(0x0A, 0x0001, 0, 8);
        let mut stream = Duplex::new(request);

        let n = server.serve_one(&mut stream, 0x0A).unwrap().unwrap();
        assert!(n > 0);
        let response = TcpAdu::decode(&stream.write_buf).unwrap();
        assert_eq!(response.unit_id, 0x0A);
        assert_eq!(response.transaction_id, 0x0001);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn serve_one_ignores_non_matching_unit_id() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = TcpServer::new(store);
        let request = make_read_coils_adu(0x02, 0x0001, 0, 8);
        let mut stream = Duplex::new(request);

        let result = server.serve_one(&mut stream, 0x0A).unwrap();
        assert!(result.is_none());
        assert!(stream.write_buf.is_empty());
    }

    #[test]
    fn serve_one_returns_decode_error_for_zero_length() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = TcpServer::new(store);

        // MBAP header with a zero length field is malformed.
        let mut frame = [0u8; TcpAdu::HEADER_SIZE];
        frame[0..2].copy_from_slice(&0x0001u16.to_be_bytes());
        frame[2..4].copy_from_slice(&crate::tcp::MODBUS_PROTOCOL_ID.to_be_bytes());
        frame[6] = 0x0A;
        let mut stream = Duplex::new(frame.to_vec());

        let err = server.serve_one(&mut stream, 0x0A).unwrap_err();
        assert!(matches!(
            err,
            TcpServerError::Decode(DecodeError::InvalidValue)
        ));
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
        let m = TcpAdu::new(transaction_id, unit_id, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_loops_until_eof() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = TcpServer::new(store);
        let mut input = make_read_coils_adu(0x0A, 0x1234, 0, 8);
        input.extend_from_slice(&make_read_coils_adu(0x0A, 0x1235, 0, 8));

        let mut stream = Duplex::new(input);
        server.serve(&mut stream, 0x0A).unwrap();

        let expected = make_read_coils_response_adu(0x0A, 0x1234, vec![0b00001101]);
        let expected2 = make_read_coils_response_adu(0x0A, 0x1235, vec![0b00001101]);
        assert_eq!(stream.write_buf.len(), expected.len() + expected2.len());
        assert_eq!(&stream.write_buf[..expected.len()], expected.as_slice());
        assert_eq!(&stream.write_buf[expected.len()..], expected2.as_slice());
    }
}
