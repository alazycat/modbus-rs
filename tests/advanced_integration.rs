//! End-to-end integration tests for advanced/diagnostic function codes
//! (07/08/0B/0C/11/16/17/18) across all transports and both sync and async
//! runtimes.

#![cfg(any(feature = "sync", feature = "async"))]

use modbus::server::MemoryStore;
use modbus::DataStore;

const UNIT_ID: u8 = 0x0A;

fn advanced_store() -> MemoryStore {
    let mut store = MemoryStore::new(0, 0, 8, 0);
    store.write_registers(0, &[0x1234]).unwrap();
    store.set_exception_status(0xAF);
    store.set_comm_event_counter(0x1234, 0x5678);
    store.set_comm_event_log(0x01, 0x02, 0x03, vec![0xAA, 0xBB]);
    store.set_server_id(vec![0x11, 0x22, 0x33]);
    store.set_fifo_queue(2, vec![0x01, 0x02, 0x03, 0x04]);
    store
}

fn assert_advanced_results(
    exception_status: u8,
    diagnostics: (u16, u16),
    counter: (u16, u16),
    log: (u16, u16, u16, Vec<u8>),
    server_id: Vec<u8>,
    mask_echo: (u16, u16, u16),
    read_write: Vec<u8>,
    fifo: (u16, Vec<u8>),
) {
    assert_eq!(exception_status, 0xAF, "exception status mismatch");
    assert_eq!(diagnostics, (0x0001, 0xAABB), "diagnostics echo mismatch");
    assert_eq!(counter, (0x1234, 0x5678), "comm event counter mismatch");
    assert_eq!(
        log,
        (0x01, 0x02, 0x03, vec![0xAA, 0xBB]),
        "comm event log mismatch"
    );
    assert_eq!(server_id, vec![0x11, 0x22, 0x33], "server ID mismatch");
    assert_eq!(mask_echo, (0, 0x00FF, 0x0001), "mask write register echo mismatch");
    assert_eq!(
        read_write,
        vec![0x00, 0x34, 0x00, 0x00],
        "read/write multiple registers mismatch"
    );
    assert_eq!(fifo, (2, vec![0x01, 0x02, 0x03, 0x04]), "FIFO queue mismatch");
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
    fn advanced_over_rtu() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle = thread::spawn(move || -> Result<(), modbus::rtu_server::RtuServerError> {
            let (mut stream, _) = listener.accept()?;
            let mut server = RtuServer::new(advanced_store());
            server.serve(&mut stream, UNIT_ID)
        });

        let stream = TcpStream::connect(addr)?;
        let mut client = Client::new(RtuTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID)?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB)?;
        let counter = client.get_comm_event_counter(UNIT_ID)?;
        let log = client.get_comm_event_log(UNIT_ID)?;
        let server_id = client.report_server_id(UNIT_ID)?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001)?;
        let read_write = client.read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0)?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    fn advanced_over_tcp() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle = thread::spawn(move || -> Result<(), modbus::tcp_server::TcpServerError> {
            let (mut stream, _) = listener.accept()?;
            let mut server = TcpServer::new(advanced_store());
            server.serve(&mut stream, UNIT_ID)
        });

        let stream = TcpStream::connect(addr)?;
        let mut client = TcpClient::new(TcpTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID)?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB)?;
        let counter = client.get_comm_event_counter(UNIT_ID)?;
        let log = client.get_comm_event_log(UNIT_ID)?;
        let server_id = client.report_server_id(UNIT_ID)?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001)?;
        let read_write = client.read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0)?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    fn advanced_over_udp() -> Result<(), Box<dyn std::error::Error>> {
        let server_socket = UdpSocket::bind("127.0.0.1:0")?;
        let server_addr = server_socket.local_addr()?;

        let server_handle = thread::spawn(move || -> Result<(), modbus::udp_server::UdpServerError> {
            let mut server = UdpServer::new(advanced_store());
            for _ in 0..8 {
                server.serve_one(&server_socket, UNIT_ID)?;
            }
            Ok(())
        });

        let client_socket = UdpSocket::bind("127.0.0.1:0")?;
        let mut client = UdpClient::new(UdpTransport::new(client_socket, server_addr));

        let exception_status = client.read_exception_status(UNIT_ID)?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB)?;
        let counter = client.get_comm_event_counter(UNIT_ID)?;
        let log = client.get_comm_event_log(UNIT_ID)?;
        let server_id = client.report_server_id(UNIT_ID)?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001)?;
        let read_write = client.read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0)?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    fn advanced_over_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let listener = TcpListener::bind("127.0.0.1:0")?;
        let addr = listener.local_addr()?;

        let server_handle = thread::spawn(
            move || -> Result<(), modbus::ascii_server::AsciiServerError> {
                let (mut stream, _) = listener.accept()?;
                let mut server = AsciiServer::new(advanced_store());
                server.serve(&mut stream, UNIT_ID)
            },
        );

        let stream = TcpStream::connect(addr)?;
        let mut client = AsciiClient::new(AsciiTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID)?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB)?;
        let counter = client.get_comm_event_counter(UNIT_ID)?;
        let log = client.get_comm_event_log(UNIT_ID)?;
        let server_id = client.report_server_id(UNIT_ID)?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001)?;
        let read_write = client.read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0)?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    async fn advanced_over_rtu() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncServer::new(advanced_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncClient::new(AsyncRtuTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID).await?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB).await?;
        let counter = client.get_comm_event_counter(UNIT_ID).await?;
        let log = client.get_comm_event_log(UNIT_ID).await?;
        let server_id = client.report_server_id(UNIT_ID).await?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001).await?;
        let read_write = client
            .read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])
            .await?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0).await?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    async fn advanced_over_tcp() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncTcpServer::new(advanced_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncTcpClient::new(AsyncTcpTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID).await?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB).await?;
        let counter = client.get_comm_event_counter(UNIT_ID).await?;
        let log = client.get_comm_event_log(UNIT_ID).await?;
        let server_id = client.report_server_id(UNIT_ID).await?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001).await?;
        let read_write = client
            .read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])
            .await?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0).await?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    async fn advanced_over_udp() -> Result<(), Box<dyn std::error::Error>> {
        let server_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
        let server_addr = server_socket.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let mut server = AsyncUdpServer::new(advanced_store());
            for _ in 0..8 {
                server.serve_one(&server_socket, UNIT_ID).await?;
            }
            Ok::<_, modbus::udp_server::UdpServerError>(())
        });

        let client_socket = tokio::net::UdpSocket::bind("127.0.0.1:0").await?;
        let mut client = AsyncUdpClient::new(AsyncUdpTransport::new(client_socket, server_addr));

        let exception_status = client.read_exception_status(UNIT_ID).await?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB).await?;
        let counter = client.get_comm_event_counter(UNIT_ID).await?;
        let log = client.get_comm_event_log(UNIT_ID).await?;
        let server_id = client.report_server_id(UNIT_ID).await?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001).await?;
        let read_write = client
            .read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])
            .await?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0).await?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

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
    async fn advanced_over_ascii() -> Result<(), Box<dyn std::error::Error>> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await?;
            let mut server = AsyncAsciiServer::new(advanced_store());
            server.serve(&mut stream, UNIT_ID).await
        });

        let stream = tokio::net::TcpStream::connect(addr).await?;
        let mut client = AsyncAsciiClient::new(AsyncAsciiTransport::new(stream));

        let exception_status = client.read_exception_status(UNIT_ID).await?;
        let diagnostics = client.diagnostics(UNIT_ID, 0x0001, 0xAABB).await?;
        let counter = client.get_comm_event_counter(UNIT_ID).await?;
        let log = client.get_comm_event_log(UNIT_ID).await?;
        let server_id = client.report_server_id(UNIT_ID).await?;
        let mask_echo = client.mask_write_register(UNIT_ID, 0, 0x00FF, 0x0001).await?;
        let read_write = client
            .read_write_multiple_registers(UNIT_ID, 0, 2, 2, &[0xDEAD, 0xBEEF])
            .await?;
        let fifo = client.read_fifo_queue(UNIT_ID, 0).await?;

        assert_advanced_results(
            exception_status,
            diagnostics,
            counter,
            log,
            server_id,
            mask_echo,
            read_write,
            fifo,
        );

        drop(client);
        server_handle.await??;
        Ok(())
    }
}
