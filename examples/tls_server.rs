//! Asynchronous Modbus TLS-over-TCP server example.
//!
//! Build with: `cargo build --example tls_server --features async,tcp,tls`
//! Run with PEM-encoded certificate and key files:
//!
//! ```sh
//! cargo run --example tls_server --features async,tcp,tls -- server.crt server.key
//! ```
//!
//! Generate test files with OpenSSL:
//!
//! ```sh
//! openssl req -x509 -newkey rsa:2048 -keyout server.key -out server.crt -days 7 \
//!   -nodes -subj /CN=localhost
//! ```

use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;

use tokio::net::TcpListener;

use modbus::server::{MemoryStore, SharedStore};
use modbus::tcp_server::AsyncTcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cert_path = std::env::args().nth(1).unwrap_or_else(|| "server.crt".to_string());
    let key_path = std::env::args().nth(2).unwrap_or_else(|| "server.key".to_string());

    let certs: Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>> =
        rustls_pemfile::certs(&mut BufReader::new(File::open(&cert_path)?)
        )
        .collect::<Result<Vec<_>, _>>()?;
    let key = rustls_pemfile::private_key(&mut BufReader::new(File::open(&key_path)?)
    )?
    .ok_or("no private key found")?;

    let config = tokio_rustls::rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)?;

    let store = SharedStore::new(MemoryStore::new(16, 0, 10, 0));
    let mut server = AsyncTcpServer::new(store);

    let listener = TcpListener::bind("127.0.0.1:802").await?;
    println!("Modbus TLS server listening on 127.0.0.1:802");

    server.serve_tls(listener, 1, Arc::new(config)).await?;
    Ok(())
}
