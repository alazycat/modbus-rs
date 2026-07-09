//! Configuration-driven Modbus TCP client example.
//!
//! Build with: `cargo build --example config_client --features config,sync,tcp`
//! Loads client settings from a JSON/TOML/YAML string.

use std::net::TcpStream;

use modbus::config::client_from_json;
use modbus::tcp_client::TcpClient;
use modbus::tcp_transport::TcpTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let text = r#"{
        "timeout_ms": 2000,
        "idle_timeout_ms": 30000
    }"#;

    let file = client_from_json(text)?;
    let (config, _retry) = file.into_parts()?;

    let stream = TcpStream::connect("127.0.0.1:502")?;
    let transport = TcpTransport::new(stream);
    let mut client = TcpClient::with_config(transport, config);

    let registers = client.read_holding_registers(1, 0, 4)?;
    println!("read holding registers: {registers:?}");

    Ok(())
}
