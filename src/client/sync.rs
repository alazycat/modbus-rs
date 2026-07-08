//! Synchronous Modbus client core.

#![cfg(feature = "sync")]

use alloc::vec;
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

use super::{AduAdapter, ClientConfig, ClientError, RtuAduAdapter};
#[cfg(test)]
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
use crate::macros::impl_client_methods;
use crate::transport::Transport;

/// Generic synchronous Modbus client.
///
/// The client dispatches request PDUs through an [`AduAdapter`], waits for the
/// response, and performs basic response validation.
#[derive(Debug)]
pub struct ClientCore<A: AduAdapter> {
    adapter: A,
}

impl<A: AduAdapter> ClientCore<A> {
    /// Create a client around an adapter.
    pub fn new(adapter: A) -> Self {
        Self { adapter }
    }

    /// Dispatch a request PDU to `unit_id` and return the response PDU.
    ///
    /// The request PDU must begin with the function code. The returned response
    /// PDU also begins with the function code, unless the server replied with
    /// an exception.
    pub fn dispatch(&mut self, unit_id: u8, request_pdu: &[u8]) -> Result<Vec<u8>, ClientError> {
        if request_pdu.is_empty() {
            return Err(ClientError::InvalidResponse);
        }
        let request_function = request_pdu[0];
        let response_pdu = self.adapter.send_receive(unit_id, request_pdu)?;
        super::validate_response_function(request_function, &response_pdu)?;
        Ok(response_pdu)
    }

    impl_client_methods!([] []);


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
        Self(ClientCore::new(RtuAduAdapter::with_config(transport, config)))
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
