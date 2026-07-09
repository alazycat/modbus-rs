//! Configuration-driven Modbus TCP server example.
//!
//! Build with: `cargo build --example config_server --features config,async,tcp`
//! Loads server settings from a JSON/TOML/YAML string and starts an async TCP
//! server.

use tokio::net::TcpListener;

use modbus::config::server_from_json;
use modbus::server::{MemoryStore, SharedStore};
use modbus::tcp_server::AsyncTcpServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"{
        "bind_address": "127.0.0.1:502",
        "transport": "tcp",
        "unit_id": 1,
        "coils": 16,
        "holding_registers": 10
    }"#;

    let cfg = server_from_json(text)?;
    let store = SharedStore::new(MemoryStore::new(
        cfg.coils,
        cfg.discrete_inputs,
        cfg.holding_registers,
        cfg.input_registers,
    ));
    let mut server = AsyncTcpServer::new(store);

    let listener = TcpListener::bind(&cfg.bind_address).await?;
    println!("Modbus TCP server listening on {}", cfg.bind_address);

    loop {
        let (mut stream, peer) = listener.accept().await?;
        println!("accepted connection from {peer}");
        if let Err(e) = server.serve(&mut stream, cfg.unit_id).await {
            eprintln!("connection closed: {e}");
        }
    }
}
