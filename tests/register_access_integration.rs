//! End-to-end integration tests for register-access function codes (03/04/06/10)
//! across all transports and both sync and async runtimes.

#![cfg(any(feature = "sync", feature = "async"))]

use modbus::server::MemoryStore;

const UNIT_ID: u8 = 0x0A;

fn register_access_store() -> MemoryStore {
    let mut store = MemoryStore::new(0, 0, 4, 4);
    store
        .write_input_registers(0, &[0xAABB, 0xCCDD, 0xEEFF])
        .unwrap();
    store
}

fn assert_register_access_results(holding: &[u8], inputs: &[u8]) {
    assert_eq!(
        holding,
        &[0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC],
        "holding registers should reflect writes"
    );
    assert_eq!(
        inputs,
        &[0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF],
        "input registers should match pre-populated values"
    );
}

#[cfg(all(feature = "sync", feature = "rtu"))]
mod sync_rtu {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use modbus::client::Client;
    use modbus::rtu_server::RtuServer;
    use modbus::rtu_transport::RtuTransport;

    use super::*;

    #[test]
    fn register_access_over_rtu() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle =
            thread::spawn(move || -> Result<(), modbus::rtu_server::RtuServerError> {
                let (mut stream, _) = listener.accept()?;
                let mut server = RtuServer::new(register_access_store());
                server.serve(&mut stream, UNIT_ID)
            });

        let stream = TcpStream::connect(addr)?;
        let mut client = Client::new(RtuTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234)?;
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3)?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3)?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.join().unwrap()?;
        Ok(())
    }
}

#[cfg(all(feature = "sync", feature = "tcp"))]
mod sync_tcp {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use modbus::tcp_client::TcpClient;
    use modbus::tcp_server::TcpServer;
    use modbus::tcp_transport::TcpTransport;

    use super::*;

    #[test]
    fn register_access_over_tcp() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle =
            thread::spawn(move || -> Result<(), modbus::tcp_server::TcpServerError> {
                let (mut stream, _) = listener.accept()?;
                let mut server = TcpServer::new(register_access_store());
                server.serve(&mut stream, UNIT_ID)
            });

        let stream = TcpStream::connect(addr)?;
        let mut client = TcpClient::new(TcpTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234)?;
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3)?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3)?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.join().unwrap()?;
        Ok(())
    }
}

#[cfg(all(feature = "sync", feature = "udp"))]
mod sync_udp {
    use std::net::UdpSocket;
    use std::thread;

    use modbus::udp_client::UdpClient;
    use modbus::udp_server::UdpServer;
    use modbus::udp_transport::UdpTransport;

    use super::*;

    #[test]
    fn register_access_over_udp() -> Result<(), Box<dyn std::error::Error>> {
        let server_socket = UdpSocket::bind("127.0.0.1:0")?;
        let server_addr = server_socket.local_addr()?;

        let server_handle =
            thread::spawn(move || -> Result<(), modbus::udp_server::UdpServerError> {
                let mut server = UdpServer::new(register_access_store());
                for _ in 0..4 {
                    server.serve_one(&server_socket, UNIT_ID)?;
                }
                Ok(())
            });

        let client_socket = UdpSocket::bind("127.0.0.1:0")?;
        let mut client = UdpClient::new(UdpTransport::new(client_socket, server_addr));

        client.write_register(UNIT_ID, 0, 0x1234)?;
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3)?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3)?;

        assert_register_access_results(&holding, &inputs);

        server_handle.join().unwrap()?;
        Ok(())
    }
}

#[cfg(all(feature = "sync", feature = "ascii"))]
mod sync_ascii {
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    use modbus::ascii_client::AsciiClient;
    use modbus::ascii_server::AsciiServer;
    use modbus::ascii_transport::AsciiTransport;

    use super::*;

    #[test]
    fn register_access_over_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle = thread::spawn(
            move || -> Result<(), modbus::ascii_server::AsciiServerError> {
                let (mut stream, _) = listener.accept()?;
                let mut server = AsciiServer::new(register_access_store());
                server.serve(&mut stream, UNIT_ID)
            },
        );

        let stream = TcpStream::connect(addr)?;
        let mut client = AsciiClient::new(AsciiTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234)?;
        client.write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3)?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3)?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.join().unwrap()?;
        Ok(())
    }
}

#[cfg(all(feature = "async", feature = "rtu"))]
mod async_rtu {
    use modbus::client::AsyncClient;
    use modbus::rtu_transport::AsyncRtuTransport;
    use modbus::AsyncServer;

    use super::*;

    #[tokio::test]
    async fn register_access_over_rtu() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncServer::new(register_access_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncClient::new(AsyncRtuTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234).await?;
        client
            .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
            .await?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.await??;
        Ok(())
    }
}

#[cfg(all(feature = "async", feature = "tcp"))]
mod async_tcp {
    use modbus::tcp_client::AsyncTcpClient;
    use modbus::tcp_server::AsyncTcpServer;
    use modbus::tcp_transport::AsyncTcpTransport;

    use super::*;

    #[tokio::test]
    async fn register_access_over_tcp() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncTcpServer::new(register_access_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncTcpClient::new(AsyncTcpTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234).await?;
        client
            .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
            .await?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.await??;
        Ok(())
    }
}

#[cfg(all(feature = "async", feature = "udp"))]
mod async_udp {
    use modbus::udp_client::AsyncUdpClient;
    use modbus::udp_server::AsyncUdpServer;
    use modbus::udp_transport::AsyncUdpTransport;

    use super::*;

    #[tokio::test]
    async fn register_access_over_udp() -> Result<(), Box<dyn std::error::Error>> {
        let server_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
        let server_addr = server_socket.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let mut server = AsyncUdpServer::new(register_access_store());
            for _ in 0..4 {
                server.serve_one(&server_socket, UNIT_ID).await?;
            }
            Ok::<_, modbus::udp_server::UdpServerError>(())
        });

        let client_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
        let mut client = AsyncUdpClient::new(AsyncUdpTransport::new(client_socket, server_addr));

        client.write_register(UNIT_ID, 0, 0x1234).await?;
        client
            .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
            .await?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await?;

        assert_register_access_results(&holding, &inputs);

        server_handle.await??;
        Ok(())
    }
}

#[cfg(all(feature = "async", feature = "ascii"))]
mod async_ascii {
    use modbus::ascii_client::AsyncAsciiClient;
    use modbus::ascii_server::AsyncAsciiServer;
    use modbus::ascii_transport::AsyncAsciiTransport;

    use super::*;

    #[tokio::test]
    async fn register_access_over_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncAsciiServer::new(register_access_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncAsciiClient::new(AsyncAsciiTransport::new(stream));

        client.write_register(UNIT_ID, 0, 0x1234).await?;
        client
            .write_registers(UNIT_ID, 1, &[0x5678, 0x9ABC])
            .await?;
        let holding = client.read_holding_registers(UNIT_ID, 0, 3).await?;
        let inputs = client.read_input_registers(UNIT_ID, 0, 3).await?;

        assert_register_access_results(&holding, &inputs);

        drop(client);
        server_handle.await??;
        Ok(())
    }
}
