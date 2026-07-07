//! Synchronous UDP transport.
//!
//! This module is available when both the `udp` and `sync` features are
//! enabled. The transport sends and receives complete MODBUS UDP ADU frames
//! (MBAP header + PDU) as datagrams over a [`std::net::UdpSocket`].

#![cfg(all(feature = "udp", feature = "sync"))]

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::transport::{Transport, TransportError};

/// A synchronous UDP transport wrapping a [`UdpSocket`] and a remote address.
#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
    remote: SocketAddr,
}

impl UdpTransport {
    /// Create a new UDP transport around `socket` sending to `remote`.
    pub fn new(socket: UdpSocket, remote: SocketAddr) -> Self {
        Self { socket, remote }
    }

    /// Return the underlying UDP socket.
    pub fn into_inner(self) -> UdpSocket {
        self.socket
    }

    /// Return an immutable reference to the underlying UDP socket.
    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    /// Return the remote socket address.
    pub fn remote(&self) -> SocketAddr {
        self.remote
    }

    /// Set the remote socket address.
    pub fn set_remote(&mut self, remote: SocketAddr) {
        self.remote = remote;
    }
}

impl Transport for UdpTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        let n = self.socket.send_to(data, self.remote).map_err(TransportError::Io)?;
        if n != data.len() {
            return Err(TransportError::Io(io::Error::new(
                io::ErrorKind::WriteZero,
                "short UDP send",
            )));
        }
        Ok(())
    }

    fn recv(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> Result<usize, TransportError> {
        if timeout.is_zero() {
            return Err(TransportError::Timeout);
        }
        self.socket.set_read_timeout(Some(timeout)).map_err(TransportError::Io)?;
        let (n, peer) = self.socket.recv_from(buf).map_err(|e| {
            if e.kind() == io::ErrorKind::WouldBlock || e.kind() == io::ErrorKind::TimedOut {
                TransportError::Timeout
            } else {
                TransportError::Io(e)
            }
        })?;
        if peer != self.remote {
            return Err(TransportError::Disconnected);
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::Transport;
    use crate::udp::UdpAdu;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn udp_transport_roundtrip() {
        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();

        let request = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let server_handle = thread::spawn(move || {
            let mut buf = [0u8; 64];
            let (len, src) = server_socket.recv_from(&mut buf).unwrap();
            assert_eq!(&buf[..len], &encoded_request[..n]);
            server_socket.send_to(&encoded_response[..m], src).unwrap();
        });

        let mut transport = UdpTransport::new(client_socket, server_addr);
        transport.send(&encoded_request[..n]).unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(100))
            .unwrap();
        let decoded = UdpAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);

        server_handle.join().unwrap();
    }

    #[test]
    fn udp_transport_recv_times_out_when_no_response() {
        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();

        let mut transport = UdpTransport::new(client_socket, server_addr);
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(50))
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }

    #[test]
    fn udp_transport_zero_timeout_returns_timeout() {
        let server_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();

        let mut transport = UdpTransport::new(client_socket, server_addr);
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::ZERO)
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }

    #[test]
    fn udp_transport_rejects_response_from_wrong_peer() {
        let remote_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let remote_addr = remote_socket.local_addr().unwrap();
        let wrong_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let client_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let client_addr = client_socket.local_addr().unwrap();

        let mut transport = UdpTransport::new(client_socket, remote_addr);
        wrong_socket.send_to(b":0000000000", client_addr).unwrap();

        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(100))
            .unwrap_err();
        assert!(matches!(err, TransportError::Disconnected));
    }
}
