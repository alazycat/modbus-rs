//! Synchronous Modbus server dispatcher.

#![cfg(any(feature = "sync", feature = "async"))]

use alloc::vec;
use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};
use crate::exception::{ExceptionCode, ExceptionResponse};
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
use crate::function_codes::read_file_record::{ReadFileRecordRequest, ReadFileRecordResponse};
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
use crate::function_codes::write_file_record::{WriteFileRecordRequest, WriteFileRecordResponse};
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

use super::{DataStore, RequestHook};

/// A synchronous Modbus server.
///
/// The server dispatches request PDUs to a [`DataStore`] and encodes the
/// resulting response PDU (or exception response) into the supplied buffer.
#[derive(Debug)]
pub struct Server<D: DataStore> {
    store: D,
    hook: Option<Box<dyn RequestHook>>,
    #[cfg(feature = "metrics")]
    metrics: Option<alloc::sync::Arc<crate::metrics::Metrics>>,
}

impl<D: DataStore> Server<D> {
    /// Create a server backed by `store`.
    pub fn new(store: D) -> Self {
        Self {
            store,
            hook: None,
            #[cfg(feature = "metrics")]
            metrics: None,
        }
    }

    /// Create a server backed by `store` and attach `metrics`.
    #[cfg(feature = "metrics")]
    pub fn with_metrics(store: D, metrics: alloc::sync::Arc<crate::metrics::Metrics>) -> Self {
        Self {
            store,
            hook: None,
            metrics: Some(metrics),
        }
    }

    /// Attach a shared [`Metrics`] instance to this server.
    #[cfg(feature = "metrics")]
    pub fn set_metrics(&mut self, metrics: alloc::sync::Arc<crate::metrics::Metrics>) {
        self.metrics = Some(metrics);
    }

    /// Return an immutable reference to the underlying store.
    pub fn store(&self) -> &D {
        &self.store
    }

    /// Return a mutable reference to the underlying store.
    pub fn store_mut(&mut self) -> &mut D {
        &mut self.store
    }

    /// Attach a request hook to this server.
    pub fn with_hook(mut self, hook: Box<dyn RequestHook>) -> Self {
        self.hook = Some(hook);
        self
    }

    /// Dispatch a request PDU with hook interception and write the response PDU
    /// into `response`.
    ///
    /// `unit_id` is passed to the hook for context; it is not part of the PDU
    /// itself.
    pub fn dispatch_with_hook(
        &mut self,
        unit_id: u8,
        request: &[u8],
        response: &mut [u8],
    ) -> Result<usize, EncodeError> {
        if let Some(ref mut hook) = self.hook {
            if let Err(exc) = hook.before_request(unit_id, request) {
                let n = encode_exception(exc.function_code, exc.exception_code, response)?;
                hook.after_response(unit_id, request, &response[..n]);
                return Ok(n);
            }
        }

        let n = self.dispatch(request, response)?;

        if let Some(ref mut hook) = self.hook {
            hook.after_response(unit_id, request, &response[..n]);
        }

        Ok(n)
    }

    /// Dispatch a request PDU and write the response PDU into `response`.
    ///
    /// Returns the number of bytes written to `response`.
    pub fn dispatch(&mut self, request: &[u8], response: &mut [u8]) -> Result<usize, EncodeError> {
        #[cfg(feature = "tracing")]
        tracing::trace!(request_len = request.len(), "dispatching request");
        #[cfg(feature = "metrics")]
        if let Some(ref metrics) = self.metrics {
            metrics.record_request_received();
        }
        if request.is_empty() {
            let n = encode_exception(0, ExceptionCode::IllegalFunction, response)?;
            #[cfg(feature = "metrics")]
            if let Some(ref metrics) = self.metrics {
                metrics.record_response_sent();
            }
            return Ok(n);
        }

        let function_code = request[0];
        match self.process_request(function_code, request) {
            Ok(pdu) => {
                #[cfg(feature = "tracing")]
                tracing::trace!(function_code, response_len = pdu.len(), "request processed");
                if response.len() < pdu.len() {
                    #[cfg(feature = "metrics")]
                    if let Some(ref metrics) = self.metrics {
                        metrics.record_error();
                    }
                    return Err(EncodeError::BufferTooSmall);
                }
                response[..pdu.len()].copy_from_slice(&pdu);
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_response_sent();
                }
                Ok(pdu.len())
            }
            Err(code) => {
                #[cfg(feature = "tracing")]
                tracing::trace!(function_code, ?code, "request returned exception");
                let n = encode_exception(function_code, code, response)?;
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_response_sent();
                }
                Ok(n)
            }
        }
    }

    fn process_request(
        &mut self,
        function_code: u8,
        request: &[u8],
    ) -> Result<Vec<u8>, ExceptionCode> {
        match function_code {
            ReadCoilsRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadCoilsRequest>(request)?;
                let coil_status = self.store.read_coils(req.starting_address, req.quantity)?;
                encode_pdu(ReadCoilsResponse { coil_status })
            }
            ReadDiscreteInputsRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadDiscreteInputsRequest>(request)?;
                let input_status = self
                    .store
                    .read_discrete_inputs(req.starting_address, req.quantity)?;
                encode_pdu(ReadDiscreteInputsResponse { input_status })
            }
            ReadHoldingRegistersRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadHoldingRegistersRequest>(request)?;
                let register_values = self
                    .store
                    .read_holding_registers(req.starting_address, req.quantity)?;
                encode_pdu(ReadHoldingRegistersResponse { register_values })
            }
            ReadInputRegistersRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadInputRegistersRequest>(request)?;
                let register_values = self
                    .store
                    .read_input_registers(req.starting_address, req.quantity)?;
                encode_pdu(ReadInputRegistersResponse { register_values })
            }
            WriteSingleCoilRequest::FUNCTION_CODE => {
                let req = decode_request::<WriteSingleCoilRequest>(request)?;
                let value = req.value == WriteSingleCoilRequest::ON;
                self.store.write_coil(req.output_address, value)?;
                encode_pdu(WriteSingleCoilResponse {
                    output_address: req.output_address,
                    value: req.value,
                })
            }
            WriteSingleRegisterRequest::FUNCTION_CODE => {
                let req = decode_request::<WriteSingleRegisterRequest>(request)?;
                self.store.write_register(req.register_address, req.value)?;
                encode_pdu(WriteSingleRegisterResponse {
                    register_address: req.register_address,
                    value: req.value,
                })
            }
            WriteMultipleCoilsRequest::FUNCTION_CODE => {
                let req = decode_request::<WriteMultipleCoilsRequest>(request)?;
                let values = unpack_bits(&req.outputs, req.quantity as usize);
                self.store.write_coils(req.starting_address, &values)?;
                encode_pdu(WriteMultipleCoilsResponse {
                    starting_address: req.starting_address,
                    quantity: req.quantity,
                })
            }
            WriteMultipleRegistersRequest::FUNCTION_CODE => {
                let req = decode_request::<WriteMultipleRegistersRequest>(request)?;
                let values = bytes_to_registers(&req.register_values);
                self.store.write_registers(req.starting_address, &values)?;
                encode_pdu(WriteMultipleRegistersResponse {
                    starting_address: req.starting_address,
                    quantity: req.quantity,
                })
            }
            MaskWriteRegisterRequest::FUNCTION_CODE => {
                let req = decode_request::<MaskWriteRegisterRequest>(request)?;
                let current_bytes = self
                    .store
                    .read_holding_registers(req.reference_address, 1)?;
                let current = u16::from_be_bytes([current_bytes[0], current_bytes[1]]);
                let new_value = (current & req.and_mask) | (req.or_mask & !req.and_mask);
                self.store
                    .write_register(req.reference_address, new_value)?;
                encode_pdu(MaskWriteRegisterResponse {
                    reference_address: req.reference_address,
                    and_mask: req.and_mask,
                    or_mask: req.or_mask,
                })
            }
            ReadWriteMultipleRegistersRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadWriteMultipleRegistersRequest>(request)?;
                let write_values = bytes_to_registers(&req.write_values);
                self.store
                    .write_registers(req.write_starting_address, &write_values)?;
                let register_values = self
                    .store
                    .read_holding_registers(req.read_starting_address, req.read_quantity)?;
                encode_pdu(ReadWriteMultipleRegistersResponse { register_values })
            }
            ReadExceptionStatusRequest::FUNCTION_CODE => {
                let _req = decode_request::<ReadExceptionStatusRequest>(request)?;
                let data = self.store.read_exception_status()?;
                encode_pdu(ReadExceptionStatusResponse { data })
            }
            DiagnosticsRequest::FUNCTION_CODE => {
                let req = decode_request::<DiagnosticsRequest>(request)?;
                let (sub_function, data) = self.store.diagnostics(req.sub_function, req.data)?;
                encode_pdu(DiagnosticsResponse { sub_function, data })
            }
            GetCommEventCounterRequest::FUNCTION_CODE => {
                let _req = decode_request::<GetCommEventCounterRequest>(request)?;
                let (status, event_count) = self.store.get_comm_event_counter()?;
                encode_pdu(GetCommEventCounterResponse {
                    status,
                    event_count,
                })
            }
            GetCommEventLogRequest::FUNCTION_CODE => {
                let _req = decode_request::<GetCommEventLogRequest>(request)?;
                let (status, event_count, message_count, events) =
                    self.store.get_comm_event_log()?;
                encode_pdu(
                    GetCommEventLogResponse::new(status, event_count, message_count, events)
                        .map_err(|_| ExceptionCode::ServerDeviceFailure)?,
                )
            }
            ReportServerIdRequest::FUNCTION_CODE => {
                let _req = decode_request::<ReportServerIdRequest>(request)?;
                let data = self.store.report_server_id()?;
                encode_pdu(ReportServerIdResponse::new(data))
            }
            ReadFifoQueueRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadFifoQueueRequest>(request)?;
                let (fifo_count, register_values) =
                    self.store.read_fifo_queue(req.fifo_pointer_address)?;
                encode_pdu(ReadFifoQueueResponse {
                    fifo_count,
                    register_values,
                })
            }
            ReadFileRecordRequest::FUNCTION_CODE => {
                let req = decode_request::<ReadFileRecordRequest>(request)?;
                let sub_responses = self.store.read_file_record(&req.sub_requests)?;
                encode_pdu(ReadFileRecordResponse { sub_responses })
            }
            WriteFileRecordRequest::FUNCTION_CODE => {
                let req = decode_request::<WriteFileRecordRequest>(request)?;
                let sub_responses = self.store.write_file_record(&req.sub_requests)?;
                encode_pdu(WriteFileRecordResponse { sub_responses })
            }
            EncapsulatedInterfaceTransportRequest::FUNCTION_CODE => {
                let req = decode_request::<EncapsulatedInterfaceTransportRequest>(request)?;
                let (mei_type, data) = self
                    .store
                    .encapsulated_interface_transport(req.mei_type, &req.data)?;
                encode_pdu(EncapsulatedInterfaceTransportResponse { mei_type, data })
            }
            _ => Err(ExceptionCode::IllegalFunction),
        }
    }
}

fn decode_request<R>(request: &[u8]) -> Result<R, ExceptionCode>
where
    R: Request,
{
    R::decode(request).map_err(map_decode_error)
}

fn encode_pdu<R>(response: R) -> Result<Vec<u8>, ExceptionCode>
where
    R: Response,
{
    let mut buf = vec![0u8; R::max_len()];
    let n = response
        .encode(&mut buf)
        .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
    buf.truncate(n);
    Ok(buf)
}

fn encode_exception(
    function_code: u8,
    code: ExceptionCode,
    buf: &mut [u8],
) -> Result<usize, EncodeError> {
    ExceptionResponse::new(function_code, code).encode(buf)
}

fn map_decode_error(err: DecodeError) -> ExceptionCode {
    match err {
        DecodeError::UnknownFunctionCode => ExceptionCode::IllegalFunction,
        _ => ExceptionCode::IllegalDataValue,
    }
}

fn unpack_bits(bytes: &[u8], count: usize) -> Vec<bool> {
    let mut bits = Vec::with_capacity(count);
    for i in 0..count {
        let byte = bytes[i / 8];
        bits.push((byte >> (i % 8)) & 1 == 1);
    }
    bits
}

fn bytes_to_registers(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect()
}

/// Internal helper trait so `decode_request` can be generic over request types.
trait Request {
    fn decode(buf: &[u8]) -> Result<Self, DecodeError>
    where
        Self: Sized;
}

/// Internal helper trait so `encode_pdu` can be generic over response types.
trait Response {
    fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
    fn max_len() -> usize;
}

macro_rules! impl_request {
    ($type:ty) => {
        impl Request for $type {
            fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
                <$type>::decode(buf)
            }
        }
    };
}

macro_rules! impl_response {
    ($type:ty, $max_len:expr) => {
        impl Response for $type {
            fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
                <$type>::encode(self, buf)
            }
            fn max_len() -> usize {
                $max_len
            }
        }
    };
}

impl_request!(ReadCoilsRequest);
impl_request!(ReadDiscreteInputsRequest);
impl_request!(ReadHoldingRegistersRequest);
impl_request!(ReadInputRegistersRequest);
impl_request!(WriteSingleCoilRequest);
impl_request!(WriteSingleRegisterRequest);
impl_request!(WriteMultipleCoilsRequest);
impl_request!(WriteMultipleRegistersRequest);
impl_request!(MaskWriteRegisterRequest);
impl_request!(ReadWriteMultipleRegistersRequest);
impl_request!(ReadExceptionStatusRequest);
impl_request!(DiagnosticsRequest);
impl_request!(GetCommEventCounterRequest);
impl_request!(GetCommEventLogRequest);
impl_request!(ReportServerIdRequest);
impl_request!(ReadFifoQueueRequest);
impl_request!(ReadFileRecordRequest);
impl_request!(WriteFileRecordRequest);
impl_request!(EncapsulatedInterfaceTransportRequest);

impl_response!(ReadCoilsResponse, 253);
impl_response!(ReadDiscreteInputsResponse, 253);
impl_response!(ReadHoldingRegistersResponse, 253);
impl_response!(ReadInputRegistersResponse, 253);
impl_response!(WriteSingleCoilResponse, 5);
impl_response!(WriteSingleRegisterResponse, 5);
impl_response!(WriteMultipleCoilsResponse, 5);
impl_response!(WriteMultipleRegistersResponse, 5);
impl_response!(MaskWriteRegisterResponse, 7);
impl_response!(ReadWriteMultipleRegistersResponse, 253);
impl_response!(ReadExceptionStatusResponse, 2);
impl_response!(DiagnosticsResponse, 5);
impl_response!(GetCommEventCounterResponse, 5);
impl_response!(GetCommEventLogResponse, 72);
impl_response!(ReportServerIdResponse, 257);
impl_response!(ReadFifoQueueResponse, 3 + 2 + 256); // header + fifo count + max data (2-byte byte count supports up to u16::MAX but response buffer is 512)
impl_response!(ReadFileRecordResponse, 253);
impl_response!(WriteFileRecordResponse, 253);
impl_response!(EncapsulatedInterfaceTransportResponse, 253);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::MemoryStore;

    fn dispatch(store: MemoryStore, request: &[u8]) -> (usize, Vec<u8>) {
        let mut server = Server::new(store);
        let mut response = [0u8; 512];
        let n = server.dispatch(request, &mut response).unwrap();
        (n, response[..n].to_vec())
    }

    #[test]
    fn read_coils() {
        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let req = ReadCoilsRequest::new(0, 8).unwrap();
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let (n, response) = dispatch(store.clone(), &request);
        assert_eq!(n, 3);
        assert_eq!(response, vec![0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn write_and_read_holding_register() {
        let store = MemoryStore::new(0, 0, 4, 0);

        let req = WriteSingleRegisterRequest::new(1, 0x1234);
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let (n, response) = dispatch(store.clone(), &request);
        assert_eq!(n, 5);
        assert_eq!(response, vec![0x06, 0x00, 0x01, 0x12, 0x34]);
    }

    #[test]
    fn unknown_function_returns_exception() {
        let store = MemoryStore::new(0, 0, 0, 0);
        let (n, response) = dispatch(store, &[0x7F, 0x00, 0x00, 0x00, 0x01]);
        assert_eq!(n, 2);
        assert_eq!(response[0], 0x7F | ExceptionResponse::EXCEPTION_FLAG);
        assert_eq!(response[1], ExceptionCode::IllegalFunction as u8);
    }

    #[test]
    fn out_of_range_read_returns_exception() {
        let store = MemoryStore::new(0, 0, 1, 0);
        let req = ReadHoldingRegistersRequest::new(0, 2).unwrap();
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let (n, response) = dispatch(store.clone(), &request);
        assert_eq!(n, 2);
        assert_eq!(response[0], 0x83);
        assert_eq!(response[1], ExceptionCode::IllegalDataAddress as u8);
    }

    #[test]
    fn mask_write_register() {
        let store = MemoryStore::new(0, 0, 1, 0);
        let mut server = Server::new(store);
        server.store_mut().write_register(0, 0x1234).unwrap();

        let req = MaskWriteRegisterRequest::new(0, 0x00FF, 0x0001);
        let mut request = [0u8; 7];
        req.encode(&mut request).unwrap();

        let mut response = [0u8; 512];
        let n = server.dispatch(&request, &mut response).unwrap();
        assert_eq!(n, 7);
        assert_eq!(response[..n], [0x16, 0x00, 0x00, 0x00, 0xFF, 0x00, 0x01]);

        let new_value = server.store().read_holding_registers(0, 1).unwrap();
        assert_eq!(new_value, vec![0x00, 0x34]);
    }

    #[cfg(feature = "metrics")]
    #[test]
    fn metrics_count_request_and_response() {
        use crate::metrics::Metrics;
        use alloc::sync::Arc;

        let metrics = Arc::new(Metrics::new());
        let mut server = Server::with_metrics(MemoryStore::new(16, 0, 0, 0), Arc::clone(&metrics));
        server
            .store_mut()
            .write_coils(0, &[true, false, true, true])
            .unwrap();

        let req = ReadCoilsRequest::new(0, 8).unwrap();
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let mut response = [0u8; 512];
        let n = server.dispatch(&request, &mut response).unwrap();
        assert_eq!(n, 3);

        assert_eq!(metrics.requests_received(), 1);
        assert_eq!(metrics.responses_sent(), 1);
    }

    #[test]
    fn noop_hook_does_not_change_response() {
        use crate::server::NoopHook;

        let mut store = MemoryStore::new(16, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();

        let req = ReadCoilsRequest::new(0, 8).unwrap();
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let mut server = Server::new(store).with_hook(Box::new(NoopHook));
        let mut response = [0u8; 512];
        let n = server.dispatch_with_hook(0x01, &request, &mut response).unwrap();

        assert_eq!(n, 3);
        assert_eq!(response[..n], [0x01, 0x01, 0b00001101]);
    }

    #[test]
    fn hook_rejection_returns_exception_without_touching_store() {
        use crate::server::RequestHook;

        #[derive(Debug)]
        struct RejectAll;

        impl RequestHook for RejectAll {
            fn before_request(
                &mut self,
                _unit_id: u8,
                request_pdu: &[u8],
            ) -> Result<(), ExceptionResponse> {
                Err(ExceptionResponse::new(
                    request_pdu[0],
                    ExceptionCode::IllegalFunction,
                ))
            }

            fn after_response(
                &mut self,
                _unit_id: u8,
                _request_pdu: &[u8],
                _response_pdu: &[u8],
            ) {
            }
        }

        let mut server = Server::new(MemoryStore::new(0, 0, 0, 0)).with_hook(Box::new(RejectAll));

        let req = ReadCoilsRequest::new(0, 8).unwrap();
        let mut request = [0u8; 5];
        req.encode(&mut request).unwrap();

        let mut response = [0u8; 512];
        let n = server
            .dispatch_with_hook(0x01, &request, &mut response)
            .unwrap();

        assert_eq!(n, 2);
        assert_eq!(response[0], 0x01 | ExceptionResponse::EXCEPTION_FLAG);
        assert_eq!(response[1], ExceptionCode::IllegalFunction as u8);
    }
}
