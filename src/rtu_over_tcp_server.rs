//! Synchronous and asynchronous RTU-over-TCP server listeners.
//!
//! This module is available when the `rtu` and `tcp` features and at least one
//! of the `sync` or `async` runtime features are enabled. It wraps the RTU
//! framer around a TCP stream so that requests and responses can be served
//! over a plain TCP connection, commonly used with serial-to-Ethernet gateways.

#![cfg(all(feature = "rtu", feature = "tcp", any(feature = "sync", feature = "async")))]

use crate::server::{DataStore, Server};

/// Errors that can occur while running the RTU-over-TCP server.
#[derive(Debug)]
pub enum RtuOverTcpServerError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// An error from the synchronous RTU server dispatcher.
    #[cfg(feature = "sync")]
    SyncServer(crate::rtu_server::RtuServerError),
    /// An error from the asynchronous RTU server dispatcher.
    #[cfg(feature = "async")]
    AsyncServer(crate::server::r#async::AsyncServerError),
}

impl core::fmt::Display for RtuOverTcpServerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "RTU-over-TCP server I/O error: {e}"),
            #[cfg(feature = "sync")]
            Self::SyncServer(e) => write!(f, "RTU-over-TCP server error: {e}"),
            #[cfg(feature = "async")]
            Self::AsyncServer(e) => write!(f, "RTU-over-TCP server error: {e}"),
        }
    }
}

impl std::error::Error for RtuOverTcpServerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            #[cfg(feature = "sync")]
            Self::SyncServer(e) => Some(e),
            #[cfg(feature = "async")]
            Self::AsyncServer(e) => Some(e),
        }
    }
}

impl From<std::io::Error> for RtuOverTcpServerError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

#[cfg(feature = "sync")]
impl From<crate::rtu_server::RtuServerError> for RtuOverTcpServerError {
    fn from(e: crate::rtu_server::RtuServerError) -> Self {
        Self::SyncServer(e)
    }
}

#[cfg(feature = "async")]
impl From<crate::server::r#async::AsyncServerError> for RtuOverTcpServerError {
    fn from(e: crate::server::r#async::AsyncServerError) -> Self {
        Self::AsyncServer(e)
    }
}

/// A synchronous RTU-over-TCP server.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct RtuOverTcpServer<D: DataStore>(crate::rtu_server::RtuServer<D>);

#[cfg(feature = "sync")]
impl<D: DataStore> RtuOverTcpServer<D> {
    /// Create a new RTU-over-TCP server backed by `store`.
    pub fn new(store: D) -> Self {
        Self(crate::rtu_server::RtuServer::new(store))
    }

    /// Return an immutable reference to the inner [`Server`].
    pub fn server(&self) -> &Server<D> {
        self.0.server()
    }

    /// Return a mutable reference to the inner [`Server`].
    pub fn server_mut(&mut self) -> &mut Server<D> {
        self.0.server_mut()
    }

    /// Continuously serve RTU-framed requests on `listener`.
    ///
    /// Each incoming TCP connection is handled sequentially. The function
    /// returns when the listener is closed or an unrecoverable error occurs.
    pub fn serve(
        &mut self,
        listener: std::net::TcpListener,
        server_address: u8,
    ) -> Result<(), RtuOverTcpServerError> {
        for stream in listener.incoming() {
            let mut stream = stream?;
            self.0.serve(&mut stream, server_address)?;
        }
        Ok(())
    }
}

/// An asynchronous RTU-over-TCP server.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncRtuOverTcpServer<D: DataStore>(crate::server::AsyncServer<D>);

#[cfg(feature = "async")]
impl<D: DataStore> AsyncRtuOverTcpServer<D> {
    /// Create a new async RTU-over-TCP server backed by `store`.
    pub fn new(store: D) -> Self {
        Self(crate::server::AsyncServer::new(store))
    }

    /// Return an immutable reference to the inner [`Server`].
    pub fn server(&self) -> &Server<D> {
        self.0.server()
    }

    /// Return a mutable reference to the inner [`Server`].
    pub fn server_mut(&mut self) -> &mut Server<D> {
        self.0.server_mut()
    }

    /// Continuously serve RTU-framed requests on `listener`.
    ///
    /// Each incoming TCP connection is handled sequentially. The function
    /// returns when the listener is closed or an unrecoverable error occurs.
    pub async fn serve(
        &mut self,
        listener: tokio::net::TcpListener,
        server_address: u8,
    ) -> Result<(), RtuOverTcpServerError> {
        loop {
            let (mut stream, _) = listener.accept().await?;
            self.0.serve(&mut stream, server_address).await?;
        }
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::rtu::RtuAdu;
    use crate::server::MemoryStore;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn sync_rtu_over_tcp_server_responds_to_read_coils() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = RtuOverTcpServer::new(store);
        server
            .server_mut()
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            server.serve(listener, 0x0A).unwrap();
        });

        let mut stream = TcpStream::connect(addr).unwrap();
        let request = RtuAdu::new(0x0A, vec![0x01, 0x00, 0x00, 0x00, 0x08]);
        let mut out = [0u8; 256];
        let n = request.encode(&mut out).unwrap();
        stream.write_all(&out[..n]).unwrap();

        let mut buf = [0u8; 256];
        let m = stream.read(&mut buf).unwrap();
        let response = RtuAdu::decode(&buf[..m]).unwrap();
        assert_eq!(response.address, 0x0A);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::rtu::RtuAdu;
    use crate::server::MemoryStore;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};

    #[tokio::test]
    async fn async_rtu_over_tcp_server_responds_to_read_coils() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = AsyncRtuOverTcpServer::new(store);
        server
            .server_mut()
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            server.serve(listener, 0x0A).await.unwrap();
        });

        let mut stream = TcpStream::connect(addr).await.unwrap();
        let request = RtuAdu::new(0x0A, vec![0x01, 0x00, 0x00, 0x00, 0x08]);
        let mut out = [0u8; 256];
        let n = request.encode(&mut out).unwrap();
        stream.write_all(&out[..n]).await.unwrap();

        let mut buf = [0u8; 256];
        let m = stream.read(&mut buf).await.unwrap();
        let response = RtuAdu::decode(&buf[..m]).unwrap();
        assert_eq!(response.address, 0x0A);
        assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
    }
}
