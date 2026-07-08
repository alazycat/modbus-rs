//! End-to-end TLS integration tests.
//!
//! These tests require the `sync`, `tcp`, and `tls` features. They start a
//! self-signed TLS Modbus server on localhost and verify that sync and async
//! clients can read coils over the encrypted channel.

#![cfg(all(feature = "sync", feature = "tcp", feature = "tls"))]

use modbus::DataStore;
use rcgen::CertifiedKey;
use std::sync::Arc;

fn self_signed_localhost_cert() -> (
    rustls::pki_types::CertificateDer<'static>,
    rustls::pki_types::PrivatePkcs8KeyDer<'static>,
) {
    let CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()]).unwrap();
    let cert_der = rustls::pki_types::CertificateDer::from(cert.der().to_vec());
    let key_der =
        rustls::pki_types::PrivatePkcs8KeyDer::from(key_pair.serialize_der().to_vec());
    (cert_der, key_der)
}

#[test]
fn sync_tls_client_server_read_coils() {
    let _ = rustls::crypto::ring::default_provider().install_default();

    let (cert_der, key_der) = self_signed_localhost_cert();

    let server_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der.into())
        .unwrap();

    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();

    let mut store = modbus::MemoryStore::new(16, 0, 0, 0);
    store.write_coils(0, &[true, false, true, true]).unwrap();

    std::thread::spawn(move || {
        let mut tcp_server = modbus::tcp_server::TcpServer::new(store);
        tcp_server
            .serve_tls(listener, 0x0A, Arc::new(server_config))
            .unwrap();
    });

    let mut root_store = rustls::RootCertStore::empty();
    root_store.add(cert_der).unwrap();
    let client_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let mut client = modbus::tcp_client::TcpClient::connect_tls(
        addr,
        "localhost",
        client_config,
        modbus::client::ClientConfig::default(),
    )
    .unwrap();
    let coils = client.read_coils(0x0A, 0, 8).unwrap();
    assert_eq!(coils, vec![0b00001101]);
}

#[tokio::test]
async fn async_tls_client_server_read_coils() {
    let _ = tokio_rustls::rustls::crypto::ring::default_provider().install_default();

    let (cert_der, key_der) = self_signed_localhost_cert();

    let server_config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(vec![cert_der.clone()], key_der.into())
        .unwrap();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let mut store = modbus::MemoryStore::new(16, 0, 0, 0);
    store.write_coils(0, &[true, false, true, true]).unwrap();

    tokio::spawn(async move {
        let mut tcp_server = modbus::tcp_server::AsyncTcpServer::new(store);
        tcp_server
            .serve_tls(listener, 0x0A, Arc::new(server_config))
            .await
            .unwrap();
    });

    let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
    root_store.add(cert_der).unwrap();
    let client_config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));

    let mut client = modbus::tcp_client::AsyncTcpClient::connect_tls(
        addr,
        "localhost",
        connector,
        modbus::client::ClientConfig::default(),
    )
    .await
    .unwrap();
    let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
    assert_eq!(coils, vec![0b00001101]);
}
