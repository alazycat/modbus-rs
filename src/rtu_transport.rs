//! Synchronous RTU transport.
//!
//! This module is available when both the `rtu` and `sync` features are
//! enabled. The transport sends and receives complete RTU ADU frames over an
//! underlying byte stream that implements [`std::io::Read`] and
//! [`std::io::Write`]. For reliable framing the stream should have a read
//! timeout configured so that inter-frame silence is reported as a
//! [`std::io::ErrorKind::TimedOut`] or [`WouldBlock`][std::io::ErrorKind::WouldBlock].

#![cfg(all(feature = "rtu", feature = "sync"))]

use std::io::{self, Read, Write};
use std::time::Duration;

use crate::rtu::RtuAdu;
use crate::transport::{Transport, TransportError};

/// A synchronous RTU transport wrapping a [`Read`] + [`Write`] stream.
#[derive(Debug)]
pub struct RtuTransport<T> {
    stream: T,
}

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

impl<T: Read + Write> Transport for RtuTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).map_err(TransportError::Io)?;
        self.stream.flush().map_err(TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
        let mut frame = Vec::new();
        let mut byte = [0u8; 1];

        loop {
            match self.stream.read(&mut byte) {
                Ok(0) => break,
                Ok(1) => {
                    frame.push(byte[0]);
                    if frame.len() >= RtuAdu::MIN_FRAME_SIZE
                        && RtuAdu::decode(&frame).is_ok()
                    {
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

        if frame.len() < RtuAdu::MIN_FRAME_SIZE {
            return Err(TransportError::Disconnected);
        }

        let _ = RtuAdu::decode(&frame).map_err(|_| TransportError::Disconnected)?;

        if buf.len() < frame.len() {
            return Err(TransportError::Disconnected);
        }
        buf[..frame.len()].copy_from_slice(&frame);
        Ok(frame.len())
    }
}

#[cfg(test)]
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
