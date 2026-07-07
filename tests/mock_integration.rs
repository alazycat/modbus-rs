//! Integration tests using the in-memory mock (loopback) transport harness.

#![cfg(any(feature = "sync", feature = "async"))]

mod common;

use common::{assert_register_results, register_store};

const UNIT_ID: u8 = 0x0A;

#[cfg(all(feature = "sync", feature = "rtu"))]
mod sync_rtu {
    use modbus::client::Client;
    use modbus::Server;

    use super::*;
    use common::sync_rtu::RtuLoopback;

    #[test]
    fn register_access_over_rtu_loopback() {
        let server = Server::new(register_store());
        let mut client = Client::new(RtuLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "sync", feature = "tcp"))]
mod sync_tcp {
    use modbus::tcp_client::TcpClient;
    use modbus::Server;

    use super::*;
    use common::sync_tcp::TcpLoopback;

    #[test]
    fn register_access_over_tcp_loopback() {
        let server = Server::new(register_store());
        let mut client = TcpClient::new(TcpLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "sync", feature = "udp"))]
mod sync_udp {
    use modbus::udp_client::UdpClient;
    use modbus::Server;

    use super::*;
    use common::sync_udp::UdpLoopback;

    #[test]
    fn register_access_over_udp_loopback() {
        let server = Server::new(register_store());
        let mut client = UdpClient::new(UdpLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "sync", feature = "ascii"))]
mod sync_ascii {
    use modbus::ascii_client::AsciiClient;
    use modbus::Server;

    use super::*;
    use common::sync_ascii::AsciiLoopback;

    #[test]
    fn register_access_over_ascii_loopback() {
        let server = Server::new(register_store());
        let mut client = AsciiClient::new(AsciiLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "async", feature = "rtu"))]
mod async_rtu {
    use modbus::client::AsyncClient;
    use modbus::AsyncServer;

    use super::*;
    use common::async_rtu::AsyncRtuLoopback;

    #[tokio::test]
    async fn register_access_over_rtu_loopback() {
        let server = AsyncServer::new(register_store());
        let mut client = AsyncClient::new(AsyncRtuLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).await.unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "async", feature = "tcp"))]
mod async_tcp {
    use modbus::tcp_client::AsyncTcpClient;
    use modbus::AsyncServer;

    use super::*;
    use common::async_tcp::AsyncTcpLoopback;

    #[tokio::test]
    async fn register_access_over_tcp_loopback() {
        let server = AsyncServer::new(register_store());
        let mut client = AsyncTcpClient::new(AsyncTcpLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).await.unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "async", feature = "udp"))]
mod async_udp {
    use modbus::udp_client::AsyncUdpClient;
    use modbus::AsyncServer;

    use super::*;
    use common::async_udp::AsyncUdpLoopback;

    #[tokio::test]
    async fn register_access_over_udp_loopback() {
        let server = AsyncServer::new(register_store());
        let mut client = AsyncUdpClient::new(AsyncUdpLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).await.unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

        assert_register_results(&holding, &inputs);
    }
}

#[cfg(all(feature = "async", feature = "ascii"))]
mod async_ascii {
    use modbus::ascii_client::AsyncAsciiClient;
    use modbus::AsyncServer;

    use super::*;
    use common::async_ascii::AsyncAsciiLoopback;

    #[tokio::test]
    async fn register_access_over_ascii_loopback() {
        let server = AsyncServer::new(register_store());
        let mut client = AsyncAsciiClient::new(AsyncAsciiLoopback::new(server));

        client.write_register(UNIT_ID, 0, 0x1234).await.unwrap();
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC]).await.unwrap();
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await.unwrap();
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await.unwrap();

        assert_register_results(&holding, &inputs);
    }
}
