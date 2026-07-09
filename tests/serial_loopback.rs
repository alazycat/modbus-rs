#![cfg(feature = "loopback-tests")]

//! Optional serial-loopback integration tests.
//!
//! These tests exercise real serial ports (physical or virtual) that are
//! cross-connected so bytes written on the client port arrive on the server
//! port and vice-versa. They are ignored by default because they require
//! external hardware or a virtual null-modem pair.
//!
//! Run with:
//!
//! ```text
//! MODBUS_LOOPBACK_PORTS=/dev/ttyUSB0,/dev/ttyUSB1 \
//!     cargo test --features loopback-tests -- --ignored
//! ```

mod common;

use std::env;
use std::path::Path;
use std::time::Duration;

use common::{assert_register_results, register_store};
use tokio_serial::{DataBits, Parity, SerialStream, StopBits};

use modbus::ascii_client::AsyncAsciiClient;
use modbus::ascii_server::AsyncAsciiServer;
use modbus::ascii_transport::AsyncAsciiTransport;
use modbus::client::AsyncClient;
use modbus::rtu_transport::open_serial_rtu;
use modbus::server::AsyncServer;

const UNIT_ID: u8 = 0x0A;
const BAUD: u32 = 9600;

/// Read the two cross-connected serial port paths from the environment.
fn loopback_ports() -> (String, String) {
    let var = env::var("MODBUS_LOOPBACK_PORTS").expect(
        "MODBUS_LOOPBACK_PORTS must be set to two cross-connected serial ports, \
         e.g. /dev/ttyUSB0,/dev/ttyUSB1 or COM3,COM4",
    );
    let mut parts = var.split(',').map(|s| s.trim().to_string());
    let a = parts
        .next()
        .expect("MODBUS_LOOPBACK_PORTS needs two comma-separated ports");
    let b = parts
        .next()
        .expect("MODBUS_LOOPBACK_PORTS needs two comma-separated ports");
    if parts.next().is_some() {
        panic!("MODBUS_LOOPBACK_PORTS must contain exactly two ports");
    }
    (a, b)
}

/// Open a serial port with the default Modbus ASCII settings and wrap it in an
/// async ASCII transport.
async fn open_serial_ascii(
    path: impl AsRef<Path>,
) -> Result<AsyncAsciiTransport<SerialStream>, tokio_serial::Error> {
    let builder = tokio_serial::new(path.as_ref().to_string_lossy(), BAUD)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .timeout(Duration::from_millis(100));
    let stream = SerialStream::open(&builder)?;
    Ok(AsyncAsciiTransport::new(stream))
}

#[tokio::test]
#[ignore = "requires two cross-connected serial ports"]
async fn rtu_loopback_register_access() {
    let (server_port, client_port) = loopback_ports();

    let mut server_transport = open_serial_rtu(server_port.as_str(), BAUD).await.unwrap();
    let server_task = tokio::spawn(async move {
        let mut server = AsyncServer::new(register_store());
        loop {
            // Swallow errors so a transient client disconnect does not stop
            // serving the next request.
            let _ = server
                .serve_one(server_transport.stream_mut(), UNIT_ID)
                .await;
        }
    });

    // Give the server task time to open its port before the client starts
    // sending requests.
    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_transport = open_serial_rtu(client_port.as_str(), BAUD).await.unwrap();
    let mut client = AsyncClient::new(client_transport);

    client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
    client
        .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
        .await
        .unwrap();
    let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
    let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

    assert_register_results(&holding, &inputs);

    server_task.abort();
}

#[tokio::test]
#[ignore = "requires two cross-connected serial ports"]
async fn ascii_loopback_register_access() {
    let (server_port, client_port) = loopback_ports();

    let mut server_transport = open_serial_ascii(server_port.as_str()).await.unwrap();
    let server_task = tokio::spawn(async move {
        let mut server = AsyncAsciiServer::new(register_store());
        loop {
            let _ = server
                .serve_one(server_transport.stream_mut(), UNIT_ID)
                .await;
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let client_transport = open_serial_ascii(client_port.as_str()).await.unwrap();
    let mut client = AsyncAsciiClient::new(client_transport);

    client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
    client
        .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
        .await
        .unwrap();
    let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
    let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

    assert_register_results(&holding, &inputs);

    server_task.abort();
}
