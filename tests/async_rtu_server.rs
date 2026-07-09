#![cfg(all(feature = "async", feature = "rtu"))]

//! Integration tests for the asynchronous RTU stream server.

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use modbus::function_codes::read_coils::ReadCoilsRequest;
use modbus::rtu::RtuAdu;
use modbus::server::{DataStore, MemoryStore};
use modbus::AsyncRtuServer;

fn make_read_coils_adu(slave: u8, address: u16, quantity: u16) -> Vec<u8> {
    let req = ReadCoilsRequest::new(address, quantity).unwrap();
    let mut pdu = [0u8; 5];
    let n = req.encode(&mut pdu).unwrap();
    let mut adu = [0u8; 32];
    let m = RtuAdu::new(slave, pdu[..n].to_vec())
        .encode(&mut adu)
        .unwrap();
    adu[..m].to_vec()
}

#[tokio::test]
async fn serve_one_responds_to_matching_address() {
    let mut store = MemoryStore::new(16, 0, 0, 0);
    store.write_coils(0, &[true, false, true, true]).unwrap();

    let mut server = AsyncRtuServer::new(store);
    let request = make_read_coils_adu(0x03, 0, 8);
    let (mut client, mut server_stream) = tokio::io::duplex(1024);
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    let n = server
        .serve_one(&mut server_stream, 0x03)
        .await
        .unwrap()
        .unwrap();
    assert!(n > 0);

    let mut rx = vec![0u8; n];
    client.read_exact(&mut rx).await.unwrap();
    let response = RtuAdu::decode(&rx).unwrap();
    assert_eq!(response.address, 0x03);
    assert_eq!(response.pdu, vec![0x01, 0x01, 0b00001101]);
}

#[tokio::test]
async fn serve_one_returns_none_for_broadcast() {
    let mut store = MemoryStore::new(16, 0, 0, 0);
    store.write_coils(0, &[true, false, true, true]).unwrap();

    let mut server = AsyncRtuServer::new(store);
    let request = make_read_coils_adu(0, 0, 8);
    let (mut client, mut server_stream) = tokio::io::duplex(1024);
    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    let result = server.serve_one(&mut server_stream, 0x03).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn serve_one_returns_none_for_non_matching_address() {
    let store = MemoryStore::new(16, 0, 0, 0);
    let mut server = AsyncRtuServer::new(store);
    let request = make_read_coils_adu(0x02, 0, 8);
    let (mut client, mut server_stream) = tokio::io::duplex(1024);
    client.write_all(&request).await.unwrap();
    client.shutdown().await.unwrap();

    let result = server.serve_one(&mut server_stream, 0x03).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn serve_loops_until_eof() {
    let mut store = MemoryStore::new(16, 0, 0, 0);
    store.write_coils(0, &[true, false, true, true]).unwrap();

    let mut server = AsyncRtuServer::new(store);
    let request = make_read_coils_adu(0x07, 0, 8);
    let (mut client, mut server_stream) = tokio::io::duplex(1024);

    let server_handle = tokio::spawn(async move {
        server.serve(&mut server_stream, 0x07).await.unwrap();
    });

    client.write_all(&request).await.unwrap();
    client.flush().await.unwrap();

    let mut rx = [0u8; 512];
    let n = client.read(&mut rx).await.unwrap();
    assert!(n > 0);
    let response = RtuAdu::decode(&rx[..n]).unwrap();
    assert_eq!(response.address, 0x07);

    client.shutdown().await.unwrap();
    server_handle.await.unwrap();
}
