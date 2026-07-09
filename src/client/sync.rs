//! Synchronous Modbus client core.

#![cfg(feature = "sync")]

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use super::{AduAdapter, ClientConfig, ClientError, RtuAduAdapter};
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
use crate::transport::Transport;

/// Generic synchronous Modbus client.
///
/// The client dispatches request PDUs through an [`AduAdapter`], waits for the
/// response, and performs basic response validation.
#[derive(Debug)]
pub struct ClientCore<A: AduAdapter> {
    adapter: A,
    #[cfg(feature = "helpers")]
    config: ClientConfig,
}

impl<A: AduAdapter> ClientCore<A> {
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

    impl_dispatch!([] []);

    impl_client_methods!([] []);
}

#[cfg(feature = "helpers")]
impl<A: AduAdapter> ClientCore<A> {
    impl_typed_helpers!([] []);
}

/// Synchronous RTU Modbus client.
///
/// This is a backward-compatible newtype around [`ClientCore`] paired with an
/// RTU ADU adapter.
#[derive(Debug)]
pub struct Client<T: Transport>(ClientCore<RtuAduAdapter<T>>);

impl<T: Transport> Client<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self(ClientCore::with_config(
            RtuAduAdapter::with_config(transport, config),
            config,
        ))
    }
}

impl<T: Transport> Deref for Client<T> {
    type Target = ClientCore<RtuAduAdapter<T>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Transport> DerefMut for Client<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(all(feature = "rtu", feature = "tcp"))]
impl Client<crate::rtu_transport::RtuTransport<std::net::TcpStream>> {
    /// Connect to a remote RTU-over-TCP server.
    ///
    /// This opens a plain TCP connection and wraps it with RTU framing. The
    /// resulting client is functionally identical to an RTU serial client, but
    /// the bytes travel over TCP.
    pub fn connect_rtu_over_tcp(
        addr: impl std::net::ToSocketAddrs,
        config: ClientConfig,
    ) -> Result<Self, ClientError> {
        let stream =
            std::net::TcpStream::connect(addr).map_err(crate::transport::TransportError::Io)?;
        stream.set_read_timeout(Some(config.timeout)).ok();
        let transport = crate::rtu_transport::RtuTransport::new(stream);
        Ok(Self::with_config(transport, config))
    }
}

#[cfg(all(feature = "rtu", feature = "sync-serial"))]
impl
    Client<
        crate::rtu_transport::RtuTransport<
            crate::serial_transport::SerialTransport<Box<dyn serialport::SerialPort>>,
        >,
    >
{
    /// Open a local serial port and return an RTU client.
    ///
    /// `path` is the platform-specific serial device name (e.g. `/dev/ttyUSB0`
    /// on Linux or `COM3` on Windows). The port is configured for 8 data bits,
    /// no parity, 1 stop bit, and a 100 ms read timeout.
    pub fn connect_serial_rtu(
        path: impl AsRef<std::path::Path>,
        baud_rate: u32,
        config: ClientConfig,
    ) -> Result<Self, ClientError> {
        let serial = crate::serial_transport::open_serial_port(path, baud_rate)
            .map_err(|e| ClientError::Transport(crate::transport::TransportError::Io(e.into())))?;
        let transport = crate::rtu_transport::RtuTransport::new(serial);
        Ok(Self::with_config(transport, config))
    }
}
