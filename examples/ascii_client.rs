//! Synchronous Modbus ASCII serial client example.
//!
//! Build with: `cargo build --example ascii_client --features sync,ascii,sync-serial`
//! Adjust the serial port path for your platform before running.

use modbus::ascii_client::{AsciiClient, AsciiClientConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "COM3".to_string());

    let config = AsciiClientConfig::default();
    let mut client = AsciiClient::connect_serial_ascii(&path, 9600, config)?;

    let registers = client.read_holding_registers(1, 0, 4)?;
    println!("read holding registers: {registers:?}");

    Ok(())
}
