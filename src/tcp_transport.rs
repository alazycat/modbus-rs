//! Synchronous TCP transport.
//!
//! This module is available when both the `tcp` and `sync` features are
//! enabled. The transport sends and receives complete MODBUS TCP ADU frames
//! over an underlying byte stream that implements [`std::io::Read`] and
//! [`std::io::Write`]. The MBAP header is used to know the exact frame size
//! before reading the PDU.

#![cfg(all(feature = "tcp", feature = "sync"))]

use std::io::{self, Read, Write};
use std::time::Duration;

use crate::tcp::{TcpAdu, MODBUS_PROTOCOL_ID};
use crate::transport::{Transport, TransportError};

/// A synchronous TCP transport wrapping a [`Read`] + [`Write`] stream.
#[derive(Debug)]
pub struct TcpTransport<T> {
    stream: T,
}

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

impl<T: Read + Write> Transport for TcpTransport<T> {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        self.stream.write_all(data).map_err(TransportError::Io)?;
        self.stream.flush().map_err(TransportError::Io)
    }

    fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
        let mut header = [0u8; TcpAdu::HEADER_SIZE];
        read_all(&mut self.stream, &mut header)?;

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
        read_all(&mut self.stream, &mut buf[TcpAdu::HEADER_SIZE..frame_len])?;
        Ok(frame_len)
    }
}

fn read_all<T: Read>(stream: &mut T, buf: &mut [u8]) -> Result<(), TransportError> {
    let mut pos = 0;
    while pos < buf.len() {
        match stream.read(&mut buf[pos..]) {
            Ok(0) => return Err(TransportError::Disconnected),
            Ok(n) => pos += n,
            Err(e)
                if e.kind() == io::ErrorKind::WouldBlock
                    || e.kind() == io::ErrorKind::TimedOut =>
            {
                return Err(TransportError::Timeout);
            }
            Err(e) => return Err(TransportError::Io(e)),
        }
    }
    Ok(())
}

#[cfg(test)]
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
        let received = transport
            .recv(&mut buf, Duration::from_millis(10))
            .unwrap();
        let decoded = TcpAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
        assert!(!transport.stream().write_buf.is_empty());
    }
}
