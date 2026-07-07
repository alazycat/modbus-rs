//! Synchronous ASCII transport.
//!
//! This module is available when both the `ascii` and `sync` features are
//! enabled. The transport sends and receives complete ASCII ADU frames over an
//! underlying byte stream that implements [`std::io::Read`] and
//! [`std::io::Write`]. Frames are delimited by the leading `:` character and
//! the trailing `\r\n` sequence.
//!
//! The `timeout` argument to [`Transport::recv`] is currently unused; the
//! caller is expected to configure a read timeout on the underlying stream.

#![cfg(all(feature = "ascii", any(feature = "sync", feature = "async")))]

use std::time::Duration;

use crate::ascii::AsciiAdu;
use crate::transport::TransportError;

#[cfg(feature = "sync")]
use std::io::{self, Read, Write};

#[cfg(feature = "sync")]
use crate::transport::Transport;

/// A synchronous ASCII transport wrapping a [`Read`] + [`Write`] stream.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct AsciiTransport<T> {
    stream: T,
}

#[cfg(feature = "sync")]
impl<T> AsciiTransport<T> {
    /// Create a new ASCII transport around `stream`.
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
impl<T: Read + Write> Transport for AsciiTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).map_err(TransportError::Io)?;
        self.stream.flush().map_err(TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];
        let mut timed_out = false;

        loop {
            match self.stream.read(&mut byte) {
                Ok(0) => {
                    if frame.is_empty() {
                        return Err(TransportError::Disconnected);
                    }
                    break;
                }
                Ok(1) => {
                    if frame.is_empty() && byte[0] != AsciiAdu::START {
                        // Discard leading garbage until the start character.
                        continue;
                    }
                    frame.push(byte[0]);
                    if frame.len() >= AsciiAdu::END.len()
                        && frame.ends_with(AsciiAdu::END)
                        && AsciiAdu::decode(&frame).is_ok()
                    {
                        break;
                    }
                }
                Ok(_) => unreachable!("single-byte read returned more than one byte"),
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e)
                    if e.kind() == io::ErrorKind::WouldBlock
                        || e.kind() == io::ErrorKind::TimedOut =>
                {
                    if frame.is_empty() {
                        return Err(TransportError::Timeout);
                    }
                    timed_out = true;
                    break;
                }
                Err(e) => return Err(TransportError::Io(e)),
            }
        }

        AsciiAdu::decode(&frame).map_err(|_| {
            if timed_out {
                TransportError::Timeout
            } else {
                TransportError::Disconnected
            }
        })?;

        if buf.len() < frame.len() {
            return Err(TransportError::Disconnected);
        }
        buf[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::ascii::AsciiAdu;
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
    fn ascii_transport_roundtrip() {
        let request = AsciiAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = AsciiAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let stream = Duplex::new(encoded_response[..m].to_vec());
        let mut transport = AsciiTransport::new(stream);
        transport.send(&encoded_request[..n]).unwrap();

        let mut buf = [0u8; 64];
        let received = transport.recv(&mut buf, Duration::from_millis(10)).unwrap();
        let decoded = AsciiAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
        assert!(!transport.stream().write_buf.is_empty());
    }

    #[test]
    fn ascii_transport_skips_garbage_before_start() {
        let response = AsciiAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let mut input = b"garbage".to_vec();
        input.extend_from_slice(&encoded_response[..m]);

        let stream = Duplex::new(input);
        let mut transport = AsciiTransport::new(stream);

        let mut buf = [0u8; 64];
        let received = transport.recv(&mut buf, Duration::from_millis(10)).unwrap();
        let decoded = AsciiAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[test]
    fn ascii_transport_partial_frame_returns_timeout() {
        struct TimeoutStream;
        impl Read for TimeoutStream {
            fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::new(io::ErrorKind::TimedOut, "no data"))
            }
        }
        impl Write for TimeoutStream {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                Ok(buf.len())
            }
            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let mut transport = AsciiTransport::new(TimeoutStream);
        transport.send(b":0103").unwrap();
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(10))
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }
}

#[cfg(feature = "async")]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// An asynchronous ASCII transport wrapping an async byte stream.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncAsciiTransport<T> {
    stream: T,
}

#[cfg(feature = "async")]
impl<T> AsyncAsciiTransport<T> {
    /// Create a new async ASCII transport around `stream`.
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
impl<T: AsyncRead + AsyncWrite + Unpin + Send> AsyncTransport for AsyncAsciiTransport<T> {
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).await.map_err(TransportError::Io)?;
        self.stream.flush().await.map_err(TransportError::Io)
    }

    async fn recv(&mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];

        let read_result = tokio::time::timeout(timeout, async {
            loop {
                let n = self
                    .stream
                    .read(&mut byte)
                    .await
                    .map_err(TransportError::Io)?;
                match n {
                    0 => {
                        if frame.is_empty() {
                            return Err(TransportError::Disconnected);
                        }
                        return Ok(());
                    }
                    1 => {
                        if frame.is_empty() && byte[0] != AsciiAdu::START {
                            // Discard leading garbage until the start character.
                            continue;
                        }
                        frame.push(byte[0]);
                        if frame.len() >= AsciiAdu::MIN_FRAME_SIZE
                            && frame.ends_with(AsciiAdu::END)
                            && AsciiAdu::decode(&frame).is_ok()
                        {
                            return Ok(());
                        }
                    }
                    _ => unreachable!("single-byte read returned more than one byte"),
                }
            }
        })
        .await;

        match read_result {
            Ok(Ok(())) => {
                AsciiAdu::decode(&frame).map_err(|_| TransportError::Disconnected)?;
                if buf.len() < frame.len() {
                    return Err(TransportError::Disconnected);
                }
                buf[..frame.len()].copy_from_slice(&frame);
                Ok(frame.len())
            }
            Ok(Err(e)) => Err(e),
            Err(_) => {
                if frame.is_empty() {
                    Err(TransportError::Timeout)
                } else {
                    Err(TransportError::Disconnected)
                }
            }
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::ascii::AsciiAdu;
    use crate::transport::AsyncTransport;
    use std::time::Duration;

    #[tokio::test]
    async fn async_ascii_transport_roundtrip() {
        let request = AsciiAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = AsciiAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let (mut client, server_stream) = tokio::io::duplex(1024);
        client.write_all(&encoded_response[..m]).await.unwrap();
        client.flush().await.unwrap();

        let mut transport = AsyncAsciiTransport::new(server_stream);
        transport.send(&encoded_request[..n]).await.unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(10))
            .await
            .unwrap();
        let decoded = AsciiAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);

        let mut rx = vec![0u8; n];
        client.read_exact(&mut rx).await.unwrap();
        assert_eq!(rx, &encoded_request[..n]);
    }

    #[tokio::test]
    async fn async_ascii_transport_skips_garbage_before_start() {
        let response = AsciiAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let mut input = b"garbage".to_vec();
        input.extend_from_slice(&encoded_response[..m]);

        let (mut client, server_stream) = tokio::io::duplex(1024);
        client.write_all(&input).await.unwrap();
        client.flush().await.unwrap();

        let mut transport = AsyncAsciiTransport::new(server_stream);
        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(10))
            .await
            .unwrap();
        let decoded = AsciiAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[tokio::test]
    async fn async_ascii_transport_empty_frame_returns_timeout() {
        let (_client, server_stream) = tokio::io::duplex(1024);
        let mut transport = AsyncAsciiTransport::new(server_stream);
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(10))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }
}
