//! Asynchronous Modbus TCP client example.
//!
//! Build with: `cargo build --example async_tcp_client --features async,tcp`
//! Run against a server on `127.0.0.1:502`.

use modbus::client::ClientConfig;
use modbus::tcp_client::AsyncTcpClient;
use modbus::tcp_transport::AsyncTcpTransport;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let stream = tokio::net::TcpStream::connect("127.0.0.1:502").await?;
    let transport = AsyncTcpTransport::new(stream);
    let mut client = AsyncTcpClient::new(transport);

    let registers = client.read_holding_registers(1, 0, 4).await?;
    println!("read holding registers: {registers:?}");

    client.write_register(1, 0, 0x1234).await?;
    println!("wrote 0x1234 to register 0");

    let value = client.read_holding_registers(1, 0, 1).await?;
    println!("read back: {value:?}");

    let _ = ClientConfig::default();
    Ok(())
}
