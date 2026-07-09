//! Asynchronous Modbus TLS-over-TCP client example.
//!
//! Build with: `cargo build --example tls_client --features async,tcp,tls`
//! Run against a server that serves a certificate trusted by `ca-cert.pem`.
//!
//! The example reads a PEM-encoded CA certificate from a file and builds a
//! `rustls` client config; it then connects to `127.0.0.1:802` using SNI
//! `localhost`.

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use modbus::client::ClientConfig;
use modbus::tcp_client::AsyncTcpClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cert_path = std::env::args().nth(1).unwrap_or_else(|| "ca-cert.pem".to_string());
    let certs = rustls_pemfile::certs(&mut BufReader::new(File::open(&cert_path)?))
        .collect::<Result<Vec<_>, _>>()?;

    let mut root_store = tokio_rustls::rustls::RootCertStore::empty();
    for cert in certs {
        root_store.add(cert)?;
    }

    let client_config = tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let config = ClientConfig::default();

    let mut client = AsyncTcpClient::connect_tls("127.0.0.1:802", "localhost", connector, config).await?;

    let registers = client.read_holding_registers(1, 0, 4).await?;
    println!("read holding registers: {registers:?}");

    Ok(())
}
