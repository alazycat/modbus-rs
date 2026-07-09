//! Synchronous and asynchronous ASCII Modbus server listener.
//!
//! This module is available when the `ascii` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps a
//! [`Server`](crate::server::Server) and the ASCII framer so that request/response
//! ADUs can be served over any byte stream.

#![cfg(all(feature = "ascii", any(feature = "sync", feature = "async")))]

use crate::ascii::AsciiAdu;
use crate::error::{DecodeError, EncodeError};
use crate::server::{DataStore, Server};

#[cfg(feature = "sync")]
use std::io::{self, Read, Write};

#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Errors that can occur while running the ASCII server.
#[derive(Debug)]
pub enum AsciiServerError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// Failed to encode a response ADU.
    Encode(EncodeError),
    /// Failed to decode a request ADU.
    Decode(DecodeError),
    /// The peer disconnected or the stream reached end-of-file.
    Disconnected,
}

impl core::fmt::Display for AsciiServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "ASCII server I/O error: {e}"),
            Self::Encode(e) => write!(f, "ASCII server encode error: {e}"),
            Self::Decode(e) => write!(f, "ASCII server decode error: {e}"),
            Self::Disconnected => write!(f, "ASCII server disconnected"),
        }
    }
}

impl std::error::Error for AsciiServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
            Self::Disconnected => None,
        }
    }
}

impl From<std::io::Error> for AsciiServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<EncodeError> for AsciiServerError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

impl From<DecodeError> for AsciiServerError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

/// A synchronous ASCII Modbus server.
///
/// The server listens on a byte stream, decodes ASCII ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into an
/// ASCII ADU. Requests addressed to a different slave (and not broadcast) are
/// ignored.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct AsciiServer<D: DataStore> {
    server: Server<D>,
}

#[cfg(feature = "sync")]
impl<D: DataStore> AsciiServer<D> {
    /// Create a new ASCII server backed by `store`.
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
    ) -> Result<Option<usize>, AsciiServerError> {
        let request = self.read_adu(stream)?;

        #[cfg(feature = "tracing")]
        tracing::trace!(
            protocol = "ascii",
            server_address,
            request_address = request.address,
            function_code = request.pdu.first().copied().unwrap_or(0),
            is_broadcast = request.is_broadcast(),
            "serving ASCII request"
        );

        if request.address != server_address && !request.is_broadcast() {
            #[cfg(feature = "tracing")]
            tracing::trace!(
                protocol = "ascii",
                server_address,
                request_address = request.address,
                "ignoring request for different server address"
            );
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch_with_hook(server_address, &request.pdu, &mut pdu_response)?;

        if request.is_broadcast() {
            #[cfg(feature = "tracing")]
            tracing::trace!(protocol = "ascii", server_address, "broadcast request, no response written");
            return Ok(None);
        }

        #[cfg(feature = "tracing")]
        tracing::trace!(
            protocol = "ascii",
            server_address,
            request_address = request.address,
            response_len = n,
            "wrote ASCII response"
        );

        let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
        let mut tx = [0u8; 513];
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
    ) -> Result<(), AsciiServerError> {
        loop {
            match self.serve_one(stream, server_address) {
                Ok(_) => {}
                Err(AsciiServerError::Disconnected) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    /// Open a local serial port and continuously serve ASCII requests on it.
    ///
    /// `path` is the platform-specific serial device name (e.g. `/dev/ttyUSB0`
    /// on Linux or `COM3` on Windows). The port is configured for 8 data bits,
    /// no parity, 1 stop bit, and a 100 ms read timeout. The function returns
    /// when the serial port is disconnected or an unrecoverable error occurs.
    #[cfg(feature = "sync-serial")]
    pub fn serve_serial(
        &mut self,
        path: impl AsRef<std::path::Path>,
        baud_rate: u32,
        server_address: u8,
    ) -> Result<(), AsciiServerError> {
        let mut serial = crate::serial_transport::open_serial_port(path, baud_rate)
            .map_err(|e| AsciiServerError::Io(e.into()))?;
        self.serve(&mut serial, server_address)
    }

    fn read_adu<T: Read>(&mut self, stream: &mut T) -> Result<AsciiAdu, AsciiServerError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];
        let mut eof = false;

        loop {
            match stream.read(&mut byte) {
                Ok(0) => {
                    if frame.is_empty() {
                        return Err(AsciiServerError::Disconnected);
                    }
                    eof = true;
                    break;
                }
                Ok(1) => {
                    if frame.is_empty() && byte[0] != AsciiAdu::START {
                        // Discard leading garbage until the start character.
                        continue;
                    }
                    frame.push(byte[0]);
                    if frame.len() >= AsciiAdu::MIN_FRAME_SIZE && frame.ends_with(AsciiAdu::END) {
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    if frame.is_empty() {
                        return Err(AsciiServerError::Disconnected);
                    }
                    eof = true;
                    break;
                }
                Err(e) => return Err(AsciiServerError::Io(e)),
            }
        }

        let complete = frame.len() >= AsciiAdu::MIN_FRAME_SIZE && frame.ends_with(AsciiAdu::END);
        AsciiAdu::decode(&frame).map_err(|e| {
            if eof && !complete {
                AsciiServerError::Disconnected
            } else {
                AsciiServerError::Decode(e)
            }
        })
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::ascii::AsciiAdu;
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
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[cfg(feature = "tracing")]
    #[test]
    fn serve_one_emits_trace_with_server_address_and_function_code() {
        use crate::test_trace::test_trace::{with_default, TraceRecorder};

        let recorder = TraceRecorder::new();
        with_default(&recorder, || {
            let mut store = MemoryStore::new(16, 0, 0, 0);
            store.write_coils(0, &[true, false, true, true]).unwrap();

            let mut server = AsciiServer::new(store);
            let request = make_read_coils_adu(0x03, 0, 8);
            let mut stream = Duplex::new(request);
            server.serve_one(&mut stream, 0x03).unwrap().unwrap();
        });

        let events = recorder.events();
        let serve_event = events
            .iter()
            .find(|e| e.fields.iter().any(|(k, v)| k == "message" && v == "serving ASCII request"))
            .expect("serving ASCII request trace event should be emitted");
        assert!(
            serve_event
                .fields
                .iter()
                .any(|(k, v)| k == "server_address" && v == "3"),
            "server trace should include server_address: {:?}",
            serve_event.fields
        );
        assert!(
            serve_event
                .fields
                .iter()
                .any(|(k, v)| k == "function_code" && v == "1"),
            "server trace should include function_code: {:?}",
            serve_event.fields
        );
    }

    #[test]
    fn serve_one_responds_to_matching_address() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsciiServer::new(store);
        let request = make_read_coils_adu(0x03, 0, 8);
        let mut stream = Duplex::new(request);

        let n = server.serve_one(&mut stream, 0x03).unwrap().unwrap();
        assert!(n > 0);
        let response = AsciiAdu::decode(&stream.write_buf).unwrap();
        assert_eq!(response.address, 0x03);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn serve_one_ignores_non_matching_address() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsciiServer::new(store);
        let request = make_read_coils_adu(0x02, 0, 8);
        let mut stream = Duplex::new(request);

        let result = server.serve_one(&mut stream, 0x03).unwrap();
        assert!(result.is_none());
        assert!(stream.write_buf.is_empty());
    }

    #[test]
    fn serve_one_processes_broadcast_without_response() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsciiServer::new(store);

        use crate::function_codes::write_single_coil::WriteSingleCoilRequest;
        let req = WriteSingleCoilRequest::new(0x05, WriteSingleCoilRequest::ON).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(0x00, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let mut stream = Duplex::new(adu[..m].to_vec());
        let result = server.serve_one(&mut stream, 0x03).unwrap();
        assert!(result.is_none());
        assert!(stream.write_buf.is_empty());
        assert!(server.server().store().read_coils(0x05, 1).unwrap()[0] & 0x01 != 0);
    }

    #[test]
    fn serve_one_returns_decode_error_for_bad_lrc() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = AsciiServer::new(store);

        let mut stream = Duplex::new(b":010300000001FA\r\n".to_vec());
        let err = server.serve_one(&mut stream, 0x01).unwrap_err();
        assert!(matches!(
            err,
            AsciiServerError::Decode(DecodeError::InvalidValue)
        ));
    }

    fn make_read_coils_response_adu(slave: u8, coil_status: Vec<u8>) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsResponse;
        let resp = ReadCoilsResponse { coil_status };
        let mut pdu = [0u8; 256];
        let n = resp.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 512];
        let m = AsciiAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[test]
    fn serve_loops_until_eof() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsciiServer::new(store);
        let mut input = make_read_coils_adu(0x07, 0, 8);
        input.extend_from_slice(&make_read_coils_adu(0x07, 0, 8));

        let mut stream = Duplex::new(input);
        server.serve(&mut stream, 0x07).unwrap();

        let expected = make_read_coils_response_adu(0x07, vec![0b00001101]);
        assert_eq!(stream.write_buf.len(), expected.len() * 2);
        assert_eq!(&stream.write_buf[..expected.len()], expected.as_slice());
        assert_eq!(&stream.write_buf[expected.len()..], expected.as_slice());
    }

    #[test]
    fn serve_one_reads_holding_registers() {
        let mut store = MemoryStore::new(0, 0, 4, 0);
        store.write_registers(0, &[0x1234]).unwrap();

        let mut server = AsciiServer::new(store);
        use crate::function_codes::read_holding_registers::ReadHoldingRegistersRequest;
        let req = ReadHoldingRegistersRequest::new(0, 1).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(0x05, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let mut stream = Duplex::new(adu[..m].to_vec());
        let n = server.serve_one(&mut stream, 0x05).unwrap().unwrap();
        assert!(n > 0);
        let response = AsciiAdu::decode(&stream.write_buf).unwrap();
        assert_eq!(response.pdu, vec![0x03, 0x02, 0x12, 0x34]);
    }
}

/// An asynchronous ASCII Modbus server.
///
/// The server listens on an async byte stream, decodes ASCII ADUs, dispatches
/// the contained PDU to a [`DataStore`], and encodes the response back into an
/// ASCII ADU. Requests addressed to a different slave (and not broadcast) are
/// ignored.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncAsciiServer<D: DataStore> {
    server: Server<D>,
}

#[cfg(feature = "async")]
impl<D: DataStore> AsyncAsciiServer<D> {
    /// Create a new async ASCII server backed by `store`.
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
    /// Returns `Some(n)` if a response ADU of `n` bytes was written back to the
    /// stream, or `None` if the request was filtered out or was a broadcast
    /// (which has no response).
    pub async fn serve_one<S>(
        &mut self,
        stream: &mut S,
        server_address: u8,
    ) -> Result<Option<usize>, AsciiServerError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        let request = self.read_adu(stream).await?;

        if request.address != server_address && !request.is_broadcast() {
            return Ok(None);
        }

        let mut pdu_response = [0u8; 512];
        let n = self.server.dispatch_with_hook(server_address, &request.pdu, &mut pdu_response)?;

        if request.is_broadcast() {
            return Ok(None);
        }

        let response = AsciiAdu::new(request.address, pdu_response[..n].to_vec());
        let mut tx = [0u8; 513];
        let m = response.encode(&mut tx)?;
        stream.write_all(&tx[..m]).await?;
        stream.flush().await?;
        Ok(Some(m))
    }

    /// Continuously serve requests on `stream`.
    ///
    /// The function returns when the stream is disconnected or reaches EOF.
    pub async fn serve<S>(
        &mut self,
        stream: &mut S,
        server_address: u8,
    ) -> Result<(), AsciiServerError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        loop {
            match self.serve_one(stream, server_address).await {
                Ok(_) => {}
                Err(AsciiServerError::Disconnected) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    async fn read_adu<S>(&mut self, stream: &mut S) -> Result<AsciiAdu, AsciiServerError>
    where
        S: AsyncReadExt + Unpin,
    {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];
        let mut eof = false;

        loop {
            match stream.read(&mut byte).await {
                Ok(0) => {
                    if frame.is_empty() {
                        return Err(AsciiServerError::Disconnected);
                    }
                    eof = true;
                    break;
                }
                Ok(1) => {
                    if frame.is_empty() && byte[0] != AsciiAdu::START {
                        // Discard leading garbage until the start character.
                        continue;
                    }
                    frame.push(byte[0]);
                    if frame.len() >= AsciiAdu::MIN_FRAME_SIZE
                        && frame.ends_with(AsciiAdu::END)
                        && AsciiAdu::decode(&frame).is_ok()
                    {
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e) => {
                    if e.kind() == std::io::ErrorKind::UnexpectedEof {
                        if frame.is_empty() {
                            return Err(AsciiServerError::Disconnected);
                        }
                        eof = true;
                        break;
                    }
                    return Err(AsciiServerError::Io(e));
                }
            }
        }

        let complete = frame.len() >= AsciiAdu::MIN_FRAME_SIZE && frame.ends_with(AsciiAdu::END);
        AsciiAdu::decode(&frame).map_err(|e| {
            if eof && !complete {
                AsciiServerError::Disconnected
            } else {
                AsciiServerError::Decode(e)
            }
        })
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::ascii::AsciiAdu;
    use crate::server::{DataStore, MemoryStore};

    fn make_read_coils_adu(slave: u8, address: u16, quantity: u16) -> Vec<u8> {
        use crate::function_codes::read_coils::ReadCoilsRequest;
        let req = ReadCoilsRequest::new(address, quantity).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[tokio::test]
    async fn serve_one_responds_to_matching_address() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncAsciiServer::new(store);
        let request = make_read_coils_adu(0x03, 0, 8);
        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(&request).await.unwrap();
        client.flush().await.unwrap();

        let n = server
            .serve_one(&mut server_stream, 0x03)
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0);

        let mut rx = vec![0u8; n];
        client.read_exact(&mut rx).await.unwrap();
        client.shutdown().await.unwrap();
        let response = AsciiAdu::decode(&rx).unwrap();
        assert_eq!(response.address, 0x03);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[tokio::test]
    async fn serve_one_ignores_non_matching_address() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsyncAsciiServer::new(store);
        let request = make_read_coils_adu(0x02, 0, 8);
        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(&request).await.unwrap();
        client.shutdown().await.unwrap();

        let result = server.serve_one(&mut server_stream, 0x03).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn serve_one_processes_broadcast_without_response() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsyncAsciiServer::new(store);

        use crate::function_codes::write_single_coil::WriteSingleCoilRequest;
        let req = WriteSingleCoilRequest::new(0x05, WriteSingleCoilRequest::ON).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(0x00, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(&adu[..m]).await.unwrap();
        client.shutdown().await.unwrap();

        let result = server.serve_one(&mut server_stream, 0x03).await.unwrap();
        assert!(result.is_none());
        assert!(server.server().store().read_coils(0x05, 1).unwrap()[0] & 0x01 != 0);
    }

    #[tokio::test]
    async fn serve_one_returns_decode_error_for_bad_lrc() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let mut server = AsyncAsciiServer::new(store);

        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(b":010300000001FA\r\n").await.unwrap();
        client.shutdown().await.unwrap();

        let err = server
            .serve_one(&mut server_stream, 0x01)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            AsciiServerError::Decode(DecodeError::InvalidValue)
        ));
    }

    #[tokio::test]
    async fn serve_loops_until_eof() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncAsciiServer::new(store);
        let mut input = make_read_coils_adu(0x07, 0, 8);
        input.extend_from_slice(&make_read_coils_adu(0x07, 0, 8));

        let (mut client, mut server_stream) = tokio::io::duplex(1024);

        let server_handle = tokio::spawn(async move {
            server.serve(&mut server_stream, 0x07).await.unwrap();
        });

        client.write_all(&input).await.unwrap();
        client.flush().await.unwrap();
        client.shutdown().await.unwrap();

        let mut rx = Vec::new();
        client.read_to_end(&mut rx).await.unwrap();

        server_handle.await.unwrap();

        fn make_response_adu(slave: u8, coil_status: Vec<u8>) -> Vec<u8> {
            use crate::function_codes::read_coils::ReadCoilsResponse;
            let resp = ReadCoilsResponse { coil_status };
            let mut pdu = [0u8; 256];
            let n = resp.encode(&mut pdu).unwrap();
            let mut adu = [0u8; 512];
            let m = AsciiAdu::new(slave, pdu[..n].to_vec())
                .encode(&mut adu)
                .unwrap();
            adu[..m].to_vec()
        }

        let expected = make_response_adu(0x07, vec![0b00001101]);
        assert_eq!(rx.len(), expected.len() * 2);
        assert_eq!(&rx[..expected.len()], expected.as_slice());
        assert_eq!(&rx[expected.len()..], expected.as_slice());
    }

    #[tokio::test]
    async fn serve_one_reads_holding_registers() {
        let mut store = MemoryStore::new(0, 0, 4, 0);
        store.write_registers(0, &[0x1234]).unwrap();

        let mut server = AsyncAsciiServer::new(store);
        use crate::function_codes::read_holding_registers::ReadHoldingRegistersRequest;
        let req = ReadHoldingRegistersRequest::new(0, 1).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = AsciiAdu::new(0x05, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();

        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(&adu[..m]).await.unwrap();
        client.flush().await.unwrap();

        let n = server
            .serve_one(&mut server_stream, 0x05)
            .await
            .unwrap()
            .unwrap();
        assert!(n > 0);

        let mut rx = vec![0u8; n];
        client.read_exact(&mut rx).await.unwrap();
        client.shutdown().await.unwrap();
        let response = AsciiAdu::decode(&rx).unwrap();
        assert_eq!(response.pdu, vec![0x03, 0x02, 0x12, 0x34]);
    }
}
