//! Asynchronous Modbus client core.

#![cfg(feature = "async")]

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use super::{AsyncAduAdapter, AsyncRtuAduAdapter, ClientConfig, ClientError};
use crate::exception::ExceptionResponse;
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
use crate::function_codes::read_file_record::{
    ReadFileRecordRequest, ReadFileRecordResponse, ReadFileRecordSubRequest,
    ReadFileRecordSubResponse,
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
use crate::transport::AsyncTransport;

/// Generic asynchronous Modbus client.
///
/// The client dispatches request PDUs through an [`AsyncAduAdapter`], waits for
/// the response, and performs basic response validation.
#[derive(Debug)]
pub struct AsyncClientCore<A: AsyncAduAdapter> {
    adapter: A,
}

impl<A: AsyncAduAdapter> AsyncClientCore<A> {
    /// Create a client around an adapter.
    pub fn new(adapter: A) -> Self {
        Self { adapter }
    }

    /// Dispatch a request PDU to `unit_id` and return the response PDU.
    ///
    /// The request PDU must begin with the function code. The returned response
    /// PDU also begins with the function code, unless the server replied with
    /// an exception.
    pub async fn dispatch(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
        if request_pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];

        let response_pdu = self.adapter.send_receive(unit_id, request_pdu).await?;
        if response_pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }

        let response_function = response_pdu[0];
        if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
            let exc = ExceptionResponse::decode(&response_pdu).map_err(ClientError::Decode)?;
            return Err(ClientError::Exception(exc));
        }
        if response_function != request_function {
            return Err(ClientError::InvalidResponse);
        }

        Ok(response_pdu)
    }

    /// Read `quantity` coils starting at `address` from `unit_id`.
    pub async fn read_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadCoilsRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadCoilsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.coil_status)
    }

    /// Read `quantity` discrete inputs starting at `address` from `unit_id`.
    pub async fn read_discrete_inputs(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadDiscreteInputsRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadDiscreteInputsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.input_status)
    }

    /// Read `quantity` holding registers starting at `address` from `unit_id`.
    pub async fn read_holding_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req =
            ReadHoldingRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadHoldingRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read `quantity` input registers starting at `address` from `unit_id`.
    pub async fn read_input_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadInputRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadInputRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Write a single coil at `address` on `unit_id`.
    pub async fn write_coil(
        &mut self,
        unit_id: u8,
        address: u16,
        value: bool,
    ) -> Result<(), ClientError> {
        let raw = if value {
            WriteSingleCoilRequest::ON
        } else {
            WriteSingleCoilRequest::OFF
        };
        let req = WriteSingleCoilRequest::new(address, raw).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteSingleCoilResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write a single holding register at `address` on `unit_id`.
    pub async fn write_register(
        &mut self,
        unit_id: u8,
        address: u16,
        value: u16,
    ) -> Result<(), ClientError> {
        let req = WriteSingleRegisterRequest::new(address, value);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteSingleRegisterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write multiple coils starting at `address` on `unit_id`.
    pub async fn write_coils(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[bool],
    ) -> Result<(), ClientError> {
        let outputs = super::pack_bits(values);
        let quantity = values.len() as u16;
        let req = WriteMultipleCoilsRequest::new(address, quantity, outputs)
            .map_err(ClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.outputs.len()];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteMultipleCoilsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write multiple holding registers starting at `address` on `unit_id`.
    pub async fn write_registers(
        &mut self,
        unit_id: u8,
        address: u16,
        values: &[u16],
    ) -> Result<(), ClientError> {
        let mut register_values = Vec::with_capacity(values.len() * 2);
        for &value in values {
            register_values.extend_from_slice(&value.to_be_bytes());
        }
        let quantity = values.len() as u16;
        let req = WriteMultipleRegistersRequest::new(address, quantity, register_values)
            .map_err(ClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.register_values.len()];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let _ = WriteMultipleRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Read the exception status byte from `unit_id` (FC 0x07).
    pub async fn read_exception_status(&mut self, unit_id: u8) -> Result<u8, ClientError> {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadExceptionStatusResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.data)
    }

    /// Execute a diagnostics sub-function on `unit_id` (FC 0x08).
    ///
    /// Returns the echoed `(sub_function, data)` pair from the response.
    pub async fn diagnostics(
        &mut self,
        unit_id: u8,
        sub_function: u16,
        data: u16,
    ) -> Result<(u16, u16), ClientError> {
        let req = DiagnosticsRequest::new(sub_function, data);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = DiagnosticsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.sub_function, resp.data))
    }

    /// Read the communication event counter from `unit_id` (FC 0x0B).
    pub async fn get_comm_event_counter(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16), ClientError> {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = GetCommEventCounterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.status, resp.event_count))
    }

    /// Read the communication event log from `unit_id` (FC 0x0C).
    pub async fn get_comm_event_log(
        &mut self,
        unit_id: u8,
    ) -> Result<(u16, u16, u16, Vec<u8>), ClientError> {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = GetCommEventLogResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.status, resp.event_count, resp.message_count, resp.events))
    }

    /// Report the server ID from `unit_id` (FC 0x11).
    pub async fn report_server_id(&mut self, unit_id: u8) -> Result<Vec<u8>, ClientError> {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReportServerIdResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.data)
    }

    /// Mask-write a holding register on `unit_id` (FC 0x16).
    ///
    /// Returns the `(reference_address, and_mask, or_mask)` echoed by the server.
    pub async fn mask_write_register(
        &mut self,
        unit_id: u8,
        reference_address: u16,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(u16, u16, u16), ClientError> {
        let req = MaskWriteRegisterRequest::new(reference_address, and_mask, or_mask);
        let mut buf = [0u8; 7];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = MaskWriteRegisterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.reference_address, resp.and_mask, resp.or_mask))
    }

    /// Atomically read and write holding registers on `unit_id` (FC 0x17).
    ///
    /// `write_values` are written starting at `write_address`; the returned
    /// bytes contain `read_quantity` registers starting at `read_address`.
    pub async fn read_write_multiple_registers(
        &mut self,
        unit_id: u8,
        read_address: u16,
        read_quantity: u16,
        write_address: u16,
        write_values: &[u16],
    ) -> Result<Vec<u8>, ClientError> {
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
        .map_err(ClientError::Decode)?;
        let mut buf = vec![0u8; 10 + req.write_values.len()];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadWriteMultipleRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read the FIFO queue at `fifo_pointer_address` from `unit_id` (FC 0x18).
    pub async fn read_fifo_queue(
        &mut self,
        unit_id: u8,
        fifo_pointer_address: u16,
    ) -> Result<(u16, Vec<u8>), ClientError> {
        let req = ReadFifoQueueRequest::new(fifo_pointer_address);
        let mut buf = [0u8; 3];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadFifoQueueResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.fifo_count, resp.register_values))
    }

    /// Read file records from `unit_id` (FC 0x14).
    pub async fn read_file_record(
        &mut self,
        unit_id: u8,
        sub_requests: &[ReadFileRecordSubRequest],
    ) -> Result<Vec<ReadFileRecordSubResponse>, ClientError> {
        let req = ReadFileRecordRequest::new(sub_requests.to_vec());
        let mut buf = vec![0u8; 2 + sub_requests.len() * 7];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = ReadFileRecordResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.sub_responses)
    }

    /// Write file records to `unit_id` (FC 0x15).
    pub async fn write_file_record(
        &mut self,
        unit_id: u8,
        sub_requests: &[WriteFileRecordSubRequest],
    ) -> Result<Vec<WriteFileRecordSubResponse>, ClientError> {
        let byte_count: usize = sub_requests.iter().map(|s| 7 + s.record_data.len()).sum();
        let req = WriteFileRecordRequest::new(sub_requests.to_vec());
        let mut buf = vec![0u8; 2 + byte_count];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = WriteFileRecordResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.sub_responses)
    }

    /// Send an encapsulated interface transport request to `unit_id` (FC 0x2B).
    pub async fn encapsulated_interface_transport(
        &mut self,
        unit_id: u8,
        mei_type: u8,
        data: &[u8],
    ) -> Result<(u8, Vec<u8>), ClientError> {
        let req = EncapsulatedInterfaceTransportRequest::new(mei_type, data.to_vec());
        let mut buf = vec![0u8; 2 + data.len()];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(unit_id, &buf[..n]).await?;
        let resp = EncapsulatedInterfaceTransportResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.mei_type, resp.data))
    }
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
        Self(AsyncClientCore::new(AsyncRtuAduAdapter::with_config(transport, config)))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
    use crate::rtu::RtuAdu;
    use crate::transport::TransportError;
    use alloc::collections::VecDeque;
    use core::time::Duration;

    struct MockTransport {
        sent: Vec<Vec<u8>>,
        responses: VecDeque<Vec<u8>>,
    }

    impl MockTransport {
        fn new(responses: Vec<Vec<u8>>) -> Self {
            Self {
                sent: Vec::new(),
                responses: responses.into(),
            }
        }
    }

    impl AsyncTransport for MockTransport {
        async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }

        async fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let resp = self
                .responses
                .pop_front()
                .ok_or(TransportError::Disconnected)?;
            let n = resp.len().min(buf.len());
            buf[..n].copy_from_slice(&resp[..n]);
            Ok(n)
        }
    }

    #[tokio::test]
    async fn dispatch_read_coils_roundtrip() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_pdu = {
            let resp = ReadCoilsResponse {
                coil_status: vec![0b11001011, 0b00000010],
            };
            let mut buf = [0u8; 4];
            let n = resp.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = RtuAdu::new(0x01, response_pdu.clone());
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let pdu = client.dispatch(0x01, &request_pdu).await.unwrap();
        assert_eq!(pdu, response_pdu);

        let decoded = ReadCoilsResponse::decode(&pdu).unwrap();
        assert_eq!(decoded.coil_status, vec![0b11001011, 0b00000010]);
    }

    #[tokio::test]
    async fn dispatch_returns_exception() {
        let request_pdu = {
            let req = ReadCoilsRequest::new(0x0000, 10).unwrap();
            let mut buf = [0u8; 5];
            let n = req.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let exception_pdu = {
            let exc =
                ExceptionResponse::new(0x01, crate::exception::ExceptionCode::IllegalDataAddress);
            let mut buf = [0u8; 2];
            let n = exc.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };
        let response_adu = {
            let adu = RtuAdu::new(0x01, exception_pdu);
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let err = client.dispatch(0x01, &request_pdu).await.unwrap_err();
        assert!(matches!(err, ClientError::Exception(_)));
    }

    #[tokio::test]
    async fn dispatch_propagates_timeout() {
        struct TimeoutTransport;
        impl AsyncTransport for TimeoutTransport {
            async fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            async fn recv(
                &mut self,
                _buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                Err(TransportError::Timeout)
            }
        }

        let mut client = AsyncClient::new(TimeoutTransport);
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x0A])
            .await
            .unwrap_err();
        assert!(matches!(err, ClientError::Timeout));
    }

    #[tokio::test]
    async fn dispatch_rejects_wrong_slave() {
        let response_adu = {
            let pdu = {
                let resp = ReadCoilsResponse {
                    coil_status: vec![0x01],
                };
                let mut buf = [0u8; 3];
                let n = resp.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };
            let adu = RtuAdu::new(0x02, pdu);
            let mut buf = [0u8; 512];
            let n = adu.encode(&mut buf).unwrap();
            buf[..n].to_vec()
        };

        let mut client = AsyncClient::new(MockTransport::new(vec![response_adu]));
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01])
            .await
            .unwrap_err();
        assert!(matches!(err, ClientError::InvalidResponse));
    }
}
