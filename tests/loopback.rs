#![cfg(any(feature = "sync", feature = "async"))]

//! In-memory loopback integration tests.
//!
//! These tests exercise every sync/async × protocol combination using a
//! loopback transport that dispatches requests directly to an in-memory
//! [`DataStore`]. They do not require any network or serial hardware and run
//! under the default `cargo test` feature set.

mod common;

const UNIT_ID: u8 = 0x0A;

macro_rules! sync_loopback_test {
    ($name:ident, $client:ty, $transport:expr) => {
        #[cfg(all(feature = "sync"))]
        #[test]
        fn $name() {
            let mut client = <$client>::new($transport);

            client.write_register(UNIT_ID, 0, 0x1234).unwrap();
            client
                .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
                .unwrap();
            let holding = client.read_holding_registers(UNIT_ID, 0, 3).unwrap();
            let inputs = client.read_input_registers(UNIT_ID, 0, 3).unwrap();

            common::assert_register_results(&holding, &inputs);
        }
    };
}

macro_rules! async_loopback_test {
    ($name:ident, $client:ty, $transport:expr) => {
        #[cfg(all(feature = "async"))]
        #[tokio::test]
        async fn $name() {
            let mut client = <$client>::new($transport);

            client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
            client
                .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
                .await
                .unwrap();
            let holding = client
                .read_holding_registers(UNIT_ID, 0, 3)
                .await
                .unwrap();
            let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

            common::assert_register_results(&holding, &inputs);
        }
    };
}

#[cfg(all(feature = "sync", feature = "rtu"))]
sync_loopback_test! {
    sync_rtu_loopback_register_access,
    modbus::client::Client<common::sync_rtu::RtuLoopback>,
    common::sync_rtu::RtuLoopback::new(modbus::server::Server::new(common::register_store()))
}

#[cfg(all(feature = "sync", feature = "tcp"))]
sync_loopback_test! {
    sync_tcp_loopback_register_access,
    modbus::tcp_client::TcpClient<common::sync_tcp::TcpLoopback>,
    common::sync_tcp::TcpLoopback::new(modbus::server::Server::new(common::register_store()))
}

#[cfg(all(feature = "sync", feature = "udp"))]
sync_loopback_test! {
    sync_udp_loopback_register_access,
    modbus::udp_client::UdpClient<common::sync_udp::UdpLoopback>,
    common::sync_udp::UdpLoopback::new(modbus::server::Server::new(common::register_store()))
}

#[cfg(all(feature = "sync", feature = "ascii"))]
sync_loopback_test! {
    sync_ascii_loopback_register_access,
    modbus::ascii_client::AsciiClient<common::sync_ascii::AsciiLoopback>,
    common::sync_ascii::AsciiLoopback::new(modbus::server::Server::new(common::register_store()))
}

#[cfg(all(feature = "async", feature = "rtu"))]
async_loopback_test! {
    async_rtu_loopback_register_access,
    modbus::client::AsyncClient<common::async_rtu::AsyncRtuLoopback>,
    common::async_rtu::AsyncRtuLoopback::new(modbus::AsyncServer::new(common::register_store()))
}

#[cfg(all(feature = "async", feature = "tcp"))]
async_loopback_test! {
    async_tcp_loopback_register_access,
    modbus::tcp_client::AsyncTcpClient<common::async_tcp::AsyncTcpLoopback>,
    common::async_tcp::AsyncTcpLoopback::new(modbus::AsyncServer::new(common::register_store()))
}

#[cfg(all(feature = "async", feature = "udp"))]
async_loopback_test! {
    async_udp_loopback_register_access,
    modbus::udp_client::AsyncUdpClient<common::async_udp::AsyncUdpLoopback>,
    common::async_udp::AsyncUdpLoopback::new(modbus::AsyncServer::new(common::register_store()))
}

#[cfg(all(feature = "async", feature = "ascii"))]
async_loopback_test! {
    async_ascii_loopback_register_access,
    modbus::ascii_client::AsyncAsciiClient<common::async_ascii::AsyncAsciiLoopback>,
    common::async_ascii::AsyncAsciiLoopback::new(modbus::AsyncServer::new(common::register_store()))
}
