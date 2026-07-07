//! Synchronous and asynchronous Modbus clients.

#![cfg(feature = "sync")]

pub mod sync;
pub use sync::{Client, ClientConfig, ClientError};

#[cfg(feature = "ascii")]
pub use crate::ascii_client::{AsciiClient, AsciiClientConfig, AsciiClientError};

#[cfg(feature = "udp")]
pub use crate::udp_client::{UdpClient, UdpClientConfig, UdpClientError};

pub(crate) use sync::pack_bits;
