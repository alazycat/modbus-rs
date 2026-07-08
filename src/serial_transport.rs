//! Synchronous serial transport wrapper.
//!
//! This module is available when the `sync-serial` feature is enabled. It
//! provides a thin [`Read`] + [`Write`] wrapper around a serial port so that
//! the existing RTU and ASCII framers can be used unchanged.
//!
//! The wrapper is generic over the underlying byte stream so that tests can
//! use mock streams without requiring real hardware.

#![cfg(feature = "sync-serial")]

use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;

/// A synchronous serial transport wrapping a [`Read`] + [`Write`] stream.
#[derive(Debug)]
pub struct SerialTransport<T> {
    inner: T,
}

impl<T> SerialTransport<T> {
    /// Create a serial transport around `inner`.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Return the underlying stream.
    pub fn into_inner(self) -> T {
        self.inner
    }

    /// Return an immutable reference to the underlying stream.
    pub fn inner(&self) -> &T {
        &self.inner
    }

    /// Return a mutable reference to the underlying stream.
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.inner
    }
}

impl<T: Read> Read for SerialTransport<T> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.read(buf)
    }
}

impl<T: Write> Write for SerialTransport<T> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.flush()
    }
}

/// Open a physical serial port with common Modbus serial settings.
///
/// This configures 8 data bits, no parity, 1 stop bit, and a 100 ms read
/// timeout. Use [`SerialTransport::new`] directly if you need custom port
/// configuration.
#[cfg(feature = "sync-serial")]
pub fn open_serial_port(
    path: impl AsRef<Path>,
    baud_rate: u32,
) -> Result<SerialTransport<Box<dyn serialport::SerialPort>>, serialport::Error> {
    let port = serialport::new(path.as_ref().to_string_lossy(), baud_rate)
        .data_bits(serialport::DataBits::Eight)
        .parity(serialport::Parity::None)
        .stop_bits(serialport::StopBits::One)
        .timeout(Duration::from_millis(100))
        .open()?;
    Ok(SerialTransport::new(port))
}

#[cfg(all(test, feature = "sync-serial"))]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    struct MockStream {
        read_buf: Vec<u8>,
        read_pos: usize,
        write_buf: Vec<u8>,
    }

    impl MockStream {
        fn new(read_buf: Vec<u8>) -> Self {
            Self {
                read_buf,
                read_pos: 0,
                write_buf: Vec::new(),
            }
        }
    }

    impl Read for MockStream {
        fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
            let remaining = self.read_buf.len().saturating_sub(self.read_pos);
            if remaining == 0 {
                return Ok(0);
            }
            let n = buf.len().min(remaining);
            buf[..n].copy_from_slice(&self.read_buf[self.read_pos..self.read_pos + n]);
            self.read_pos += n;
            Ok(n)
        }
    }

    impl Write for MockStream {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            self.write_buf.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn serial_transport_delegates_read_write() {
        let stream = MockStream::new(vec![0x01, 0x02, 0x03]);
        let mut transport = SerialTransport::new(stream);

        let mut buf = [0u8; 3];
        transport.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0x01, 0x02, 0x03]);

        transport.write_all(&[0x04, 0x05]).unwrap();
        transport.flush().unwrap();
        assert_eq!(transport.inner().write_buf, vec![0x04, 0x05]);
    }

    #[test]
    fn open_serial_port_rejects_invalid_path() {
        assert!(open_serial_port("/dev/this-does-not-exist", 9600).is_err());
    }

    #[cfg(all(test, feature = "rtu"))]
    #[test]
    fn rtu_framing_over_serial_transport() {
        use crate::rtu::RtuAdu;
        use crate::rtu_transport::RtuTransport;
        use crate::transport::Transport;
        use std::time::Duration;

        let request = RtuAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = RtuAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let stream = MockStream::new(encoded_response[..m].to_vec());
        let mut transport = RtuTransport::new(SerialTransport::new(stream));
        transport.send(&encoded_request[..n]).unwrap();

        let mut buf = [0u8; 64];
        let received = transport.recv(&mut buf, Duration::from_millis(100)).unwrap();
        let decoded = RtuAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }

    #[cfg(all(test, feature = "ascii"))]
    #[test]
    fn ascii_framing_over_serial_transport() {
        use crate::ascii::AsciiAdu;
        use crate::ascii_transport::AsciiTransport;
        use crate::transport::Transport;
        use std::time::Duration;

        let request = AsciiAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = AsciiAdu::new(0x01, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let stream = MockStream::new(encoded_response[..m].to_vec());
        let mut transport = AsciiTransport::new(SerialTransport::new(stream));
        transport.send(&encoded_request[..n]).unwrap();

        let mut buf = [0u8; 64];
        let received = transport.recv(&mut buf, Duration::from_millis(100)).unwrap();
        let decoded = AsciiAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);
    }
}
