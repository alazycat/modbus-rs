//! Synchronous Modbus client core.

#![cfg(feature = "sync")]

use alloc::vec;
use alloc::vec::Vec;

use super::{pack_bits, ClientConfig, ClientError};
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
use crate::rtu::RtuAdu;
use crate::transport::{Transport, TransportError};

/// A synchronous Modbus client.
///
/// The client dispatches request PDUs over a [`Transport`], waits for the
/// response, and performs basic response validation. This implementation wraps
/// PDUs in RTU ADUs.
pub struct Client<T: Transport> {
    transport: T,
    config: ClientConfig,
}

impl<T: Transport> Client<T> {
    /// Create a client with the default configuration.
    pub fn new(transport: T) -> Self {
        Self::with_config(transport, ClientConfig::default())
    }

    /// Create a client with a custom configuration.
    pub fn with_config(transport: T, config: ClientConfig) -> Self {
        Self { transport, config }
    }

    /// Dispatch a request PDU to `slave` and return the response PDU.
    ///
    /// The request PDU must begin with the function code. The returned response
    /// PDU also begins with the function code, unless the server replied with
    /// an exception.
    pub fn dispatch(&mut self, slave: u8, request_pdu: &[u8]) -> Result<Vec<u8>, ClientError> {
        if request_pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];

        let adu = RtuAdu::new(slave, request_pdu.to_vec());
        let mut tx = [0u8; 512];
        let n = adu.encode(&mut tx).map_err(ClientError::Encode)?;
        self.transport.send(&tx[..n])?;

        let mut rx = [0u8; 512];
        let m = self.transport.recv(&mut rx, self.config.timeout)?;
        if m == 0 {
            return Err(ClientError::Transport(TransportError::Disconnected));
        }
        let response = RtuAdu::decode(&rx[..m]).map_err(ClientError::Decode)?;
        if response.address != slave {
            return Err(ClientError::InvalidResponse);
        }
        if response.pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }

        let response_function = response.pdu[0];
        if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
            let exc = ExceptionResponse::decode(&response.pdu).map_err(ClientError::Decode)?;
            return Err(ClientError::Exception(exc));
        }
        if response_function != request_function {
            return Err(ClientError::InvalidResponse);
        }

        Ok(response.pdu)
    }

    /// Read `quantity` coils starting at `address` from `slave`.
    pub fn read_coils(
        &mut self,
        slave: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadCoilsRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadCoilsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.coil_status)
    }

    /// Read `quantity` discrete inputs starting at `address` from `slave`.
    pub fn read_discrete_inputs(
        &mut self,
        slave: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadDiscreteInputsRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadDiscreteInputsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.input_status)
    }

    /// Read `quantity` holding registers starting at `address` from `slave`.
    pub fn read_holding_registers(
        &mut self,
        slave: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req =
            ReadHoldingRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadHoldingRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read `quantity` input registers starting at `address` from `slave`.
    pub fn read_input_registers(
        &mut self,
        slave: u8,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ClientError> {
        let req = ReadInputRegistersRequest::new(address, quantity).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadInputRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Write a single coil at `address` on `slave`.
    pub fn write_coil(&mut self, slave: u8, address: u16, value: bool) -> Result<(), ClientError> {
        let raw = if value {
            WriteSingleCoilRequest::ON
        } else {
            WriteSingleCoilRequest::OFF
        };
        let req = WriteSingleCoilRequest::new(address, raw).map_err(ClientError::Decode)?;
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let _ = WriteSingleCoilResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write a single holding register at `address` on `slave`.
    pub fn write_register(
        &mut self,
        slave: u8,
        address: u16,
        value: u16,
    ) -> Result<(), ClientError> {
        let req = WriteSingleRegisterRequest::new(address, value);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let _ = WriteSingleRegisterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write multiple coils starting at `address` on `slave`.
    pub fn write_coils(
        &mut self,
        slave: u8,
        address: u16,
        values: &[bool],
    ) -> Result<(), ClientError> {
        let outputs = pack_bits(values);
        let quantity = values.len() as u16;
        let req = WriteMultipleCoilsRequest::new(address, quantity, outputs)
            .map_err(ClientError::Decode)?;
        let mut buf = vec![0u8; 6 + req.outputs.len()];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let _ = WriteMultipleCoilsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Write multiple holding registers starting at `address` on `slave`.
    pub fn write_registers(
        &mut self,
        slave: u8,
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
        let pdu = self.dispatch(slave, &buf[..n])?;
        let _ = WriteMultipleRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(())
    }

    /// Read the exception status byte from `slave` (FC 0x07).
    pub fn read_exception_status(&mut self, slave: u8) -> Result<u8, ClientError> {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadExceptionStatusResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.data)
    }

    /// Execute a diagnostics sub-function on `slave` (FC 0x08).
    ///
    /// Returns the echoed `(sub_function, data)` pair from the response.
    pub fn diagnostics(
        &mut self,
        slave: u8,
        sub_function: u16,
        data: u16,
    ) -> Result<(u16, u16), ClientError> {
        let req = DiagnosticsRequest::new(sub_function, data);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = DiagnosticsResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.sub_function, resp.data))
    }

    /// Read the communication event counter from `slave` (FC 0x0B).
    pub fn get_comm_event_counter(&mut self, slave: u8) -> Result<(u16, u16), ClientError> {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = GetCommEventCounterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.status, resp.event_count))
    }

    /// Read the communication event log from `slave` (FC 0x0C).
    pub fn get_comm_event_log(
        &mut self,
        slave: u8,
    ) -> Result<(u16, u16, u16, Vec<u8>), ClientError> {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = GetCommEventLogResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.status, resp.event_count, resp.message_count, resp.events))
    }

    /// Report the server ID from `slave` (FC 0x11).
    pub fn report_server_id(&mut self, slave: u8) -> Result<Vec<u8>, ClientError> {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReportServerIdResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.data)
    }

    /// Mask-write a holding register on `slave` (FC 0x16).
    ///
    /// Returns the `(reference_address, and_mask, or_mask)` echoed by the server.
    pub fn mask_write_register(
        &mut self,
        slave: u8,
        reference_address: u16,
        and_mask: u16,
        or_mask: u16,
    ) -> Result<(u16, u16, u16), ClientError> {
        let req = MaskWriteRegisterRequest::new(reference_address, and_mask, or_mask);
        let mut buf = [0u8; 7];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = MaskWriteRegisterResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.reference_address, resp.and_mask, resp.or_mask))
    }

    /// Atomically read and write holding registers on `slave` (FC 0x17).
    ///
    /// `write_values` are written starting at `write_address`; the returned
    /// bytes contain `read_quantity` registers starting at `read_address`.
    pub fn read_write_multiple_registers(
        &mut self,
        slave: u8,
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
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadWriteMultipleRegistersResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok(resp.register_values)
    }

    /// Read the FIFO queue at `fifo_pointer_address` from `slave` (FC 0x18).
    pub fn read_fifo_queue(
        &mut self,
        slave: u8,
        fifo_pointer_address: u16,
    ) -> Result<(u16, Vec<u8>), ClientError> {
        let req = ReadFifoQueueRequest::new(fifo_pointer_address);
        let mut buf = [0u8; 3];
        let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
        let pdu = self.dispatch(slave, &buf[..n])?;
        let resp = ReadFifoQueueResponse::decode(&pdu).map_err(ClientError::Decode)?;
        Ok((resp.fifo_count, resp.register_values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
    use crate::rtu::RtuAdu;
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

    impl Transport for MockTransport {
        fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
            self.sent.push(data.to_vec());
            Ok(())
        }

        fn recv(&mut self, buf: &mut [u8], _timeout: Duration) -> Result<usize, TransportError> {
            let resp = self
                .responses
                .pop_front()
                .ok_or(TransportError::Disconnected)?;
            let n = resp.len().min(buf.len());
            buf[..n].copy_from_slice(&resp[..n]);
            Ok(n)
        }
    }

    #[test]
    fn dispatch_read_coils_roundtrip() {
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

        let mut client = Client::new(MockTransport::new(vec![response_adu]));
        let pdu = client.dispatch(0x01, &request_pdu).unwrap();
        assert_eq!(pdu, response_pdu);

        let decoded = ReadCoilsResponse::decode(&pdu).unwrap();
        assert_eq!(decoded.coil_status, vec![0b11001011, 0b00000010]);
    }

    #[test]
    fn dispatch_returns_exception() {
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

        let mut client = Client::new(MockTransport::new(vec![response_adu]));
        let err = client.dispatch(0x01, &request_pdu).unwrap_err();
        assert!(matches!(err, ClientError::Exception(_)));
    }

    #[test]
    fn dispatch_propagates_timeout() {
        struct TimeoutTransport;
        impl Transport for TimeoutTransport {
            fn send(&mut self, _data: &[u8]) -> Result<(), TransportError> {
                Ok(())
            }
            fn recv(
                &mut self,
                _buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                Err(TransportError::Timeout)
            }
        }

        let mut client = Client::new(TimeoutTransport);
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x0A])
            .unwrap_err();
        assert!(matches!(err, ClientError::Timeout));
    }

    #[test]
    fn dispatch_rejects_wrong_slave() {
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

        let mut client = Client::new(MockTransport::new(vec![response_adu]));
        let err = client
            .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01])
            .unwrap_err();
        assert!(matches!(err, ClientError::InvalidResponse));
    }
}
