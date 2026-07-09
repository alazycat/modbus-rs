//! Synchronous Modbus RTU serial client example.
//!
//! Build with: `cargo build --example rtu_client --features sync,rtu,sync-serial`
//! Adjust the serial port path for your platform before running.

use modbus::client::{Client, ClientConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Replace with the correct serial device for your system, e.g. `/dev/ttyUSB0`
    // on Linux or `COM3` on Windows.
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "COM3".to_string());

    let config = ClientConfig::default();
    let mut client = Client::connect_serial_rtu(&path, 9600, config)?;

    let coils = client.read_coils(1, 0, 8)?;
    println!("read coils: {coils:?}");

    Ok(())
}
