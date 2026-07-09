//! Typed register helper example.
//!
//! Build with: `cargo build --example typed_helpers --features sync,tcp,helpers`
//! Demonstrates reading and writing `u32`/`f32` values over a TCP connection.
//!
//! This example compiles against a server on `127.0.0.1:502`; it does not run
//! without a server present.

use std::net::TcpStream;

use modbus::client::ClientConfig;
use modbus::helpers::{Endian, WordOrder};
use modbus::tcp_client::TcpClient;
use modbus::tcp_transport::TcpTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect("127.0.0.1:502")?;
    let transport = TcpTransport::new(stream);

    let mut config = ClientConfig::default();
    config.endian = Endian::Big;
    config.word_order = WordOrder::MostSignificantFirst;

    let mut client = TcpClient::with_config(transport, config);

    let value = client.read_holding_registers_u32(1, 0)?;
    println!("read u32: {value}");

    client.write_multiple_registers_u32(1, 0, value + 1)?;
    println!("wrote u32: {}", value + 1);

    let float = client.read_holding_registers_f32(1, 2)?;
    println!("read f32: {float}");

    Ok(())
}
