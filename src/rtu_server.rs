//! Synchronous RTU server listener.
//!
//! This module is available when both the `rtu` and `sync` features are
//! enabled. It wraps a [`Server`](crate::server::Server) and the RTU framer so
//! that request/response ADUs can be served over any [`Read`] + [`Write`]
//! stream.

#![cfg(all(feature = "rtu", feature = "sync"))]

use std::io::{self, Read, Write};

use crate::error::EncodeError;
use crate::rtu::RtuAdu;
use crate::server::{DataStore, Server};

/// Errors that can occur while running the RTU server.
#[derive(Debug)]
pub enum RtuServerError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// Failed to encode a response ADU.
    Encode(EncodeError),
    /// The peer disconnected or the stream reached end-of-file.
    Disconnected,
}

impl core::fmt::Display for RtuServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "RTU server I/O error: {e}"),
            Self::Encode(e) => write!(f, "RTU server encode error: {e:?}"),
            Self::Disconnected => write!(f, "RTU server disconnected"),
        }
    }
}

impl std::error::Error for RtuServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for RtuServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<EncodeError> for RtuServerError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

/// A synchronous RTU server.
///
/// The server listens on a byte stream, decodes RTU ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into an
/// RTU ADU. Requests addressed to a different slave (and not broadcast) are
/// ignored.
#[derive(Debug)]
pub struct RtuServer<D: DataStore> {
    server: Server<D>,
}

impl<D: DataStore> RtuServer<D> {
    /// Create a new RTU server backed by `store`.
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
    /// the stream, or `None` if the request was filtered out or was a
    /// broadcast (which has no response).
    pub fn serve_one<T: Read + Write>(
        &mut self,
        stream: &mut T,
        server_address: u8,
    ) -> Result<Option<usize>, RtuServerError> {
        let request = self.read_adu(stream)?;

        if request.address != server_address && !request.is_broadcast() {
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch(&request.pdu, &mut pdu_response)?;

        if request.is_broadcast() {
            return Ok(None);
        }

        let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
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
        server_address: u8,
    ) -> Result<(), RtuServerError> {
        loop {
            match self.serve_one(stream, server_address) {
                Ok(_) => {}
                Err(RtuServerError::Disconnected) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    fn read_adu<T: Read + Write>(&mut self,
        stream: &mut T,
    ) -> Result<RtuAdu, RtuServerError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            match stream.read(&mut byte) {
                Ok(0) => {
                    if frame.is_empty() {
                        return Err(RtuServerError::Disconnected);
                    }
                    break;
                }
                Ok(1) => {
                    frame.push(byte[0]);
                    if frame.len() > RtuAdu::MAX_FRAME_SIZE {
                        return Err(RtuServerError::Disconnected);
                    }
                    if frame.len() >= RtuAdu::MIN_FRAME_SIZE
                        && RtuAdu::decode(&frame).is_ok()
                    {
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    if frame.is_empty() {
                        return Err(RtuServerError::Disconnected);
                    }
                    break;
                }
                Err(e) => return Err(RtuServerError::Io(e)),
            }
        }

        RtuAdu::decode(&frame).map_err(|_| RtuServerError::Disconnected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtu::RtuAdu;
    use crate::server::{DataStore, MemoryStore};

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

    fn make_read_coils_adu(slave: u8, address: u16, quantity: u16) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsRequest;
        let req = ReadCoilsRequest::new(address, quantity).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        RtuAdu::new(slave, pdu[..n].to_vec()).encode(&mut [0u8; 32]).unwrap();
        let mut adu = [0u8; 32];
        let m = RtuAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_one_responds_to_matching_address() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = RtuServer::new(store);
        let request = make_read_coils_adu(0x03, 0, 8);
        let mut stream = Duplex::new(request);

        let n = server.serve_one(&mut stream, 0x03).unwrap().unwrap();
        assert!(n > 0);
        let response = RtuAdu::decode(&stream.write_buf).unwrap();
        assert_eq!(response.address, 0x03);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn serve_one_ignores_non_matching_address() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = RtuServer::new(store);
        let request = make_read_coils_adu(0x02, 0, 8);
        let mut stream = Duplex::new(request);

        let result = server.serve_one(&mut stream, 0x03).unwrap();
        assert!(result.is_none());
        assert!(stream.write_buf.is_empty());
    }

    #[test]
    fn serve_one_rejects_oversized_frame() {
        struct GarbageStream;
        impl Read for GarbageStream {
            fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
                if buf.is_empty() {
                    return Ok(0);
                }
                buf[0] = 0x00;
                Ok(1)
            }
        }
        impl Write for GarbageStream {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = RtuServer::new(store);
        let err = server.serve_one(&mut GarbageStream, 0x01).unwrap_err();
        assert!(matches!(err, RtuServerError::Disconnected));
    }

    #[test]
    fn serve_one_processes_broadcast_without_response() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = RtuServer::new(store);

        use crate::function_codes::write_single_coil::WriteSingleCoilRequest;
        let req = WriteSingleCoilRequest::new(0x05, WriteSingleCoilRequest::ON).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = RtuAdu::new(RtuAdu::BROADCAST_ADDRESS, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let mut stream = Duplex::new(adu[..m].to_vec());
        let result = server.serve_one(&mut stream, 0x03).unwrap();
        assert!(result.is_none());
        assert!(stream.write_buf.is_empty());
        assert!(server.server().store().read_coils(0x05, 1).unwrap()[0] & 0x01 != 0);
    }

    fn make_read_coils_response_adu(slave: u8, coil_status: Vec<u8>) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsResponse;
        let resp = ReadCoilsResponse { coil_status };
        let mut pdu = [0u8; 256];
        let n = resp.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 512];
        let m = RtuAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_loops_until_eof() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = RtuServer::new(store);
        let mut input = make_read_coils_adu(0x07, 0, 8);
        input.extend_from_slice(&make_read_coils_adu(0x07, 0, 8));

        let mut stream = Duplex::new(input);
        server.serve(&mut stream, 0x07).unwrap();

        let expected = make_read_coils_response_adu(0x07, vec![0b00001101]);
        assert_eq!(stream.write_buf.len(), expected.len() * 2);
        assert_eq!(&stream.write_buf[..expected.len()], expected.as_slice());
        assert_eq!(
            &stream.write_buf[expected.len()..],
            expected.as_slice()
        );
    }
}
