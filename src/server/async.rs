//! Asynchronous Modbus server dispatcher.

#![cfg(feature = "async")]

use std::io;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{DataStore, Server};
use crate::error::{DecodeError, EncodeError};
use crate::rtu::RtuAdu;

/// Errors that can occur while running the asynchronous server.
#[derive(Debug)]
pub enum AsyncServerError {
    /// An underlying I/O error.
    Io(io::Error),
    /// Failed to encode a response ADU.
    Encode(EncodeError),
    /// Failed to decode a request ADU.
    Decode(DecodeError),
    /// The peer disconnected or the stream reached end-of-file.
    Disconnected,
}

impl core::fmt::Display for AsyncServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "async server I/O error: {e}"),
            Self::Encode(e) => write!(f, "async server encode error: {e}"),
            Self::Decode(e) => write!(f, "async server decode error: {e}"),
            Self::Disconnected => write!(f, "async server disconnected"),
        }
    }
}

impl std::error::Error for AsyncServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for AsyncServerError {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<EncodeError> for AsyncServerError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

impl From<DecodeError> for AsyncServerError {
    fn from(e: DecodeError) -> Self {
        Self::Decode(e)
    }
}

/// An asynchronous Modbus server.
///
/// The server listens on an async byte stream, decodes RTU ADUs, dispatches the
/// contained PDU to a [`DataStore`], and encodes the response back into an RTU
/// ADU. Requests with a non-matching slave address are silently ignored.
#[derive(Debug)]
pub struct AsyncServer<D: DataStore> {
    server: Server<D>,
}

impl<D: DataStore> AsyncServer<D> {
    /// Create a new async server backed by `store`.
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
    /// stream, or `None` if the request was filtered out because its slave
    /// address did not match `server_address`.
    pub async fn serve_one<S>(
        &mut self,
        stream: &mut S,
        server_address: u8,
    ) -> Result<Option<usize>, AsyncServerError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        let request = self.read_adu(stream).await?;

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
    ) -> Result<(), AsyncServerError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        loop {
            match self.serve_one(stream, server_address).await {
                Ok(_) => {}
                Err(AsyncServerError::Disconnected) => return Ok(()),
                Err(e) => return Err(e),
            }
        }
    }

    async fn read_adu<S>(&mut self, stream: &mut S) -> Result<RtuAdu, AsyncServerError>
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
                        return Err(AsyncServerError::Disconnected);
                    }
                    eof = true;
                    break;
                }
                Ok(1) => {
                    frame.push(byte[0]);
                    if frame.len() >= RtuAdu::MIN_FRAME_SIZE && RtuAdu::decode(&frame).is_ok() {
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e) => {
                    if e.kind() == io::ErrorKind::UnexpectedEof {
                        if frame.is_empty() {
                            return Err(AsyncServerError::Disconnected);
                        }
                        eof = true;
                        break;
                    }
                    return Err(AsyncServerError::Io(e));
                }
            }
        }

        let complete = frame.len() >= RtuAdu::MIN_FRAME_SIZE && RtuAdu::decode(&frame).is_ok();
        RtuAdu::decode(&frame).map_err(|e| {
            if eof && !complete {
                AsyncServerError::Disconnected
            } else {
                AsyncServerError::Decode(e)
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_codes::read_coils::ReadCoilsRequest;
    use crate::rtu::RtuAdu;
    use crate::server::{DataStore, MemoryStore};

    fn make_read_coils_adu(slave: u8, address: u16, quantity: u16) -> Vec<u8> {
        let req = ReadCoilsRequest::new(address, quantity).unwrap();
        let mut pdu = [0u8; 5];
        let n = req.encode(&mut pdu).unwrap();
        let mut adu = [0u8; 32];
        let m = RtuAdu::new(slave, pdu[..n].to_vec())
            .encode(&mut adu)
            .unwrap();
        adu[..m].to_vec()
    }

    #[tokio::test]
    async fn serve_one_responds_to_matching_address() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncServer::new(store);
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
        let response = RtuAdu::decode(&rx).unwrap();
        assert_eq!(response.address, 0x03);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }

    #[tokio::test]
    async fn serve_one_ignores_non_matching_address() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsyncServer::new(store);
        let request = make_read_coils_adu(0x02, 0, 8);
        let (mut client, mut server_stream) = tokio::io::duplex(1024);
        client.write_all(&request).await.unwrap();
        client.shutdown().await.unwrap();

        let result = server.serve_one(&mut server_stream, 0x03).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn serve_spawns_task_per_connection() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut server = AsyncServer::new(store);
            server.serve(&mut stream, 0x07).await.unwrap();
        });

        let client_handle = tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
            let request = make_read_coils_adu(0x07, 0, 8);
            stream.write_all(&request).await.unwrap();
            stream.flush().await.unwrap();

            let mut rx = [0u8; 512];
            let n = stream.read(&mut rx).await.unwrap();
            assert!(n > 0);
            let response = RtuAdu::decode(&rx[..n]).unwrap();
            assert_eq!(response.address, 0x07);
            assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);

            // Close the stream so the server's serve() loop returns.
            stream.shutdown().await.unwrap();
        });

        client_handle.await.unwrap();
        server_handle.await.unwrap();
    }
}
