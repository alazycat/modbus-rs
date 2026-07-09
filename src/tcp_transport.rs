//! Synchronous and asynchronous TCP transport.
//!
//! This module is available when the `tcp` feature and at least one of the
//! `sync` or `async` runtime features are enabled. The transport sends and
//! receives complete MODBUS TCP ADU frames over an underlying byte stream.
//! The MBAP header is used to know the exact frame size before reading the PDU.

#![cfg(all(feature = "tcp", any(feature = "sync", feature = "async")))]

use std::time::Duration;

use crate::tcp::{tcp_frame_len, TcpAdu};
use crate::transport::TransportError;

#[cfg(feature = "sync")]
use std::io::{self, Read, Write};

#[cfg(feature = "sync")]
use crate::transport::Transport;

#[cfg(feature = "async")]
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// A synchronous TCP transport wrapping a [`Read`] + [`Write`] stream.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct TcpTransport<T> {
    stream: T,
}

#[cfg(feature = "sync")]
impl<T> TcpTransport<T> {
    /// Create a new TCP transport around `stream`.
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
impl<T: Read + Write> Transport for TcpTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).map_err(TransportError::Io)?;
        self.stream.flush().map_err(TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
        let mut header = [0u8; TcpAdu::HEADER_SIZE];
        read_all(&mut self.stream, &mut header)?;

        let frame_len = tcp_frame_len(&header)?;
        if buf.len() < frame_len {
            return Err(TransportError::Disconnected);
        }

        buf[..TcpAdu::HEADER_SIZE].copy_from_slice(&header);
        read_all(&mut self.stream, &mut buf[TcpAdu::HEADER_SIZE..frame_len])?;
        Ok(frame_len)
    }
}

#[cfg(feature = "sync")]
fn read_all<T: Read>(stream: &mut T, buf: &mut [u8]) -> Result<(), TransportError> {
    let mut pos = 0;
    while pos < buf.len() {
        match stream.read(&mut buf[pos..]) {
            Ok(0) => return Err(TransportError::Disconnected),
            Ok(n) => pos += n,
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut =>
            {
                return Err(TransportError::Timeout);
            }
            Err(e) => return Err(TransportError::Io(e)),
        }
    }
    Ok(())
}

/// An asynchronous TCP transport wrapping an [`AsyncReadExt`] + [`AsyncWriteExt`] stream.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncTcpTransport<T> {
    stream: T,
}

#[cfg(feature = "async")]
impl<T> AsyncTcpTransport<T> {
    /// Create a new async TCP transport around `stream`.
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
impl<T: AsyncReadExt + AsyncWriteExt + Unpin + Send> AsyncTransport for AsyncTcpTransport<T> {
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream
            .write_all(data)
            .await
            .map_err(TransportError::Io)?;
        self.stream.flush().await.map_err(TransportError::Io)
    }

    async fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        let mut header = [0u8; TcpAdu::HEADER_SIZE];
        read_all_async(&mut self.stream, &mut header, timeout).await?;

        if u16::from_be_bytes([header[2], header[3]]) != MODBUS_PROTOCOL_ID {
            return Err(TransportError::Disconnected);
        }

        let length = u16::from_be_bytes([header[4], header[5]]) as usize;
        if length == 0 {
            return Err(TransportError::Disconnected);
        }
        let pdu_len = length - 1;
        let frame_len = TcpAdu::HEADER_SIZE + pdu_len;

        if buf.len() < frame_len {
            return Err(TransportError::Disconnected);
        }

        buf[..TcpAdu::HEADER_SIZE].copy_from_slice(&header);
        read_all_async(
            &mut self.stream,
            &mut buf[TcpAdu::HEADER_SIZE..frame_len],
            timeout,
        )
        .await?;
        Ok(frame_len)
    }
}

#[cfg(feature = "async")]
async fn read_all_async<T: AsyncReadExt + Unpin>(
    stream: &mut T,
    buf: &mut [u8],
    timeout: Duration,
) -> Result<(), TransportError> {
    let mut pos = 0;
    while pos < buf.len() {
        match tokio::time::timeout(timeout, stream.read(&mut buf[pos..])).await {
            Ok(Ok(0)) => return Err(TransportError::Disconnected),
            Ok(Ok(n)) => pos += n,
            Ok(Err(e)) => return Err(TransportError::Io(e)),
            Err(_) => return Err(TransportError::Timeout),
        }
    }
    Ok(())
}

/// An asynchronous TLS transport wrapping a [`tokio_rustls::client::TlsStream`].
///
/// Because TLS only changes the byte stream, this is a type alias around the
/// TCP framer. It is available when the `tls` feature is enabled.
#[cfg(feature = "tls")]
pub type AsyncTlsTransport<T = tokio::net::TcpStream> =
    AsyncTcpTransport<tokio_rustls::client::TlsStream<T>>;

/// A synchronous TLS transport wrapping a [`rustls::StreamOwned`].
///
/// Because TLS only changes the byte stream, this is a type alias around the
/// TCP framer. It is available when both the `tls` and `sync` features are
/// enabled.
#[cfg(all(feature = "tls", feature = "sync"))]
pub type TlsTransport<T = std::net::TcpStream> =
    TcpTransport<rustls::StreamOwned<rustls::ClientConnection, T>>;

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::tcp::TcpAdu;
    use crate::transport::Transport;
    use std::io::{Read, Write};
    use std::time::Duration;

    struct Duplex {
        read_buf: Vec<u8>,
        read_pos: usize,
        write_buf: Vec<u8>,
    }

    impl Duplex {
        fn new(read_buf: Vec<u8>) -> Self {
            Self {
                read_buf,
                read_pos: 0,
                write_buf: Vec::new(),
            }
        }
    }

    impl Read for Duplex {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let remaining = self.read_buf.len() - self.read_pos;
            if remaining == 0 {
                return Ok(0);
            }
            let n = buf.len().min(remaining);
            buf[..n].copy_from_slice(&self.read_buf[self.read_pos..self.read_pos + n]);
            self.read_pos += n;
            Ok(n)
        }
    }

    impl Write for Duplex {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.write_buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn tcp_transport_roundtrip() {
        let request = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let stream = Duplex::new(encoded_response[..m].to_vec());
        let mut transport = TcpTransport::new(stream);
        transport.send(&encoded_request[..n]).unwrap();

        let mut buf = [0u8; 64];
        let received = transport.recv(&mut buf, Duration::from_millis(10)).unwrap();
        let decoded = TcpAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
        assert!(!transport.stream().write_buf.is_empty());
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::tcp::TcpAdu;
    use crate::transport::AsyncTransport;
    use std::time::Duration;

    #[tokio::test]
    async fn async_tcp_transport_roundtrip() {
        let request = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let (client, mut server) = tokio::io::duplex(1024);
        server.write_all(&encoded_response[..m]).await.unwrap();
        server.flush().await.unwrap();

        let mut transport = AsyncTcpTransport::new(client);
        transport.send(&encoded_request[..n]).await.unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap();
        let decoded = TcpAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[tokio::test]
    async fn async_tcp_transport_recv_times_out_when_no_response() {
        let (client, _server) = tokio::io::duplex(1024);
        let mut transport = AsyncTcpTransport::new(client);
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }
}
