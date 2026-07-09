//! Asynchronous Modbus RTU-over-TCP server example.
//!
//! Build with: `cargo build --example rtu_over_tcp_server --features async,rtu,tcp`
//! Listens on `127.0.0.1:502` and speaks RTU framing over each TCP connection.

use tokio::net::TcpListener;

use modbus::rtu_over_tcp_server::AsyncRtuOverTcpServer;
use modbus::server::{MemoryStore, SharedStore};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SharedStore::new(MemoryStore::new(16, 0, 10, 0));
    let mut server = AsyncRtuOverTcpServer::new(store);

    let listener = TcpListener::bind("127.0.0.1:502").await?;
    println!("Modbus RTU-over-TCP server listening on 127.0.0.1:502");

    server.serve(listener, 1).await?;
    Ok(())
}
