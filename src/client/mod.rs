//! Synchronous and asynchronous Modbus clients.

#![cfg(any(feature = "sync", feature = "async"))]

use core::time::Duration;

use crate::error::{DecodeError, EncodeError};
use crate::exception::ExceptionResponse;
#[cfg(feature = "helpers")]
use crate::helpers::{Endian, WordOrder};
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
    /// Byte order used by typed register helpers.
    #[cfg(feature = "helpers")]
    pub endian: Endian,
    /// Word order used by multi-register typed helpers.
    #[cfg(feature = "helpers")]
    pub word_order: WordOrder,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            #[cfg(feature = "helpers")]
            endian: Endian::Big,
            #[cfg(feature = "helpers")]
            word_order: WordOrder::MostSignificantFirst,
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
    /// A TLS handshake or certificate error.
    #[cfg(feature = "tls")]
    Tls(String),
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
            #[cfg(feature = "tls")]
            Self::Tls(e) => write!(f, "TLS error: {e}"),
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

#[cfg(feature = "helpers")]
impl From<crate::helpers::HelpersError> for ClientError {
    fn from(e: crate::helpers::HelpersError) -> Self {
        match e {
            crate::helpers::HelpersError::InvalidLength => Self::Decode(DecodeError::InvalidLength),
            crate::helpers::HelpersError::InvalidString => Self::Decode(DecodeError::InvalidValue),
        }
    }
}

/// Validate that `response_pdu` echoes the requested function code.
///
/// If the response begins with the request function code combined with the
/// exception flag, it is decoded as an [`ExceptionResponse`] and returned as
/// [`ClientError::Exception`]. A mismatching or empty response yields
/// [`ClientError::InvalidResponse`].
pub(crate) fn validate_response_function(
    request_function: u8,
    response_pdu: &[u8],
) -> Result<(), ClientError> {
    if response_pdu.is_empty() {
        return Err(ClientError::InvalidResponse);
    }
    let response_function = response_pdu[0];
    if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
        let exc = ExceptionResponse::decode(response_pdu).map_err(ClientError::Decode)?;
        return Err(ClientError::Exception(exc));
    }
    if response_function != request_function {
        return Err(ClientError::InvalidResponse);
    }
    Ok(())
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
