//! Synchronous and asynchronous Modbus clients.

#![cfg(feature = "sync")]

pub mod sync;
pub use sync::{Client, ClientConfig, ClientError};
