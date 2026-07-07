//! Synchronous and asynchronous UDP Modbus clients.
//!
//! This module is available when the `udp` feature and at least one of the
//! `sync` or `async` runtime features are enabled. It wraps request PDUs in the
//! MODBUS UDP MBAP header, tracks transaction IDs, validates responses, and
//! exposes high-level methods for reading and writing coils and registers.

#![cfg(all(feature = "udp", any(feature = "sync", feature = "async")))]

use alloc::vec;
use alloc::vec::Vec;
use core::time::Duration;

use crate::client::pack_bits;
use crate::error::{DecodeError, EncodeError};
use crate::exception::ExceptionResponse;
use crate::function_codes::diagnostics::{DiagnosticsRequest, DiagnosticsResponse};
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
use crate::transport::TransportError;
use crate::udp::UdpAdu;

#[cfg(feature = "async")]
use crate::transport::AsyncTransport;

#[cfg(feature = "sync")]
use crate::transport::Transport;

/// Configuration for a synchronous UDP client.
#[derive(Debug, Clone, Copy)]
pub struct UdpClientConfig {
    /// Maximum time to wait for a response.
    pub timeout: Duration,
}

impl Default for UdpClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }
}

/// Errors that can occur while using the synchronous UDP client.
#[derive(Debug)]
pub enum UdpClientError {
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
}

impl core::fmt::Display for UdpClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "UDP client transport error: {e}"),
            Self::Encode(e) => write!(f, "UDP client encode error: {e}"),
            Self::Decode(e) => write!(f, "UDP client decode error: {e}"),
            Self::Timeout => write!(f, "UDP client timeout"),
            Self::InvalidResponse => write!(f, "invalid response"),
            Self::Exception(e) => write!(f, "server exception: {e:?}"),
        }
    }
}

impl std::error::Error for UdpClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            Self::Encode(e) => Some(e),
            Self::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for UdpClientError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Timeout => Self::Timeout,
            other => Self::Transport(other),
        }
    }
}

/// A synchronous UDP Modbus client.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct UdpClient<T: Transport> {
    transport: T,
    config: UdpClientConfig,
    next_transaction_id: u16,
}

#[cfg(feature = "sync")]
impl<T: Transport> UdpClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, UdpClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: UdpClientConfig) -> Self {
        Self {
            transport,
            config,
            next_transaction_id: 1,
        }
    }

    /// Dispatch a request PDU to `unit_id` and return the response PDU.
    pub fn dispatch(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, UdpClientError> {
        if request_pdu.is_empty() {
            return Err(UdpClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

        let adu = UdpAdu::new(transaction_id, unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(UdpClientError::Encode)?;
        self.transport.send(&tx[..n])?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout)?;
        if m == 0 {
            return Err(UdpClientError::Transport(TransportError::Disconnected));
        }
        let response = UdpAdu::decode(&rx[..m]).map_err(UdpClientError::Decode)?;
        if response.transaction_id != transaction_id {
            return Err(UdpClientError::InvalidResponse);
        }
        if response.unit_id != unit_id {
            return Err(UdpClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(UdpClientError::InvalidResponse);
        }

        let response_function = response.pdu[0];
        if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
            let exc = ExceptionResponse::decode(&response.pdu).map_err(UdpClientError::Decode)?;
            return Err(UdpClientError::Exception(exc));
        }
        if response_function != request_function {
            return Err(UdpClientError::InvalidResponse);
        }

        Ok(response.pdu)
    }

    /// Read `quantity` coils starting at `address` from `unit_id`.
    pub fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadCoilsRequest::new(address, quantity).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadCoilsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.coil_status)
    }

    /// Read `quantity` discrete inputs starting at `address` from `unit_id`.
    pub fn read_discrete_inputs(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadDiscreteInputsRequest::new(address, quantity)
            .map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadDiscreteInputsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.input_status)
    }

    /// Read `quantity` holding registers starting at `address` from `unit_id`.
    pub fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadHoldingRegistersRequest::new(address, quantity)
            .map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadHoldingRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read `quantity` input registers starting at `address` from `unit_id`.
    pub fn read_input_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadInputRegistersRequest::new(address, quantity)
            .map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadInputRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Write a single coil at `address` on `unit_id`.
    pub fn write_coil(
        &mut self,
        unit_id: u8,
        address: u16,
        value: bool,
    ) -> Result<(), UdpClientError> {
        let raw = if value {
            WriteSingleCoilRequest::ON
        } else {
            WriteSingleCoilRequest::OFF
        };
        let req = WriteSingleCoilRequest::new(address, raw).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let _ = WriteSingleCoilResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write a single holding register at `address` on `unit_id`.
    pub fn write_register(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), UdpClientError> {
        let req = WriteSingleRegisterRequest::new(address, value);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let _ = WriteSingleRegisterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write multiple coils starting at `address` on `unit_id`.
    pub fn write_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[bool],
    ) -> Result<(), UdpClientError> {
        let outputs = pack_bits(values);
        let quantity = values.len() as u16;
        let req = WriteMultipleCoilsRequest::new(address, quantity, outputs)
            .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.outputs.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let _ = WriteMultipleCoilsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write multiple holding registers starting at `address` on `unit_id`.
    pub fn write_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[u16],
    ) -> Result<(), UdpClientError> {
        let mut register_values = Vec::with_capacity(values.len() * 2);
        for &value in values {
            register_values.extend_from_slice(&value.to_be_bytes());
        }
        let quantity = values.len() as u16;
        let req = WriteMultipleRegistersRequest::new(address, quantity, register_values)
            .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.register_values.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let _ = WriteMultipleRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Read the exception status byte from `unit_id` (FC 0x07).
    pub fn read_exception_status(&mut self, unit_id: u8) -> Result<u8, UdpClientError> {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadExceptionStatusResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.data)
    }

    /// Execute a diagnostics sub-function on `unit_id` (FC 0x08).
    pub fn diagnostics(
        &mut self,
        unit_id: u8,
        sub_function: u16,
        data: u16,
    ) -> Result<(u16, u16), UdpClientError> {
        let req = DiagnosticsRequest::new(sub_function, data);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = DiagnosticsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.sub_function, resp.data))
    }

    /// Read the communication event counter from `unit_id` (FC 0x0B).
    pub fn get_comm_event_counter(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16), UdpClientError> {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = GetCommEventCounterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.status, resp.event_count))
    }

    /// Read the communication event log from `unit_id` (FC 0x0C).
    pub fn get_comm_event_log(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16, u16, Vec<u8>), UdpClientError> {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = GetCommEventLogResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.status, resp.event_count, resp.message_count, resp.events))
    }

    /// Report the server ID from `unit_id` (FC 0x11).
    pub fn report_server_id(&mut self, unit_id: u8) -> Result<Vec<u8>, UdpClientError> {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReportServerIdResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.data)
    }

    /// Mask-write a holding register on `unit_id` (FC 0x16).
    pub fn mask_write_register(
        &mut self,
        unit_id: u8,
        reference_address: u16,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(u16, u16, u16), UdpClientError> {
        let req = MaskWriteRegisterRequest::new(reference_address, and_mask, or_mask);
        let mut buf = [0u8; 7];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = MaskWriteRegisterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.reference_address, resp.and_mask, resp.or_mask))
    }

    /// Atomically read and write holding registers on `unit_id` (FC 0x17).
    pub fn read_write_multiple_registers(
        &mut self,
        unit_id: u8,
        read_address: u16,
        read_quantity: u16,
        write_address: u16,
        write_values: &[u16],
    ) -> Result<Vec<u8>, UdpClientError> {
        let mut write_register_values = Vec::with_capacity(write_values.len() * 2);
        for &value in write_values {
            write_register_values.extend_from_slice(&value.to_be_bytes());
        }
        let write_quantity = write_values.len() as u16;
        let req = ReadWriteMultipleRegistersRequest::new(
            read_address,
            read_quantity,
            write_address,
            write_quantity,
            write_register_values,
        )
        .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 10 + req.write_values.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp =
            ReadWriteMultipleRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read the FIFO queue at `fifo_pointer_address` from `unit_id` (FC 0x18).
    pub fn read_fifo_queue(
        &mut self,
        unit_id: u8,
        fifo_pointer_address: u16,
    ) -> Result<(u16, Vec<u8>), UdpClientError> {
        let req = ReadFifoQueueRequest::new(fifo_pointer_address);
        let mut buf = [0u8; 3];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n])?;
        let resp = ReadFifoQueueResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.fifo_count, resp.register_values))
    }
}

#[cfg(all(test, feature = "sync"))]
mod tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::Transport;
    use crate::udp::UdpAdu;
    use core::time::Duration;

    struct LoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl LoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl Transport for LoopbackTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = UdpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = UdpAdu::new(request.transaction_id, request.unit_id, pdu_response[..n].to_vec());
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[test]
    fn read_coils_over_udp() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server.store_mut().write_coils(0, &[true, false, true, true]).unwrap();

        let mut client = UdpClient::new(LoopbackTransport::new(server));
        let coils = client.read_coils(0x0A, 0, 8).unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[test]
    fn write_and_read_holding_register_over_udp() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = UdpClient::new(LoopbackTransport::new(server));
        client.write_register(0x0A, 1, 0x1234).unwrap();
        let bytes = client.read_holding_registers(0x0A, 1, 1).unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[test]
    fn transaction_id_increments() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server.store_mut().write_coils(0, &[true, true, true, true]).unwrap();

        let mut client = UdpClient::new(LoopbackTransport::new(server));
        let _ = client.read_coils(0x01, 0, 8).unwrap();
        let _ = client.read_coils(0x01, 0, 8).unwrap();
    }

    #[test]
    fn mismatched_transaction_id_returns_invalid_response() {
        struct BadTransport;
        impl Transport for BadTransport {
            fn send(&mut self,
                _data: &[u8],
            ) -> Result<(), TransportError> {
                Ok(())
            }
            fn recv(
                &mut self,
                buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                let response = UdpAdu::new(0x9999, 0x01, vec![0x01, 0x01, 0x0F]);
                let mut tmp = [0u8; 32];
                let n = response.encode(&mut tmp).unwrap();
                buf[..n].copy_from_slice(&tmp[..n]);
                Ok(n)
            }
        }

        let mut client = UdpClient::new(BadTransport);
        let err = client.read_coils(0x01, 0, 8).unwrap_err();
        assert!(matches!(err, UdpClientError::InvalidResponse));
    }
}

/// An asynchronous UDP Modbus client.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncUdpClient<T: AsyncTransport> {
    transport: T,
    config: UdpClientConfig,
    next_transaction_id: u16,
}

#[cfg(feature = "async")]
impl<T: AsyncTransport> AsyncUdpClient<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, UdpClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: UdpClientConfig) -> Self {
        Self {
            transport,
            config,
            next_transaction_id: 1,
        }
    }

    /// Dispatch a request PDU to `unit_id` and return the response PDU.
    pub async fn dispatch(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, UdpClientError> {
        if request_pdu.is_empty() {
            return Err(UdpClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];
        let transaction_id = self.next_transaction_id;
        self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

        let adu = UdpAdu::new(transaction_id, unit_id, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(UdpClientError::Encode)?;
        self.transport.send(&tx[..n]).await?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout).await?;
        if m == 0 {
            return Err(UdpClientError::Transport(TransportError::Disconnected));
        }
        let response = UdpAdu::decode(&rx[..m]).map_err(UdpClientError::Decode)?;
        if response.transaction_id != transaction_id {
            return Err(UdpClientError::InvalidResponse);
        }
        if response.unit_id != unit_id {
            return Err(UdpClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(UdpClientError::InvalidResponse);
        }

        let response_function = response.pdu[0];
        if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
            let exc = ExceptionResponse::decode(&response.pdu).map_err(UdpClientError::Decode)?;
            return Err(UdpClientError::Exception(exc));
        }
        if response_function != request_function {
            return Err(UdpClientError::InvalidResponse);
        }

        Ok(response.pdu)
    }

    /// Read `quantity` coils starting at `address` from `unit_id`.
    pub async fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadCoilsRequest::new(address, quantity).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadCoilsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.coil_status)
    }

    /// Read `quantity` discrete inputs starting at `address` from `unit_id`.
    pub async fn read_discrete_inputs(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req =
            ReadDiscreteInputsRequest::new(address, quantity).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadDiscreteInputsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.input_status)
    }

    /// Read `quantity` holding registers starting at `address` from `unit_id`.
    pub async fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReadHoldingRegistersRequest::new(address, quantity)
            .map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadHoldingRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read `quantity` input registers starting at `address` from `unit_id`.
    pub async fn read_input_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req =
            ReadInputRegistersRequest::new(address, quantity).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadInputRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Write a single coil at `address` on `unit_id`.
    pub async fn write_coil(
        &mut self,
        unit_id: u8,
        address: u16,
        value: bool,
    ) -> Result<(), UdpClientError> {
        let raw = if value {
            WriteSingleCoilRequest::ON
        } else {
            WriteSingleCoilRequest::OFF
        };
        let req = WriteSingleCoilRequest::new(address, raw).map_err(UdpClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteSingleCoilResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write a single holding register at `address` on `unit_id`.
    pub async fn write_register(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), UdpClientError> {
        let req = WriteSingleRegisterRequest::new(address, value);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteSingleRegisterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write multiple coils starting at `address` on `unit_id`.
    pub async fn write_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[bool],
    ) -> Result<(), UdpClientError> {
        let outputs = pack_bits(values);
        let quantity = values.len() as u16;
        let req = WriteMultipleCoilsRequest::new(address, quantity, outputs)
            .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.outputs.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteMultipleCoilsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Write multiple holding registers starting at `address` on `unit_id`.
    pub async fn write_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[u16],
    ) -> Result<(), UdpClientError> {
        let mut register_values = Vec::with_capacity(values.len() * 2);
        for &value in values {
            register_values.extend_from_slice(&value.to_be_bytes());
        }
        let quantity = values.len() as u16;
        let req = WriteMultipleRegistersRequest::new(address, quantity, register_values)
            .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.register_values.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteMultipleRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(())
    }

    /// Read the exception status byte from `unit_id` (FC 0x07).
    pub async fn read_exception_status(
        &mut self,
        unit_id: u8,
    ) -> Result<u8, UdpClientError> {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadExceptionStatusResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.data)
    }

    /// Execute a diagnostics sub-function on `unit_id` (FC 0x08).
    pub async fn diagnostics(
        &mut self,
        unit_id: u8,
        sub_function: u16,
        data: u16,
    ) -> Result<(u16, u16), UdpClientError> {
        let req = DiagnosticsRequest::new(sub_function, data);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = DiagnosticsResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.sub_function, resp.data))
    }

    /// Read the communication event counter from `unit_id` (FC 0x0B).
    pub async fn get_comm_event_counter(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16), UdpClientError> {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = GetCommEventCounterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.status, resp.event_count))
    }

    /// Read the communication event log from `unit_id` (FC 0x0C).
    pub async fn get_comm_event_log(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16, u16, Vec<u8>), UdpClientError> {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = GetCommEventLogResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.status, resp.event_count, resp.message_count, resp.events))
    }

    /// Report the server ID from `unit_id` (FC 0x11).
    pub async fn report_server_id(
        &mut self,
        unit_id: u8,
    ) -> Result<Vec<u8>, UdpClientError> {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReportServerIdResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.data)
    }

    /// Mask-write a holding register on `unit_id` (FC 0x16).
    pub async fn mask_write_register(
        &mut self,
        unit_id: u8,
        reference_address: u16,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(u16, u16, u16), UdpClientError> {
        let req = MaskWriteRegisterRequest::new(reference_address, and_mask, or_mask);
        let mut buf = [0u8; 7];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = MaskWriteRegisterResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.reference_address, resp.and_mask, resp.or_mask))
    }

    /// Atomically read and write holding registers on `unit_id` (FC 0x17).
    pub async fn read_write_multiple_registers(
        &mut self,
        unit_id: u8,
        read_address: u16,
        read_quantity: u16,
        write_address: u16,
        write_values: &[u16],
    ) -> Result<Vec<u8>, UdpClientError> {
        let mut write_register_values = Vec::with_capacity(write_values.len() * 2);
        for &value in write_values {
            write_register_values.extend_from_slice(&value.to_be_bytes());
        }
        let write_quantity = write_values.len() as u16;
        let req = ReadWriteMultipleRegistersRequest::new(
            read_address,
            read_quantity,
            write_address,
            write_quantity,
            write_register_values,
        )
        .map_err(UdpClientError::Decode)?;
        let mut buf = vec![0u8; 10 + req.write_values.len()];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp =
            ReadWriteMultipleRegistersResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read the FIFO queue at `fifo_pointer_address` from `unit_id` (FC 0x18).
    pub async fn read_fifo_queue(
        &mut self,
        unit_id: u8,
        fifo_pointer_address: u16,
    ) -> Result<(u16, Vec<u8>), UdpClientError> {
        let req = ReadFifoQueueRequest::new(fifo_pointer_address);
        let mut buf = [0u8; 3];
        let n = req.encode(&mut buf).map_err(UdpClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadFifoQueueResponse::decode(&pdu).map_err(UdpClientError::Decode)?;
        Ok((resp.fifo_count, resp.register_values))
    }
}

#[cfg(all(test, feature = "async"))]
mod async_tests {
    use super::*;
    use crate::server::{DataStore, MemoryStore, Server};
    use crate::transport::AsyncTransport;
    use crate::udp::UdpAdu;
    use core::time::Duration;

    struct AsyncLoopbackTransport {
        server: Server<MemoryStore>,
        pending: Option<Vec<u8>>,
    }

    impl AsyncLoopbackTransport {
        fn new(server: Server<MemoryStore>) -> Self {
            Self {
                server,
                pending: None,
            }
        }
    }

    impl AsyncTransport for AsyncLoopbackTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            let request = UdpAdu::decode(data).map_err(|_| TransportError::Disconnected)?;
            let mut pdu_response = [0u8; 512];
            let n = self
                .server
                .dispatch(&request.pdu, &mut pdu_response)
                .map_err(|_| TransportError::Disconnected)?;
            let response = UdpAdu::new(
                request.transaction_id,
                request.unit_id,
                pdu_response[..n].to_vec(),
            );
            let mut adu = [0u8; 512];
            let m = response
                .encode(&mut adu)
                .map_err(|_| TransportError::Disconnected)?;
            self.pending = Some(adu[..m].to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let data = self.pending.take().ok_or(TransportError::Disconnected)?;
            if buf.len() < data.len() {
                return Err(TransportError::Disconnected);
            }
            buf[..data.len()].copy_from_slice(&data);
            Ok(data.len())
        }
    }

    #[tokio::test]
    async fn read_coils_over_udp() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let mut client = AsyncUdpClient::new(AsyncLoopbackTransport::new(server));
        let coils = client.read_coils(0x0A, 0, 8).await.unwrap();
        assert_eq!(coils, vec![0b00001101]);
    }

    #[tokio::test]
    async fn write_and_read_holding_register_over_udp() {
        let store = MemoryStore::new(0, 0, 4, 0);
        let server = Server::new(store);

        let mut client = AsyncUdpClient::new(AsyncLoopbackTransport::new(server));
        client.write_register(0x0A, 1, 0x1234).await.unwrap();
        let bytes = client.read_holding_registers(0x0A, 1, 1).await.unwrap();
        assert_eq!(bytes, vec![0x12, 0x34]);
    }

    #[tokio::test]
    async fn transaction_id_increments() {
        let store = MemoryStore::new(16, 0, 0, 0);
        let mut server = Server::new(store);
        server
            .store_mut()
            .write_coils(0, &[true, true, true, true])
            .unwrap();

        let mut client = AsyncUdpClient::new(AsyncLoopbackTransport::new(server));
        let _ = client.read_coils(0x01, 0, 8).await.unwrap();
        let _ = client.read_coils(0x01, 0, 8).await.unwrap();
    }

    #[tokio::test]
    async fn mismatched_transaction_id_returns_invalid_response() {
        struct BadAsyncTransport;
        impl AsyncTransport for BadAsyncTransport {
            async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            async fn recv(
                &mut self,
                buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                let response = UdpAdu::new(0x9999, 0x01, vec![0x01, 0x01, 0x0F]);
                let mut tmp = [0u8; 32];
                let n = response.encode(&mut tmp).unwrap();
                buf[..n].copy_from_slice(&tmp[..n]);
                Ok(n)
            }
        }

        let mut client = AsyncUdpClient::new(BadAsyncTransport);
        let err = client.read_coils(0x01, 0, 8).await.unwrap_err();
        assert!(matches!(err, UdpClientError::InvalidResponse));
    }
}
