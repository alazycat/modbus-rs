//! RTU ADU adapter for the unified client facade.
//!
//! The adapter owns the transport and configuration, frames request PDUs as
//! RTU ADUs, sends them, receives the response, and returns the response PDU.

#![cfg(any(feature = "rtu", feature = "async"))]

use alloc::vec::Vec;

use crate::macros::impl_adu_adapter;

#[cfg(feature = "sync")]
impl_adu_adapter! {
    [] [],
    /// Synchronous RTU ADU adapter.
    RtuAduAdapter,
    crate::rtu::RtuAdu,
    no_transaction
}

#[cfg(feature = "async")]
impl_adu_adapter! {
    [async] [.await],
    /// Asynchronous RTU ADU adapter.
    AsyncRtuAduAdapter,
    crate::rtu::RtuAdu,
    no_transaction
}
