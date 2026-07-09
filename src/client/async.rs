//! Asynchronous Modbus client core.

#![cfg(feature = "async")]

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use super::{AsyncAduAdapter, AsyncRtuAduAdapter, ClientConfig, ClientError};
use crate::function_codes::diagnostics::{DiagnosticsRequest, DiagnosticsResponse};
use crate::function_codes::encapsulated_interface_transport::{
    EncapsulatedInterfaceTransportRequest, EncapsulatedInterfaceTransportResponse,
};
use crate::function_codes::get_comm_event_counter::{
    GetCommEventCounterRequest, GetCommEventCounterResponse,
};
use crate::function_codes::get_comm_event_log::{GetCommEventLogRequest, GetCommEventLogResponse};
use crate::function_codes::mask_write_register::{
    MaskWriteRegisterRequest, MaskWriteRegisterResponse,
};
use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
use crate::function_codes::read_discrete_inputs::{
    ReadDiscreteInputsRequest, ReadDiscreteInputsResponse,
};
use crate::function_codes::read_exception_status::{
    ReadExceptionStatusRequest, ReadExceptionStatusResponse,
};
use crate::function_codes::read_fifo_queue::{ReadFifoQueueRequest, ReadFifoQueueResponse};
use crate::function_codes::read_file_record::{
    ReadFileRecordRequest, ReadFileRecordResponse, ReadFileRecordSubRequest,
    ReadFileRecordSubResponse,
};
use crate::function_codes::read_holding_registers::{
    ReadHoldingRegistersRequest, ReadHoldingRegistersResponse,
};
use crate::function_codes::read_input_registers::{
    ReadInputRegistersRequest, ReadInputRegistersResponse,
};
use crate::function_codes::read_write_multiple_registers::{
    ReadWriteMultipleRegistersRequest, ReadWriteMultipleRegistersResponse,
};
use crate::function_codes::report_server_id::{ReportServerIdRequest, ReportServerIdResponse};
use crate::function_codes::write_file_record::{
    WriteFileRecordRequest, WriteFileRecordResponse, WriteFileRecordSubRequest,
    WriteFileRecordSubResponse,
};
use crate::function_codes::write_multiple_coils::{
    WriteMultipleCoilsRequest, WriteMultipleCoilsResponse,
};
use crate::function_codes::write_multiple_registers::{
    WriteMultipleRegistersRequest, WriteMultipleRegistersResponse,
};
use crate::function_codes::write_single_coil::{WriteSingleCoilRequest, WriteSingleCoilResponse};
use crate::function_codes::write_single_register::{
    WriteSingleRegisterRequest, WriteSingleRegisterResponse,
};
#[cfg(feature = "helpers")]
use crate::helpers;
use crate::transport::AsyncTransport;

/// Generic asynchronous Modbus client.
///
/// The client dispatches request PDUs through an [`AsyncAduAdapter`], waits for
/// the response, and performs basic response validation.
#[derive(Debug)]
pub struct AsyncClientCore<A: AsyncAduAdapter> {
    adapter: A,
    #[cfg(feature = "helpers")]
    config: ClientConfig,
}

impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    /// Create a client around an adapter with the default configuration.
    pub fn new(adapter: A) -> Self {
        Self::with_config(adapter, ClientConfig::default())
    }

    /// Create a client around an adapter with a custom configuration.
    pub fn with_config(adapter: A, config: ClientConfig) -> Self {
        #[cfg(not(feature = "helpers"))]
        let _ = config;
        Self {
            adapter,
            #[cfg(feature = "helpers")]
            config,
        }
    }

    impl_dispatch!([async] [.await]);

    impl_client_methods!([async] [.await]);
}

#[cfg(feature = "helpers")]
impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    impl_typed_helpers!([async] [.await]);
}

/// Asynchronous RTU Modbus client.
///
/// This is a backward-compatible newtype around [`AsyncClientCore`] paired with
/// an asynchronous RTU ADU adapter.
#[derive(Debug)]
pub struct AsyncClient<T: AsyncTransport>(AsyncClientCore<AsyncRtuAduAdapter<T>>);

impl<T: AsyncTransport> AsyncClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self(AsyncClientCore::with_config(
            AsyncRtuAduAdapter::with_config(transport, config),
            config,
        ))
    }
}

impl<T: AsyncTransport> Deref for AsyncClient<T> {
    type Target = AsyncClientCore<AsyncRtuAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: AsyncTransport> DerefMut for AsyncClient<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(feature = "rtu", feature = "tcp"))]
impl AsyncClient<crate::rtu_transport::AsyncRtuTransport<tokio::net::TcpStream>> {
    /// Connect to a remote RTU-over-TCP server asynchronously.
    ///
    /// This opens a plain TCP connection and wraps it with RTU framing. The
    /// resulting client is functionally identical to an RTU serial client, but
    /// the bytes travel over TCP.
    pub async fn connect_rtu_over_tcp(
        addr: impl tokio::net::ToSocketAddrs,
        config: ClientConfig,
    ) -> Result<Self, ClientError> {
        let stream = tokio::net::TcpStream::connect(addr)
            .await
            .map_err(crate::transport::TransportError::Io)?;
        let transport = crate::rtu_transport::AsyncRtuTransport::new(stream)
            .with_idle_timeout(config.idle_timeout);
        Ok(Self::with_config(transport, config))
    }
}
