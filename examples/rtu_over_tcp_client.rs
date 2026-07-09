//! Synchronous Modbus RTU-over-TCP client example.
//!
//! Build with: `cargo build --example rtu_over_tcp_client --features sync,rtu,tcp`
//! Connects to an RTU-over-TCP gateway on `127.0.0.1:502`.

use modbus::client::{Client, ClientConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ClientConfig::default();
    let mut client = Client::connect_rtu_over_tcp("127.0.0.1:502", config)?;

    let coils = client.read_coils(1, 0, 8)?;
    println!("read coils: {coils:?}");

    Ok(())
}
