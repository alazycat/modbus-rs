//! Synchronous and asynchronous RTU transport.
//!
//! This module is available when the `rtu` feature and at least one of the
//! `sync` or `async` runtime features are enabled. The transport sends and
//! receives complete RTU ADU frames over an underlying byte stream. For
//! reliable framing the stream should have a read timeout configured so that
//! inter-frame silence is reported as a timeout.

#![cfg(all(feature = "rtu", any(feature = "sync", feature = "async")))]

use std::time::Duration;

use crate::rtu::RtuAdu;
use crate::transport::TransportError;

#[cfg(feature = "sync")]
use std::io::{self, Read, Write};

#[cfg(feature = "sync")]
use crate::transport::Transport;

#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// A synchronous RTU transport wrapping a [`Read`] + [`Write`] stream.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct RtuTransport<T> {
    stream: T,
}

#[cfg(feature = "sync")]
impl<T> RtuTransport<T> {
    /// Create a new RTU transport around `stream`.
    pub fn new(stream: T) -> Self {
        Self { stream }
    }

    /// Return the underlying stream.
    pub fn into_inner(self) -> T {
        self.stream
    }

    /// Return an immutable reference to the underlying stream.
    pub fn stream(&self) -> &T {
        &self.stream
    }

    /// Return a mutable reference to the underlying stream.
    pub fn stream_mut(&mut self) -> &mut T {
        &mut self.stream
    }
}

#[cfg(feature = "sync")]
impl<T: Read + Write> Transport for RtuTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).map_err(TransportError::Io)?;
        self.stream.flush().map_err(TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];
        let mut complete = false;

        loop {
            match self.stream.read(&mut byte) {
                Ok(0) => break,
                Ok(1) => {
                    frame.push(byte[0]);
                    if frame.len() > RtuAdu::MAX_FRAME_SIZE {
                        return Err(TransportError::Disconnected);
                    }
                    if frame.len() >= RtuAdu::MIN_FRAME_SIZE
                        && RtuAdu::decode(&frame).is_ok()
                    {
                        complete = true;
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    if frame.is_empty() {
                        return Err(TransportError::Timeout);
                    }
                    break;
                }
                Err(e) => return Err(TransportError::Io(e)),
            }
        }

        if !complete {
            if frame.len() < RtuAdu::MIN_FRAME_SIZE {
                return Err(TransportError::Disconnected);
            }
            RtuAdu::decode(&frame).map_err(|_| TransportError::Disconnected)?;
        }

        if buf.len() < frame.len() {
            return Err(TransportError::Disconnected);
        }
        buf[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
}

/// An asynchronous RTU transport wrapping an [`AsyncReadExt`] + [`AsyncWriteExt`] stream.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncRtuTransport<T> {
    stream: T,
}

#[cfg(feature = "async")]
impl<T> AsyncRtuTransport<T> {
    /// Create a new async RTU transport around `stream`.
    pub fn new(stream: T) -> Self {
        Self { stream }
    }

    /// Return the underlying stream.
    pub fn into_inner(self) -> T {
        self.stream
    }

    /// Return an immutable reference to the underlying stream.
    pub fn stream(&self) -> &T {
        &self.stream
    }

    /// Return a mutable reference to the underlying stream.
    pub fn stream_mut(&mut self) -> &mut T {
        &mut self.stream
    }
}

#[cfg(feature = "async")]
impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> AsyncTransport for AsyncRtuTransport<T> {
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).await.map_err(TransportError::Io)?;
        self.stream.flush().await.map_err(TransportError::Io)
    }

    async fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];
        let mut complete = false;
        // `timeout` bounds the entire receive of one full frame, per `AsyncTransport::recv`.
        let deadline = tokio::time::Instant::now() + timeout;

        loop {
            match tokio::time::timeout_at(deadline, self.stream.read(&mut byte)).await {
                Ok(Ok(0)) => {
                    if frame.is_empty() {
                        return Err(TransportError::Disconnected);
                    }
                    break;
                }
                Ok(Ok(1)) => {
                    frame.push(byte[0]);
                    if frame.len() > RtuAdu::MAX_FRAME_SIZE {
                        return Err(TransportError::Disconnected);
                    }
                    if frame.len() >= RtuAdu::MIN_FRAME_SIZE
                        && RtuAdu::decode(&frame).is_ok()
                    {
                        complete = true;
                        break;
                    }
                }
                Ok(Ok(_)) => unreachable!("single-byte read returned more than one byte"),
                Ok(Err(e)) => {
                    if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
                        if frame.is_empty() {
                            return Err(TransportError::Timeout);
                        }
                        break;
                    }
                    return Err(TransportError::Io(e));
                }
                Err(_) => {
                    if frame.is_empty() {
                        return Err(TransportError::Timeout);
                    }
                    break;
                }
            }
        }

        if !complete {
            if frame.len() < RtuAdu::MIN_FRAME_SIZE {
                return Err(TransportError::Disconnected);
            }
            RtuAdu::decode(&frame).map_err(|_| TransportError::Disconnected)?;
        }

        if buf.len() < frame.len() {
            return Err(TransportError::Disconnected);
        }
        buf[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
}

#[cfg(feature = "async")]
/// Convenience alias for an asynchronous RTU Modbus client.
pub type AsyncRtuClient<T> = crate::client::AsyncClient<AsyncRtuTransport<T>>;

#[cfg(feature = "serial")]
mod serial {
    use super::AsyncRtuTransport;
    use std::path::Path;
    use std::time::Duration;
    use tokio_serial::{DataBits, Parity, SerialStream, StopBits};

    /// Open a serial port at `path` and wrap it in an [`AsyncRtuTransport`].
    ///
    /// This is a convenience helper for the common Modbus RTU serial settings
    /// (8 data bits, no parity, 1 stop bit). Use [`AsyncRtuTransport::new`]
    /// directly if you need custom serial port configuration.
    pub async fn open_serial_rtu(
        path: impl AsRef<Path>,
        baud_rate: u32,
    ) -> Result<AsyncRtuTransport<SerialStream>, tokio_serial::Error> {
        let builder = tokio_serial::new(path.as_ref().to_string_lossy(), baud_rate)
            .data_bits(DataBits::Eight)
            .parity(Parity::None)
            .stop_bits(StopBits::One)
            .timeout(Duration::from_millis(100));
        let stream = SerialStream::open(&builder)?;
        Ok(AsyncRtuTransport::new(stream))
    }
}

#[cfg(feature = "serial")]
pub use serial::open_serial_rtu;

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::client::{Client, ClientConfig};
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::Transport;
    use core::time::Duration;

    /// A loopback transport that dispatches each RTU request through a local
    /// [`Server`]. This lets the client ↔ server integration test run without
    /// any external hardware or network socket.
    struct LoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl LoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for LoopbackTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = RtuAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response.encode(&mut adu).map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[test]
    fn read_coils_over_rtu() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server.store_mut().write_coils(0, &[true, false, true, true]).unwrap();

        let config = ClientConfig {
            timeout: Duration::from_secs(1),
        };
        let mut client = Client::with_config(LoopbackTransport::new(server), config);
        let coils = client.read_coils(0x01, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[test]
    fn write_and_read_holding_register_over_rtu() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = Client::new(LoopbackTransport::new(server));
        client.write_register(0x01, 1, 0x1234).unwrap();
        let bytes = client.read_holding_registers(0x01, 1, 1).unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[test]
    fn rtu_transport_rejects_truncated_frame() {
        struct ShortStream;
        impl Read for ShortStream {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::TimedOut, "no data"))
            }
        }
        impl Write for ShortStream {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut transport = RtuTransport::new(ShortStream);
        transport.send(&[0x01, 0x03, 0x00, 0x00, 0x00, 0x0A]).unwrap();
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(10))
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::client::AsyncClient;
    use crate::server::{AsyncServer, DataStore, MemoryStore};
    use crate::transport::AsyncTransport;
    use core::time::Duration;

    struct AsyncLoopbackTransport {
        server: crate::server::Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncLoopbackTransport {
        fn new(server: crate::server::Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncLoopbackTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = RtuAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = RtuAdu::new(request.address, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[tokio::test]
    async fn read_coils_over_async_rtu() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = crate::server::Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let mut client = AsyncClient::new(AsyncLoopbackTransport::new(server));
        let coils = client.read_coils(0x01, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn write_and_read_holding_register_over_async_rtu() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = crate::server::Server::new(store);

        let mut client = AsyncClient::new(AsyncLoopbackTransport::new(server));
        client.write_register(0x01, 1, 0x1234).await.unwrap();
        let bytes = client.read_holding_registers(0x01, 1, 1).await.unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[tokio::test]
    async fn async_rtu_transport_roundtrip() {
        let request = RtuAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = RtuAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let (client, mut server) = tokio::io::duplex(1024);
        server.write_all(&encoded_response[..m]).await.unwrap();
        server.flush().await.unwrap();

        let mut transport = AsyncRtuTransport::new(client);
        transport.send(&encoded_request[..n]).await.unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap();
        let decoded = RtuAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[tokio::test]
    async fn async_rtu_transport_roundtrip_one_byte_at_a_time() {
        use std::io;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

        struct ByteStream {
            bytes: Vec<u8>,
            pos: usize,
        }

        impl AsyncRead for ByteStream {
            fn poll_read(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                let this = self.get_mut();
                if this.pos >= this.bytes.len() {
                    return Poll::Ready(Ok(()));
                }
                buf.put_slice(&[this.bytes[this.pos]]);
                this.pos += 1;
                Poll::Ready(Ok(()))
            }
        }

        impl AsyncWrite for ByteStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Ok(buf.len()))
            }
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
            fn poll_shutdown(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
            ) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        let request = RtuAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = RtuAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let stream = ByteStream {
            bytes: encoded_response[..m].to_vec(),
            pos: 0,
        };
        let mut transport = AsyncRtuTransport::new(stream);
        transport.send(&encoded_request[..n]).await.unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap();
        let decoded = RtuAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[tokio::test]
    async fn async_rtu_transport_recv_times_out_when_no_data() {
        use std::io;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

        struct ShortStream;
        impl AsyncRead for ShortStream {
            fn poll_read(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                _buf: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                Poll::Ready(Err(io::Error::new(io::ErrorKind::TimedOut, "no data")))
            }
        }
        impl AsyncWrite for ShortStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Ok(buf.len()))
            }
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
            fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        let mut transport = AsyncRtuTransport::new(ShortStream);
        transport
            .send(&[0x01, 0x03, 0x00, 0x00, 0x00, 0x0A])
            .await
            .unwrap();
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }

    #[tokio::test]
    async fn async_rtu_transport_rejects_oversized_frame() {
        use std::io;
        use std::pin::Pin;
        use std::task::{Context, Poll};
        use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

        struct GarbageStream;
        impl AsyncRead for GarbageStream {
            fn poll_read(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &mut ReadBuf<'_>,
            ) -> Poll<io::Result<()>> {
                buf.put_slice(&[0x00]);
                Poll::Ready(Ok(()))
            }
        }
        impl AsyncWrite for GarbageStream {
            fn poll_write(
                self: Pin<&mut Self>,
                _cx: &mut Context<'_>,
                buf: &[u8],
            ) -> Poll<io::Result<usize>> {
                Poll::Ready(Ok(buf.len()))
            }
            fn poll_flush(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
            fn poll_shutdown(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<io::Result<()>> {
                Poll::Ready(Ok(()))
            }
        }

        let mut transport = AsyncRtuTransport::new(GarbageStream);
        let mut buf = [0u8; 512];
        let err = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Disconnected));
    }

    #[tokio::test]
    async fn async_rtu_server_serves_matching_address() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let mut server = AsyncServer::new(store);
        let request = {
            use crate::function_codes::read_coils::ReadCoilsRequest;
            let req = ReadCoilsRequest::new(0, 8).unwrap();
            let mut pdu = [0u8; 5];
            let n = req.encode(&mut pdu).unwrap();
            let mut adu = [0u8; 32];
            let m = RtuAdu::new(0x03, pdu[..n].to_vec())
                .encode(&mut adu)
                .unwrap();
            adu[..m].to_vec()
        };

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
}
