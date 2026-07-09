//! Synchronous and asynchronous Modbus server.

#![cfg(any(feature = "sync", feature = "async"))]

pub mod data_store;
pub mod hook;

mod sync;

#[cfg(feature = "async")]
pub mod r#async;

pub use data_store::{DataStore, MemoryStore, SharedStore};
pub use hook::{NoopHook, RequestHook};
pub use sync::Server;

#[cfg(feature = "async")]
pub use r#async::AsyncServer;
