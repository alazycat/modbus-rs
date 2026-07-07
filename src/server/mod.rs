//! Synchronous Modbus server.

#![cfg(feature = "sync")]

pub mod data_store;
pub mod sync;

pub use data_store::{DataStore, MemoryStore};
pub use sync::Server;
