//! Asynchronous Modbus TCP server example.
//!
//! Build with: `cargo build --example tcp_server --features async,tcp`
//! Run and connect with any Modbus TCP client on `127.0.0.1:502`.

use tokio::net::TcpListener;

use modbus::server::{MemoryStore, SharedStore};
use modbus::tcp_server::AsyncTcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SharedStore::new(MemoryStore::new(16, 0, 10, 0));
    let mut server = AsyncTcpServer::new(store);

    let listener = TcpListener::bind("127.0.0.1:502").await?;
    println!("Modbus TCP server listening on 127.0.0.1:502");

    loop {
        let (mut stream, peer) = listener.accept().await?;
        println!("accepted connection from {peer}");
        if let Err(e) = server.serve(&mut stream, 1).await {
            eprintln!("connection closed: {e}");
        }
    }
}
