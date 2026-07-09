//! Asynchronous RTU Modbus server for arbitrary byte streams.
//!
//! This server accepts any `AsyncRead + AsyncWrite` stream, decodes RTU ADUs,
//! dispatches the contained PDU to a [`DataStore`], and encodes the response
//! back into an RTU ADU. It is functionally equivalent to [`AsyncServer`], but
//! lives in its own module and can be extended with transport-specific helpers
//! such as [`AsyncRtuServer::serve_serial`].

#![cfg(all(feature = "rtu", feature = "async"))]

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::server::r#async::{serve_rtu_request, AsyncServerError};
use crate::server::{DataStore, Server};

/// An asynchronous Modbus RTU server that accepts any async byte stream.
#[derive(Debug)]
pub struct AsyncRtuServer<D: DataStore> {
    server: Server<D>,
}

impl<D: DataStore> AsyncRtuServer<D> {
    /// Create a new async RTU server backed by `store`.
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
    /// address did not match `server_address` or because it was a broadcast.
    pub async fn serve_one<S>(
        &mut self,
        stream: &mut S,
        server_address: u8,
    ) -> Result<Option<usize>, AsyncServerError>
    where
        S: AsyncReadExt + AsyncWriteExt + Unpin,
    {
        serve_rtu_request(&mut self.server, stream, server_address).await
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

    /// Open a serial port and continuously serve RTU requests on it.
    ///
    /// This is a convenience helper that uses the common Modbus RTU serial
    /// settings (8 data bits, no parity, 1 stop bit, 100 ms timeout). The
    /// function returns when the serial stream is disconnected or reaches EOF.
    #[cfg(feature = "serial")]
    pub async fn serve_serial(
        &mut self,
        path: impl AsRef<std::path::Path>,
        baud_rate: u32,
        server_address: u8,
    ) -> Result<(), AsyncServerError> {
        let mut transport = crate::rtu_transport::open_serial_rtu(path, baud_rate)
            .await
            .map_err(|e| AsyncServerError::Io(std::io::Error::other(e.to_string())))?;
        self.serve(transport.stream_mut(), server_address).await
    }
}

#[cfg(all(test, feature = "serial"))]
mod serial_tests {
    use super::*;
    use crate::server::MemoryStore;

    #[tokio::test]
    async fn serve_serial_rejects_invalid_path() {
        let mut server = AsyncRtuServer::new(MemoryStore::new(0, 0, 0, 0));
        let err = server
            .serve_serial("COM99999", 9600, 0x01)
            .await
            .unwrap_err();
        assert!(matches!(err, AsyncServerError::Io(_)));
    }
}
