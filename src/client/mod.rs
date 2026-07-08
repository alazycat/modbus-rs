//! Synchronous and asynchronous Modbus clients.

#![cfg(any(feature = "sync", feature = "async"))]

use core::time::Duration;

use crate::error::{DecodeError, EncodeError};
use crate::exception::ExceptionResponse;
use crate::transport::TransportError;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

#[cfg(any(feature = "rtu", feature = "async"))]
pub mod rtu_adapter;

#[cfg(feature = "sync")]
pub use sync::{Client, ClientCore};

#[cfg(feature = "async")]
pub use r#async::{AsyncClient, AsyncClientCore};

#[cfg(all(feature = "sync", any(feature = "rtu", feature = "async")))]
pub use rtu_adapter::RtuAduAdapter;

#[cfg(all(feature = "async", any(feature = "rtu", feature = "async")))]
pub use rtu_adapter::AsyncRtuAduAdapter;

#[cfg(all(feature = "ascii", any(feature = "sync", feature = "async")))]
pub use crate::ascii_client::{AsciiClientConfig, AsciiClientError};

#[cfg(all(feature = "ascii", feature = "sync"))]
pub use crate::ascii_client::AsciiClient;

#[cfg(all(feature = "ascii", feature = "async"))]
pub use crate::ascii_client::AsyncAsciiClient;

#[cfg(all(feature = "udp", any(feature = "sync", feature = "async")))]
pub use crate::udp_client::{UdpClientConfig, UdpClientError};

#[cfg(all(feature = "udp", feature = "sync"))]
pub use crate::udp_client::UdpClient;

#[cfg(all(feature = "udp", feature = "async"))]
pub use crate::udp_client::AsyncUdpClient;

/// Configuration shared between synchronous and asynchronous clients.
#[derive(Debug, Clone, Copy)]
pub struct ClientConfig {
    /// Maximum time to wait for a response.
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }
}

/// Errors that can occur while using a Modbus client.
#[derive(Debug)]
pub enum ClientError {
    /// Transport-level failure.
    Transport(TransportError),
    /// Failed to encode the request.
    Encode(EncodeError),
    /// Failed to decode the response.
    Decode(DecodeError),
    /// No response was received within the configured timeout.
    Timeout,
    /// The response was malformed or did not match the request.
    InvalidResponse,
    /// The server returned an exception response.
    Exception(ExceptionResponse),
}

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "client transport error: {e}"),
            Self::Encode(e) => write!(f, "client encode error: {e:?}"),
            Self::Decode(e) => write!(f, "client decode error: {e:?}"),
            Self::Timeout => write!(f, "client timeout"),
            Self::InvalidResponse => write!(f, "invalid response"),
            Self::Exception(e) => write!(f, "server exception: {e:?}"),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Timeout => Self::Timeout,
            other => Self::Transport(other),
        }
    }
}

/// Seam for synchronous ADU framing and I/O.
#[cfg(feature = "sync")]
pub trait AduAdapter {
    /// Send `request_pdu` to `unit_id` and return the response PDU.
    fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError>;
}

/// Seam for asynchronous ADU framing and I/O.
#[cfg(feature = "async")]
#[allow(async_fn_in_trait)]
pub trait AsyncAduAdapter {
    /// Send `request_pdu` to `unit_id` and return the response PDU.
    async fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError>;
}

pub(crate) fn pack_bits(bits: &[bool]) -> alloc::vec::Vec<u8> {
    let mut bytes = alloc::vec::Vec::with_capacity(bits.len().div_ceil(8));
    for (i, &bit) in bits.iter().enumerate() {
        if i % 8 == 0 {
            bytes.push(0);
        }
        if bit {
            let last = bytes.last_mut().expect("byte was just pushed");
            *last |= 1 << (i % 8);
        }
    }
    bytes
}
