//! Asynchronous Modbus UDP server example.
//!
//! Build with: `cargo build --example udp_server --features async,udp`
//! Responds to datagrams sent to `127.0.0.1:502`.

use tokio::net::UdpSocket;

use modbus::server::{MemoryStore, SharedStore};
use modbus::udp_server::AsyncUdpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let store = SharedStore::new(MemoryStore::new(16, 0, 10, 0));
    let mut server = AsyncUdpServer::new(store);

    let socket = UdpSocket::bind("127.0.0.1:502").await?;
    println!("Modbus UDP server listening on 127.0.0.1:502");

    server.serve(&socket, 1).await?;
    Ok(())
}
