//! Synchronous UDP transport.
//!
//! This module is available when both the `udp` and `sync` features are
//! enabled. The transport sends and receives complete MODBUS UDP ADU frames
//! (MBAP header + PDU) as datagrams over a [`std::net::UdpSocket`].

#![cfg(all(feature = "udp", any(feature = "sync", feature = "async")))]

use std::net::SocketAddr;
use std::time::Duration;

use crate::transport::TransportError;

#[cfg(feature = "sync")]
use std::io;

#[cfg(feature = "sync")]
use std::net::UdpSocket;

#[cfg(feature = "sync")]
use crate::transport::Transport;

/// A synchronous UDP transport wrapping a [`UdpSocket`] and a remote address.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct UdpTransport {
    socket: UdpSocket,
    remote: SocketAddr,
}

#[cfg(feature = "sync")]
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

#[cfg(feature = "sync")]
impl Transport for UdpTransport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        #[cfg(feature = "tracing")]
        tracing::trace!(protocol = "udp", data_len = data.len(), "transport send");
        let n = self
            .socket
            .send_to(data, self.remote)
            .map_err(TransportError::Io)?;
        if n != data.len() {
            return Err(TransportError::Io(io::Error::new(
                io::ErrorKind::WriteZero,
                "short UDP send",
            )));
        }
        Ok(())
    }

    fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        if timeout.is_zero() {
            return Err(TransportError::Timeout);
        }
        self.socket
            .set_read_timeout(Some(timeout))
            .map_err(TransportError::Io)?;
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
        #[cfg(feature = "tracing")]
        tracing::trace!(protocol = "udp", received_len = n, "transport recv");
        Ok(n)
    }
}

#[cfg(all(test, feature = "sync"))]
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
        let err = transport.recv(&mut buf, Duration::ZERO).unwrap_err();
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

#[cfg(feature = "async")]
use tokio::net::UdpSocket as TokioUdpSocket;

#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// An asynchronous UDP transport wrapping a tokio [`UdpSocket`] and a remote address.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncUdpTransport {
    socket: TokioUdpSocket,
    remote: SocketAddr,
}

#[cfg(feature = "async")]
impl AsyncUdpTransport {
    /// Create a new async UDP transport around `socket` sending to `remote`.
    pub fn new(socket: TokioUdpSocket, remote: SocketAddr) -> Self {
        Self { socket, remote }
    }

    /// Return the underlying UDP socket.
    pub fn into_inner(self) -> TokioUdpSocket {
        self.socket
    }

    /// Return an immutable reference to the underlying UDP socket.
    pub fn socket(&self) -> &TokioUdpSocket {
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

#[cfg(feature = "async")]
impl AsyncTransport for AsyncUdpTransport {
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
        #[cfg(feature = "tracing")]
        tracing::trace!(protocol = "udp", data_len = data.len(), "async transport send");
        let n = self
            .socket
            .send_to(data, self.remote)
            .await
            .map_err(TransportError::Io)?;
        if n != data.len() {
            return Err(TransportError::Io(std::io::Error::new(
                std::io::ErrorKind::WriteZero,
                "short UDP send",
            )));
        }
        Ok(())
    }

    async fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError> {
        if timeout.is_zero() {
            return Err(TransportError::Timeout);
        }

        match tokio::time::timeout(timeout, self.socket.recv_from(buf)).await {
            Ok(Ok((n, peer))) => {
                if peer != self.remote {
                    return Err(TransportError::Disconnected);
                }
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = "udp", received_len = n, "async transport recv");
                Ok(n)
            }
            Ok(Err(e)) => Err(TransportError::Io(e)),
            Err(_) => Err(TransportError::Timeout),
        }
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::transport::AsyncTransport;
    use crate::udp::UdpAdu;
    use std::time::Duration;

    #[tokio::test]
    async fn async_udp_transport_roundtrip() {
        let server_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();

        let request = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let response = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x02, 0x00, 0x0A]);

        let mut encoded_request = [0u8; 32];
        let n = request.encode(&mut encoded_request).unwrap();
        let mut encoded_response = [0u8; 32];
        let m = response.encode(&mut encoded_response).unwrap();

        let server_handle = tokio::spawn(async move {
            let mut buf = [0u8; 64];
            let (len, src) = server_socket.recv_from(&mut buf).await.unwrap();
            assert_eq!(&buf[..len], &encoded_request[..n]);
            server_socket
                .send_to(&encoded_response[..m], src)
                .await
                .unwrap();
        });

        let mut transport = AsyncUdpTransport::new(client_socket, server_addr);
        transport.send(&encoded_request[..n]).await.unwrap();

        let mut buf = [0u8; 64];
        let received = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap();
        let decoded = UdpAdu::decode(&buf[..received]).unwrap();
        assert_eq!(decoded, response);

        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn async_udp_transport_recv_times_out_when_no_response() {
        let server_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();

        let mut transport = AsyncUdpTransport::new(client_socket, server_addr);
        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(50))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }

    #[tokio::test]
    async fn async_udp_transport_zero_timeout_returns_timeout() {
        let server_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let server_addr = server_socket.local_addr().unwrap();
        let client_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();

        let mut transport = AsyncUdpTransport::new(client_socket, server_addr);
        let mut buf = [0u8; 64];
        let err = transport.recv(&mut buf, Duration::ZERO).await.unwrap_err();
        assert!(matches!(err, TransportError::Timeout));
    }

    #[tokio::test]
    async fn async_udp_transport_rejects_response_from_wrong_peer() {
        let remote_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let remote_addr = remote_socket.local_addr().unwrap();
        let wrong_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client_socket = TokioUdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client_addr = client_socket.local_addr().unwrap();

        let mut transport = AsyncUdpTransport::new(client_socket, remote_addr);
        wrong_socket
            .send_to(b":0000000000", client_addr)
            .await
            .unwrap();

        let mut buf = [0u8; 64];
        let err = transport
            .recv(&mut buf, Duration::from_millis(100))
            .await
            .unwrap_err();
        assert!(matches!(err, TransportError::Disconnected));
    }
}
