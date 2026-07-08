//! RTU ADU adapter for the unified client facade.
//!
//! The adapter owns the transport and configuration, frames request PDUs as
//! RTU ADUs, sends them, receives the response, and returns the response PDU.

#![cfg(any(feature = "rtu", feature = "async"))]

use alloc::vec::Vec;

use crate::rtu::RtuAdu;
use crate::transport::TransportError;

use super::{ClientConfig, ClientError};

#[cfg(feature = "sync")]
use super::AduAdapter;
#[cfg(feature = "sync")]
use crate::transport::Transport;

#[cfg(feature = "async")]
use super::AsyncAduAdapter;
#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

/// Synchronous RTU ADU adapter.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct RtuAduAdapter<T: Transport> {
    transport: T,
    config: ClientConfig,
}

#[cfg(feature = "sync")]
impl<T: Transport> RtuAduAdapter<T> {
    /// Create an adapter with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create an adapter with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self { transport, config }
    }
}

#[cfg(feature = "sync")]
impl<T: Transport> AduAdapter for RtuAduAdapter<T> {
    fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
        let adu = RtuAdu::new(unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(ClientError::Encode)?;
        self.transport.send(&tx[..n])?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout)?;
        if m == 0 {
            return Err(ClientError::Transport(TransportError::Disconnected));
        }
        let response = RtuAdu::decode(&rx[..m]).map_err(ClientError::Decode)?;
        if response.address != unit_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response.pdu)
    }
}

/// Asynchronous RTU ADU adapter.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncRtuAduAdapter<T: AsyncTransport> {
    transport: T,
    config: ClientConfig,
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncRtuAduAdapter<T> {
    /// Create an adapter with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create an adapter with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self { transport, config }
    }
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncAduAdapter for AsyncRtuAduAdapter<T> {
    async fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
        let adu = RtuAdu::new(unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(ClientError::Encode)?;
        self.transport.send(&tx[..n]).await?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout).await?;
        if m == 0 {
            return Err(ClientError::Transport(TransportError::Disconnected));
        }
        let response = RtuAdu::decode(&rx[..m]).map_err(ClientError::Decode)?;
        if response.address != unit_id {
            return Err(ClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        Ok(response.pdu)
    }
}
