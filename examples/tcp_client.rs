//! Synchronous Modbus TCP client example.
//!
//! Build with: `cargo build --example tcp_client --features sync,tcp`
//! Run against a server on `127.0.0.1:502`.

use std::net::TcpStream;

use modbus::client::ClientConfig;
use modbus::tcp_client::TcpClient;
use modbus::tcp_transport::TcpTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:502")?;
    let transport = TcpTransport::new(stream);
    let mut client = TcpClient::new(transport);

    let registers = client.read_holding_registers(1, 0, 4)?;
    println!("read holding registers: {registers:?}");

    client.write_register(1, 0, 0x1234)?;
    println!("wrote 0x1234 to register 0");

    let value = client.read_holding_registers(1, 0, 1)?;
    println!("read back: {value:?}");

    let _ = ClientConfig::default();
    Ok(())
}
