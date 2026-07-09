//! Synchronous Modbus UDP client example.
//!
//! Build with: `cargo build --example udp_client --features sync,udp`
//! Run against a server on `127.0.0.1:502`.

use std::net::{SocketAddr, UdpSocket};

use modbus::client::ClientConfig;
use modbus::udp_client::UdpClient;
use modbus::udp_transport::UdpTransport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let remote: SocketAddr = "127.0.0.1:502".parse()?;
    let socket = UdpSocket::bind("0.0.0.0:0")?;
    let transport = UdpTransport::new(socket, remote);
    let mut client = UdpClient::new(transport);

    let registers = client.read_holding_registers(1, 0, 4)?;
    println!("read holding registers: {registers:?}");

    client.write_register(1, 0, 0x1234)?;
    println!("wrote 0x1234 to register 0");

    let value = client.read_holding_registers(1, 0, 1)?;
    println!("read back: {value:?}");

    let _ = ClientConfig::default();
    Ok(())
}
