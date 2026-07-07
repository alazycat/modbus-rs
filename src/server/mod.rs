//! Synchronous and asynchronous Modbus server.

#![cfg(any(feature = "sync", feature = "async"))]

pub mod data_store;

mod sync;

#[cfg(feature = "async")]
pub mod r#async;

pub use data_store::{DataStore, MemoryStore};
pub use sync::Server;

#[cfg(feature = "async")]
pub use r#async::AsyncServer;
