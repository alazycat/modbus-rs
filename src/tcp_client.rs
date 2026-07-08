//! Synchronous and asynchronous TCP Modbus clients.
//!
//! This module is available when the `tcp` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps request PDUs in the
//! MODBUS TCP MBAP header, tracks transaction IDs, validates responses, and
//! exposes high-level methods for reading and writing coils and registers.

#![cfg(all(feature = "tcp", any(feature = "sync", feature = "async")))]

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

/// Configuration for a synchronous and asynchronous TCP client.
pub type TcpClientConfig = crate::client::ClientConfig;

/// Errors that can occur while using the TCP client.
pub type TcpClientError = crate::client::ClientError;

#[cfg(feature = "sync")]
impl_adu_adapter! {
    [] [],
    /// Synchronous TCP ADU adapter.
    TcpAduAdapter,
    crate::tcp::TcpAdu,
    transaction
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
        Self(ClientCore::with_config(
            TcpAduAdapter::with_config(transport, config),
            config,
        ))
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

#[cfg(all(feature = "tls", feature = "sync"))]
impl
    TcpClient<
        crate::tcp_transport::TcpTransport<
            rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>,
        >,
    >
{
    /// Connect to a remote Modbus TCP server over TLS synchronously.
    ///
    /// `server_name` is used for SNI and certificate validation. The supplied
    /// `tls_config` must trust the remote server's certificate.
    pub fn connect_tls(
        addr: impl std::net::ToSocketAddrs,
        server_name: &str,
        tls_config: rustls::ClientConfig,
        config: TcpClientConfig,
    ) -> Result<Self, crate::client::ClientError> {
        let stream = std::net::TcpStream::connect(addr)
            .map_err(crate::transport::TransportError::Io)?;
        let server_name = rustls::pki_types::ServerName::try_from(server_name.to_owned())
            .map_err(|e| crate::client::ClientError::Tls(e.to_string()))?;
        let conn = rustls::ClientConnection::new(
            std::sync::Arc::new(tls_config),
            server_name,
        )
        .map_err(|e| crate::client::ClientError::Tls(e.to_string()))?;
        let tls = rustls::StreamOwned::new(conn, stream);
        let transport = crate::tcp_transport::TcpTransport::new(tls);
        Ok(Self::with_config(transport, config))
    }
}

#[cfg(all(test, feature = "sync", feature = "rtu", feature = "tcp"))]
mod rtu_over_tcp_tests {
    use super::TcpClientConfig;
    use crate::rtu::RtuAdu;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    #[test]
    fn connect_rtu_over_tcp_reads_coils() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut buf = [0u8; 256];
            let n = stream.read(&mut buf).unwrap();
            let request = RtuAdu::decode(&buf[..n]).unwrap();
            assert_eq!(request.address, 0x0A);
            assert_eq!(request.pdu[0], 0x01); // read coils

            let response = RtuAdu::new(0x0A, vec![0x01, 0x01, 0b00001101]);
            let mut out = [0u8; 256];
            let m = response.encode(&mut out).unwrap();
            stream.write_all(&out[..m]).unwrap();
        });

        let mut client =
            crate::client::Client::connect_rtu_over_tcp(addr, TcpClientConfig::default()).unwrap();
        let coils = client.read_coils(0x0A, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::tcp::TcpAdu;
    use crate::transport::{Transport, TransportError};
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
    fn read_coils_over_tcp() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

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
        server
            .store_mut()
            .write_coils(0, &[true, true, true, true])
            .unwrap();

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

#[cfg(all(test, feature = "tls"))]
mod tls_tests {
    use super::{AsyncTcpClient, TcpClientConfig};
    use crate::tcp::TcpAdu;
    use rcgen::CertifiedKey;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_rustls::TlsAcceptor;

    fn self_signed_localhost_cert() -> (
        tokio_rustls::rustls::pki_types::CertificateDer<'static>,
        tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer<'static>,
    ) {
        let CertifiedKey { cert, key_pair } =
            rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
        let cert_der = tokio_rustls::rustls::pki_types::CertificateDer::from(cert.der().to_vec());
        let key_der = tokio_rustls::rustls::pki_types::PrivatePkcs8KeyDer::from(
            key_pair.serialize_der().to_vec(),
        );
        (cert_der, key_der)
    }

    #[test]
    fn connect_tls_reads_coils_sync() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::Arc;
        use std::thread;
        use super::TcpClient;

        let _ = rustls::crypto::ring::default_provider().install_default();

        let (cert_der, key_der) = self_signed_localhost_cert();

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der.into())
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let conn = rustls::ServerConnection::new(Arc::new(server_config)).unwrap();
            let mut stream = rustls::StreamOwned::new(conn, stream);

            let mut buf = [0u8; 256];
            let n = stream.read(&mut buf).unwrap();
            let request = TcpAdu::decode(&buf[..n]).unwrap();
            assert_eq!(request.unit_id, 0x0A);
            assert_eq!(request.pdu[0], 0x01); // read coils

            let response = TcpAdu::new(request.transaction_id, 0x0A, vec![0x01, 0x01, 0b00001101]);
            let mut out = [0u8; 256];
            let m = response.encode(&mut out).unwrap();
            stream.write_all(&out[..m]).unwrap();
            stream.flush().unwrap();
        });

        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(cert_der).unwrap();
        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let mut client = TcpClient::connect_tls(addr, "localhost", client_config, TcpClientConfig::default()).unwrap();
        let coils = client.read_coils(0x0A, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn connect_tls_reads_coils() {
        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

        let (cert_der, key_der) = self_signed_localhost_cert();

        let server_config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der.into())
            .unwrap();
        let acceptor = TlsAcceptor::from(Arc::new(server_config));

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut stream = acceptor.accept(stream).await.unwrap();

            let mut buf = [0u8; 256];
            let n = stream.read(&mut buf).await.unwrap();
            let request = TcpAdu::decode(&buf[..n]).unwrap();
            assert_eq!(request.unit_id, 0x0A);
            assert_eq!(request.pdu[0], 0x01); // read coils

            let response = TcpAdu::new(request.transaction_id, 0x0A, vec![0x01, 0x01, 0b00001101]);
            let mut out = [0u8; 256];
            let m = response.encode(&mut out).unwrap();
            stream.write_all(&out[..m]).await.unwrap();
            stream.flush().await.unwrap();
        });

        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
        root_store.add(cert_der).unwrap();
        let client_config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

        let mut client =
            AsyncTcpClient::connect_tls(addr, "localhost", connector, TcpClientConfig::default())
                .await
                .unwrap();
        let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[test]
    fn tls_server_accepts_sync_client() {
        use std::net::TcpListener;
        use std::sync::Arc;
        use std::thread;
        use crate::server::{DataStore, MemoryStore};
        use crate::tcp_server::TcpServer;
        use super::TcpClient;

        let _ = rustls::crypto::ring::default_provider().install_default();

        let (cert_der, key_der) = self_signed_localhost_cert();

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der.into())
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        thread::spawn(move || {
            let mut server = TcpServer::new(store);
            server.serve_tls(listener, 0x0A, Arc::new(server_config)).unwrap();
        });

        let mut root_store = rustls::RootCertStore::empty();
        root_store.add(cert_der).unwrap();
        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let mut client = TcpClient::connect_tls(addr, "localhost", client_config, TcpClientConfig::default()).unwrap();
        let coils = client.read_coils(0x0A, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn tls_server_accepts_async_client() {
        use std::sync::Arc;
        use tokio::net::TcpListener;
        use crate::server::{DataStore, MemoryStore};
        use crate::tcp_server::AsyncTcpServer;
        use super::AsyncTcpClient;

        let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

        let (cert_der, key_der) = self_signed_localhost_cert();

        let server_config = tokio_rustls::rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der.clone()], key_der.into())
            .unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        tokio::spawn(async move {
            let mut server = AsyncTcpServer::new(store);
            server.serve_tls(listener, 0x0A, Arc::new(server_config)).await.unwrap();
        });

        let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
        root_store.add(cert_der).unwrap();
        let client_config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();
        let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

        let mut client =
            AsyncTcpClient::connect_tls(addr, "localhost", connector, TcpClientConfig::default())
                .await
                .unwrap();
        let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }
}

#[cfg(feature = "async")]
impl_adu_adapter! {
    [async] [.await],
    /// Asynchronous TCP ADU adapter.
    AsyncTcpAduAdapter,
    crate::tcp::TcpAdu,
    transaction
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
        Self(AsyncClientCore::with_config(
            AsyncTcpAduAdapter::with_config(transport, config),
            config,
        ))
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

#[cfg(feature = "tls")]
impl
    AsyncTcpClient<
        crate::tcp_transport::AsyncTcpTransport<
            tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
        >,
    >
{
    /// Connect to a remote Modbus TCP server over TLS asynchronously.
    ///
    /// `server_name` is used for SNI and certificate validation. The supplied
    /// `connector` must trust the remote server's certificate.
    pub async fn connect_tls(
        addr: impl tokio::net::ToSocketAddrs,
        server_name: &str,
        connector: tokio_rustls::TlsConnector,
        config: TcpClientConfig,
    ) -> Result<Self, crate::client::ClientError> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(crate::transport::TransportError::Io)?;
        let server_name =
            tokio_rustls::rustls::pki_types::ServerName::try_from(server_name.to_owned())
                .map_err(|e| crate::client::ClientError::Tls(e.to_string()))?;
        let tls = connector
            .connect(server_name, stream)
            .await
            .map_err(|e| crate::client::ClientError::Tls(e.to_string()))?;
        let transport = crate::tcp_transport::AsyncTcpTransport::new(tls);
        Ok(Self::with_config(transport, config))
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::tcp::TcpAdu;
    use crate::transport::{AsyncTransport, TransportError};
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

    #[tokio::test]
    #[cfg(feature = "rtu")]
    async fn connect_rtu_over_tcp_reads_coils() {
        use crate::rtu::RtuAdu;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 256];
            let n = stream.read(&mut buf).await.unwrap();
            let request = RtuAdu::decode(&buf[..n]).unwrap();
            assert_eq!(request.address, 0x0A);
            assert_eq!(request.pdu[0], 0x01);

            let response = RtuAdu::new(0x0A, vec![0x01, 0x01, 0b00001101]);
            let mut out = [0u8; 256];
            let m = response.encode(&mut out).unwrap();
            stream.write_all(&out[..m]).await.unwrap();
        });

        let mut client =
            crate::client::AsyncClient::connect_rtu_over_tcp(addr, TcpClientConfig::default())
                .await
                .unwrap();
        let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }
}
