//! Synchronous and asynchronous transport traits.

#![cfg(any(feature = "sync", feature = "async"))]

use std::fmt;
use std::time::Duration;

/// Errors that can occur at the transport layer.
#[derive(Debug)]
pub enum TransportError {
    /// An underlying I/O error.
    Io(std::io::Error),
    /// The operation did not complete within the requested timeout.
    Timeout,
    /// The transport peer disconnected.
    Disconnected,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(f, "transport I/O error: {e}"),
            Self::Timeout => write!(f, "transport timeout"),
            Self::Disconnected => write!(f, "transport disconnected"),
        }
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for TransportError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// A synchronous, byte-oriented transport.
///
/// Implementations are responsible for sending and receiving complete ADU
/// frames. The `recv` call must return at least one full frame (or an error)
/// within the supplied timeout.
#[cfg(feature = "sync")]
pub trait Transport {
    /// Send a complete frame.
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;

    /// Receive a complete frame into `buf`, waiting at most `timeout`.
    fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
}

/// An asynchronous, byte-oriented transport.
///
/// Implementations are responsible for sending and receiving complete ADU
/// frames. The `recv` call must return at least one full frame (or an error)
/// within the supplied timeout.
#[cfg(feature = "async")]
pub trait AsyncTransport {
    /// Send a complete frame.
    fn send(
        &mut self,
        data: &[u8],
    ) -> impl std::future::Future<Output = Result<(), TransportError>> + Send;

    /// Receive a complete frame into `buf`, waiting at most `timeout`.
    fn recv(
        &mut self,
        buf: &mut [u8],
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<usize, TransportError>> + Send;
}
