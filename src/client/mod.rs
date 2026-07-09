//! Synchronous and asynchronous Modbus clients.

#![cfg(any(feature = "sync", feature = "async"))]

use core::time::Duration;

use crate::error::{DecodeError, EncodeError};
use crate::exception::ExceptionResponse;
#[cfg(feature = "helpers")]
use crate::helpers::{self, Endian, WordOrder};
use crate::transport::TransportError;

#[macro_use]
mod macros;

#[cfg(feature = "sync")]
pub mod sync;

#[cfg(feature = "async")]
pub mod r#async;

#[cfg(any(feature = "rtu", feature = "async"))]
pub mod rtu_adapter;

#[cfg(feature = "sync")]
pub use sync::{Client, ClientCore};

#[cfg(feature = "async")]
pub use r#async::{AsyncClient, AsyncClientCore};

#[cfg(all(feature = "sync", any(feature = "rtu", feature = "async")))]
pub use rtu_adapter::RtuAduAdapter;

#[cfg(all(feature = "async", any(feature = "rtu", feature = "async")))]
pub use rtu_adapter::AsyncRtuAduAdapter;

#[cfg(any(feature = "sync", feature = "async"))]
pub mod retry_adapter;

#[cfg(all(feature = "ascii", any(feature = "sync", feature = "async")))]
pub use crate::ascii_client::{AsciiClientConfig, AsciiClientError};

#[cfg(all(feature = "ascii", feature = "sync"))]
pub use crate::ascii_client::AsciiClient;

#[cfg(all(feature = "ascii", feature = "async"))]
pub use crate::ascii_client::AsyncAsciiClient;

#[cfg(all(feature = "udp", any(feature = "sync", feature = "async")))]
pub use crate::udp_client::{UdpClientConfig, UdpClientError};

#[cfg(all(feature = "udp", feature = "sync"))]
pub use crate::udp_client::UdpClient;

#[cfg(all(feature = "udp", feature = "async"))]
pub use crate::udp_client::AsyncUdpClient;

/// Configuration shared between synchronous and asynchronous clients.
#[derive(Debug, Clone, Copy)]
pub struct ClientConfig {
    /// Maximum time to wait for a response.
    pub timeout: Duration,
    /// Maximum time the connection may remain idle before reads/writes time out.
    ///
    /// When set, the underlying stream's read and write timeouts are configured
    /// to this value so that an idle connection is closed instead of remaining
    /// open indefinitely. If `None`, the response `timeout` is used as today.
    pub idle_timeout: Option<Duration>,
    /// Byte order used by typed register helpers.
    #[cfg(feature = "helpers")]
    pub endian: Endian,
    /// Word order used by multi-register typed helpers.
    #[cfg(feature = "helpers")]
    pub word_order: WordOrder,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(5),
            idle_timeout: None,
            #[cfg(feature = "helpers")]
            endian: Endian::Big,
            #[cfg(feature = "helpers")]
            word_order: WordOrder::MostSignificantFirst,
        }
    }
}

/// Policy controlling retry behavior for reconnecting adapters.
#[derive(Debug, Clone, Copy)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts before giving up.
    pub max_retries: u32,
    /// Initial delay before the first retry.
    pub initial_backoff: Duration,
    /// Maximum delay between retries.
    pub max_backoff: Duration,
    /// Predicate that decides whether an error is worth retrying.
    pub retryable: fn(&ClientError) -> bool,
}

/// Default retry predicate: treat transport disconnects as retryable.
pub fn default_retryable(err: &ClientError) -> bool {
    matches!(err, ClientError::Transport(TransportError::Disconnected))
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(5),
            retryable: default_retryable,
        }
    }
}

/// Errors that can occur while using a Modbus client.
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
    /// A TLS handshake or certificate error.
    #[cfg(feature = "tls")]
    Tls(String),
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
            #[cfg(feature = "tls")]
            Self::Tls(e) => write!(f, "TLS error: {e}"),
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

#[cfg(feature = "helpers")]
impl From<crate::helpers::HelpersError> for ClientError {
    fn from(e: crate::helpers::HelpersError) -> Self {
        match e {
            crate::helpers::HelpersError::InvalidLength => Self::Decode(DecodeError::InvalidLength),
            crate::helpers::HelpersError::InvalidString => Self::Decode(DecodeError::InvalidValue),
        }
    }
}

/// Validate that `response_pdu` echoes the requested function code.
///
/// If the response begins with the request function code combined with the
/// exception flag, it is decoded as an [`ExceptionResponse`] and returned as
/// [`ClientError::Exception`]. A mismatching or empty response yields
/// [`ClientError::InvalidResponse`].
pub(crate) fn validate_response_function(
    request_function: u8,
    response_pdu: &[u8],
) -> Result<(), ClientError> {
    if response_pdu.is_empty() {
        return Err(ClientError::InvalidResponse);
    }
    let response_function = response_pdu[0];
    if response_function == request_function | ExceptionResponse::EXCEPTION_FLAG {
        let exc = ExceptionResponse::decode(response_pdu).map_err(ClientError::Decode)?;
        return Err(ClientError::Exception(exc));
    }
    if response_function != request_function {
        return Err(ClientError::InvalidResponse);
    }
    Ok(())
}

/// Seam for synchronous ADU framing and I/O.
#[cfg(feature = "sync")]
pub trait AduAdapter {
    /// Send `request_pdu` to `unit_id` and return the response PDU.
    fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError>;
}

/// Seam for asynchronous ADU framing and I/O.
#[cfg(feature = "async")]
#[allow(async_fn_in_trait)]
pub trait AsyncAduAdapter {
    /// Send `request_pdu` to `unit_id` and return the response PDU.
    async fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError>;
}

pub(crate) fn pack_bits(bits: &[bool]) -> alloc::vec::Vec<u8> {
    let mut bytes = alloc::vec::Vec::with_capacity(bits.len().div_ceil(8));
    for (i, &bit) in bits.iter().enumerate() {
        if i % 8 == 0 {
            bytes.push(0);
        }
        if bit {
            let last = bytes.last_mut().expect("byte was just pushed");
            *last |= 1 << (i % 8);
        }
    }
    bytes
}

#[cfg(feature = "helpers")]
pub(crate) fn bytes_to_words(
    bytes: &[u8],
    endian: Endian,
) -> Result<alloc::vec::Vec<u16>, ClientError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(ClientError::Decode(DecodeError::InvalidLength));
    }
    bytes
        .chunks_exact(2)
        .map(|chunk| helpers::u16_from_bytes(chunk, endian))
        .collect::<Result<alloc::vec::Vec<_>, _>>()
        .map_err(ClientError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;
    use std::sync::{Arc, Mutex};

    /// Mock adapter that satisfies both [`AduAdapter`] and [`AsyncAduAdapter`]
    /// for tests in this module.
    struct MockAduAdapter {
        handler: Box<dyn Fn(u8, &[u8]) -> Result<Vec<u8>, ClientError> + Send + Sync>,
        recorded: Arc<Mutex<Vec<Vec<u8>>>>,
    }

    impl MockAduAdapter {
        fn new<F>(handler: F) -> Self
        where
            F: Fn(u8, &[u8]) -> Result<Vec<u8>, ClientError> + Send + Sync + 'static,
        {
            Self {
                handler: Box::new(handler),
                recorded: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn recorded(&self) -> Vec<Vec<u8>> {
            self.recorded.lock().unwrap().clone()
        }

        fn share_recorded(&self) -> Arc<Mutex<Vec<Vec<u8>>>> {
            self.recorded.clone()
        }
    }

    #[cfg(feature = "sync")]
    impl AduAdapter for MockAduAdapter {
        fn send_receive(
            &mut self,
            unit_id: u8,
            request_pdu: &[u8],
        ) -> Result<Vec<u8>, ClientError> {
            self.recorded.lock().unwrap().push(request_pdu.to_vec());
            (self.handler)(unit_id, request_pdu)
        }
    }

    #[cfg(feature = "async")]
    impl AsyncAduAdapter for MockAduAdapter {
        async fn send_receive(
            &mut self,
            unit_id: u8,
            request_pdu: &[u8],
        ) -> Result<Vec<u8>, ClientError> {
            self.recorded.lock().unwrap().push(request_pdu.to_vec());
            (self.handler)(unit_id, request_pdu)
        }
    }

    macro_rules! dispatch_tests {
        ([$($async:tt)*] $attr:meta, $core:ident, $adapter:ident, [$($await:tt)*]) => {
            #[$attr]
            $($async)* fn dispatch_read_coils_roundtrip() -> Result<(), ClientError> {
                let request_pdu = {
                    let req = crate::function_codes::read_coils::ReadCoilsRequest::new(0x0000, 10)
                        .map_err(ClientError::Decode)?;
                    let mut buf = [0u8; 5];
                    let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };
                let response_pdu = {
                    let resp = crate::function_codes::read_coils::ReadCoilsResponse {
                        coil_status: vec![0b11001011, 0b00000010],
                    };
                    let mut buf = [0u8; 4];
                    let n = resp.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };

                let expected_response_pdu = response_pdu.clone();
                let mut client = $core::new(MockAduAdapter::new(move |_unit_id, _req| {
                    Ok(response_pdu.clone())
                }));
                let pdu = client.dispatch(0x01, &request_pdu)$($await)*?;
                assert_eq!(pdu, expected_response_pdu);

                let decoded =
                    crate::function_codes::read_coils::ReadCoilsResponse::decode(&pdu)
                        .map_err(ClientError::Decode)?;
                assert_eq!(decoded.coil_status, vec![0b11001011, 0b00000010]);
                Ok(())
            }

            #[$attr]
            $($async)* fn dispatch_returns_exception() -> Result<(), ClientError> {
                let request_pdu = {
                    let req = crate::function_codes::read_coils::ReadCoilsRequest::new(0x0000, 10)
                        .map_err(ClientError::Decode)?;
                    let mut buf = [0u8; 5];
                    let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };
                let exception_pdu = {
                    let exc = crate::exception::ExceptionResponse::new(
                        0x01,
                        crate::exception::ExceptionCode::IllegalDataAddress,
                    );
                    let mut buf = [0u8; 2];
                    let n = exc.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };

                let mut client = $core::new(MockAduAdapter::new(move |_unit_id, _req| {
                    Ok(exception_pdu.clone())
                }));
                let err = client.dispatch(0x01, &request_pdu)$($await)*.unwrap_err();
                assert!(matches!(err, ClientError::Exception(_)));
                Ok(())
            }

            #[$attr]
            $($async)* fn dispatch_propagates_timeout() -> Result<(), ClientError> {
                let mut client = $core::new(MockAduAdapter::new(move |_unit_id, _req| {
                    Err(ClientError::Timeout)
                }));
                let err = client
                    .dispatch(0x01, &[0x01, 0x00, 0x00, 0x00, 0x0A])
                    $($await)*
                    .unwrap_err();
                assert!(matches!(err, ClientError::Timeout));
                Ok(())
            }

            #[$attr]
            $($async)* fn dispatch_rejects_mismatched_function() -> Result<(), ClientError> {
                let request_pdu = {
                    let req = crate::function_codes::read_coils::ReadCoilsRequest::new(0x0000, 1)
                        .map_err(ClientError::Decode)?;
                    let mut buf = [0u8; 5];
                    let n = req.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };
                let response_pdu = {
                    let resp = crate::function_codes::read_discrete_inputs::ReadDiscreteInputsResponse {
                        input_status: vec![0x01],
                    };
                    let mut buf = [0u8; 3];
                    let n = resp.encode(&mut buf).map_err(ClientError::Encode)?;
                    buf[..n].to_vec()
                };

                let mut client = $core::new(MockAduAdapter::new(move |_unit_id, _req| {
                    Ok(response_pdu.clone())
                }));
                let err = client.dispatch(0x01, &request_pdu)$($await)*.unwrap_err();
                assert!(matches!(err, ClientError::InvalidResponse));
                Ok(())
            }
        };
    }

    #[cfg(all(test, feature = "sync"))]
    mod sync_tests {
        use super::*;
        dispatch_tests!([] test, ClientCore, AduAdapter, []);
    }

    #[cfg(all(test, feature = "async"))]
    mod async_tests {
        use super::*;
        dispatch_tests!([async] tokio::test, AsyncClientCore, AsyncAduAdapter, [.await]);
    }

    #[cfg(feature = "helpers")]
    macro_rules! typed_helpers_tests {
        ([$($async:tt)*] $attr:meta, $core:ident, $adapter:ident, [$($await:tt)*]) => {
            #[$attr]
            $($async)* fn read_holding_registers_u16_uses_endian_config() -> Result<(), ClientError> {
                let mut big_config = ClientConfig::default();
                big_config.endian = Endian::Big;
                let mut client = $core::with_config(
                    MockAduAdapter::new(move |_, _| Ok(vec![0x03, 0x02, 0x12, 0x34])),
                    big_config,
                );
                let value = client.read_holding_registers_u16(0x01, 0)$($await)*?;
                assert_eq!(value, 0x1234);

                let mut little_config = ClientConfig::default();
                little_config.endian = Endian::Little;
                let mut client = $core::with_config(
                    MockAduAdapter::new(move |_, _| Ok(vec![0x03, 0x02, 0x12, 0x34])),
                    little_config,
                );
                let value = client.read_holding_registers_u16(0x01, 0)$($await)*?;
                assert_eq!(value, 0x3412);
                Ok(())
            }

            #[$attr]
            $($async)* fn read_holding_registers_u32_big_endian_msf() -> Result<(), ClientError> {
                let mut client = $core::new(MockAduAdapter::new(move |_, _| {
                    Ok(vec![0x03, 0x04, 0x12, 0x34, 0x56, 0x78])
                }));
                let value = client.read_holding_registers_u32(0x01, 0)$($await)*?;
                assert_eq!(value, 0x12345678);
                Ok(())
            }

            #[$attr]
            $($async)* fn read_holding_registers_u32_little_endian_lsf() -> Result<(), ClientError> {
                let mut config = ClientConfig::default();
                config.endian = Endian::Little;
                config.word_order = WordOrder::LeastSignificantFirst;
                let mut client = $core::with_config(
                    MockAduAdapter::new(move |_, _| Ok(vec![0x03, 0x04, 0x56, 0x78, 0x12, 0x34])),
                    config,
                );
                let value = client.read_holding_registers_u32(0x01, 0)$($await)*?;
                assert_eq!(value, 0x12345678);
                Ok(())
            }

            #[$attr]
            $($async)* fn read_holding_registers_f32_roundtrip() -> Result<(), ClientError> {
                let value = 3.1415925f32;
                let regs =
                    crate::helpers::f32_to_registers(value, Endian::Big, WordOrder::MostSignificantFirst);
                let payload: Vec<u8> = regs.iter().flat_map(|&r| r.to_be_bytes()).collect();
                let response = std::iter::once(0x03u8)
                    .chain(std::iter::once(payload.len() as u8))
                    .chain(payload.into_iter())
                    .collect::<Vec<_>>();
                let mut client = $core::new(MockAduAdapter::new(move |_, _| Ok(response.clone())));
                let decoded = client.read_holding_registers_f32(0x01, 0)$($await)*?;
                assert_eq!(decoded, value);
                Ok(())
            }

            #[$attr]
            $($async)* fn read_holding_registers_string_stops_at_nul() -> Result<(), ClientError> {
                let regs =
                    crate::helpers::string_to_registers("Hi", Endian::Big, 4).map_err(ClientError::from)?;
                let payload: Vec<u8> = regs.iter().flat_map(|&r| r.to_be_bytes()).collect();
                let response = std::iter::once(0x03u8)
                    .chain(std::iter::once(payload.len() as u8))
                    .chain(payload.into_iter())
                    .collect::<Vec<_>>();
                let mut client = $core::new(MockAduAdapter::new(move |_, _| Ok(response.clone())));
                let value = client.read_holding_registers_string(0x01, 0, 4)$($await)*?;
                assert_eq!(value, "Hi");
                Ok(())
            }

            #[$attr]
            $($async)* fn read_input_registers_u32_uses_config() -> Result<(), ClientError> {
                let mut client = $core::new(MockAduAdapter::new(move |_, _| {
                    Ok(vec![0x04, 0x04, 0x12, 0x34, 0x56, 0x78])
                }));
                let value = client.read_input_registers_u32(0x01, 0)$($await)*?;
                assert_eq!(value, 0x12345678);
                Ok(())
            }

            #[$attr]
            $($async)* fn write_multiple_registers_u32() -> Result<(), ClientError> {
                let mock = MockAduAdapter::new(move |_, _| {
                    let resp = crate::function_codes::write_multiple_registers::WriteMultipleRegistersResponse {
                        starting_address: 0,
                        quantity: 2,
                    };
                    let mut buf = [0u8; 5];
                    let n = resp.encode(&mut buf).map_err(ClientError::Encode)?;
                    Ok(buf[..n].to_vec())
                });
                let recorded = mock.share_recorded();
                let mut client = $core::new(mock);
                client.write_multiple_registers_u32(0x01, 0, 0xDEADBEEFu32)$($await)*?;
                let requests = recorded.lock().unwrap();
                assert_eq!(requests.len(), 1);
                let req = crate::function_codes::write_multiple_registers::WriteMultipleRegistersRequest::decode(
                    &requests[0],
                )
                .map_err(ClientError::Decode)?;
                let expected =
                    crate::helpers::u32_to_registers(0xDEADBEEFu32, Endian::Big, WordOrder::MostSignificantFirst);
                let expected_bytes: Vec<u8> = expected.iter().flat_map(|&r| r.to_be_bytes()).collect();
                assert_eq!(req.register_values, expected_bytes);
                Ok(())
            }

            #[$attr]
            $($async)* fn write_multiple_registers_f32() -> Result<(), ClientError> {
                let value = -1.5f32;
                let mock = MockAduAdapter::new(move |_, _| {
                    let resp = crate::function_codes::write_multiple_registers::WriteMultipleRegistersResponse {
                        starting_address: 0,
                        quantity: 2,
                    };
                    let mut buf = [0u8; 5];
                    let n = resp.encode(&mut buf).map_err(ClientError::Encode)?;
                    Ok(buf[..n].to_vec())
                });
                let recorded = mock.share_recorded();
                let mut client = $core::new(mock);
                client.write_multiple_registers_f32(0x01, 0, value)$($await)*?;
                let requests = recorded.lock().unwrap();
                assert_eq!(requests.len(), 1);
                let req = crate::function_codes::write_multiple_registers::WriteMultipleRegistersRequest::decode(
                    &requests[0],
                )
                .map_err(ClientError::Decode)?;
                let expected =
                    crate::helpers::f32_to_registers(value, Endian::Big, WordOrder::MostSignificantFirst);
                let expected_bytes: Vec<u8> = expected.iter().flat_map(|&r| r.to_be_bytes()).collect();
                assert_eq!(req.register_values, expected_bytes);
                Ok(())
            }

            #[$attr]
            $($async)* fn write_multiple_registers_string() -> Result<(), ClientError> {
                let mock = MockAduAdapter::new(move |_, _| {
                    let resp = crate::function_codes::write_multiple_registers::WriteMultipleRegistersResponse {
                        starting_address: 0,
                        quantity: 4,
                    };
                    let mut buf = [0u8; 5];
                    let n = resp.encode(&mut buf).map_err(ClientError::Encode)?;
                    Ok(buf[..n].to_vec())
                });
                let recorded = mock.share_recorded();
                let mut client = $core::new(mock);
                client.write_multiple_registers_string(0x01, 0, "Hello", 4)$($await)*?;
                let requests = recorded.lock().unwrap();
                assert_eq!(requests.len(), 1);
                let req = crate::function_codes::write_multiple_registers::WriteMultipleRegistersRequest::decode(
                    &requests[0],
                )
                .map_err(ClientError::Decode)?;
                let expected =
                    crate::helpers::string_to_registers("Hello", Endian::Big, 4).map_err(ClientError::from)?;
                let expected_bytes: Vec<u8> = expected.iter().flat_map(|&r| r.to_be_bytes()).collect();
                assert_eq!(req.register_values, expected_bytes);
                Ok(())
            }
        };
    }

    #[cfg(all(test, feature = "sync", feature = "helpers"))]
    mod sync_typed_helpers_tests {
        use super::*;
        typed_helpers_tests!([] test, ClientCore, AduAdapter, []);
    }

    #[cfg(all(test, feature = "async", feature = "helpers"))]
    mod async_typed_helpers_tests {
        use super::*;
        typed_helpers_tests!([async] tokio::test, AsyncClientCore, AsyncAduAdapter, [.await]);
    }
}
