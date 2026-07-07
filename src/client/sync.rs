//! Synchronous Modbus client core.

#![cfg(feature = "sync")]

use alloc::vec::Vec;
use core::time::Duration;

use crate::error::{DecodeError, EncodeError};
use crate::exception::ExceptionResponse;
use crate::rtu::RtuAdu;
use crate::transport::{Transport, TransportError};

/// Configuration for a synchronous client.
#[derive(Debug, Clone, Copy)]
pub struct ClientConfig {
    /// Maximum time to wait for a response.
    pub timeout: Duration,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
        }
    }
}

/// Errors that can occur while using the synchronous client.
#[derive(Debug)]
pub enum ClientError {
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

impl core::fmt::Display for ClientError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Transport(e) => write!(f, "client transport error: {e}"),
            Self::Encode(e) => write!(f, "client encode error: {e:?}"),
            Self::Decode(e) => write!(f, "client decode error: {e:?}"),
            Self::Timeout => write!(f, "client timeout"),
            Self::InvalidResponse => write!(f, "invalid response"),
            Self::Exception(e) => write!(f, "server exception: {e:?}"),
        }
    }
}

impl std::error::Error for ClientError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Transport(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for ClientError {
    fn from(e: TransportError) -> Self {
        match e {
            TransportError::Timeout => Self::Timeout,
            other => Self::Transport(other),
        }
    }
}

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
    pub fn dispatch(
        &mut self,
        slave: u8,
        request_pdu: &[u8],
    ) -> Result<Vec<u8>, ClientError> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function_codes::read_coils::{ReadCoilsRequest, ReadCoilsResponse};
    use crate::rtu::RtuAdu;
    use alloc::collections::VecDeque;

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

        fn recv(
            &mut self,
            buf: &mut [u8],
            _timeout: Duration,
        ) -> Result<usize, TransportError> {
            let resp = self.responses.pop_front().ok_or(TransportError::Disconnected)?;
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
            let exc = ExceptionResponse::new(0x01, crate::exception::ExceptionCode::IllegalDataAddress);
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
        let err = client.dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x0A]).unwrap_err();
        assert!(matches!(err, ClientError::Timeout));
    }

    #[test]
    fn dispatch_rejects_wrong_slave() {
        let response_adu = {
            let pdu = {
                let resp = ReadCoilsResponse { coil_status: vec![0x01] };
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
        let err = client.dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x01]).unwrap_err();
        assert!(matches!(err, ClientError::InvalidResponse));
    }
}
