//! RTU ADU adapter for the unified client facade.
//!
//! The adapter owns the transport and configuration, frames request PDUs as
//! RTU ADUs, sends them, receives the response, and returns the response PDU.

#![cfg(any(feature = "rtu", feature = "async"))]

use alloc::vec::Vec;

use crate::macros::impl_adu_adapter;

#[cfg(feature = "sync")]
impl_adu_adapter! {
    [] [],
    /// Synchronous RTU ADU adapter.
    "rtu",
    RtuAduAdapter,
    crate::rtu::RtuAdu,
    no_transaction
}

#[cfg(feature = "async")]
impl_adu_adapter! {
    [async] [.await],
    /// Asynchronous RTU ADU adapter.
    "rtu",
    AsyncRtuAduAdapter,
    crate::rtu::RtuAdu,
    no_transaction
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "sync")]
    mod sync_tests {
        use alloc::vec::Vec;
        use core::time::Duration;

        use crate::client::{AduAdapter, ClientConfig};
        use crate::function_codes::read_coils::ReadCoilsRequest;
        use crate::rtu::RtuAdu;
        use crate::transport::{Transport, TransportError};

        struct MockTransport {
            sent: Vec<Vec<u8>>,
            response: Option<Vec<u8>>,
        }

        impl MockTransport {
            fn new() -> Self {
                Self {
                    sent: Vec::new(),
                    response: None,
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
                let data = self.response.take().ok_or(TransportError::Disconnected)?;
                if buf.len() < data.len() {
                    return Err(TransportError::Disconnected);
                }
                buf[..data.len()].copy_from_slice(&data);
                Ok(data.len())
            }
        }

        #[test]
        fn broadcast_skips_recv_and_returns_empty_pdu() {
            use super::super::RtuAduAdapter;

            let request_pdu = {
                let req = ReadCoilsRequest::new(0, 8).unwrap();
                let mut buf = [0u8; 5];
                let n = req.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };

            let mut adapter =
                RtuAduAdapter::with_config(MockTransport::new(), ClientConfig::default());
            let response = adapter.send_receive(0, &request_pdu).unwrap();

            assert!(response.is_empty());
            assert_eq!(adapter.transport.sent.len(), 1);

            let sent = RtuAdu::decode(&adapter.transport.sent[0]).unwrap();
            assert_eq!(sent.address, 0);
            assert_eq!(sent.pdu, request_pdu);
        }

        #[cfg(feature = "metrics")]
        #[test]
        fn metrics_count_request_and_response() {
            use super::super::RtuAduAdapter;
            use crate::metrics::Metrics;
            use alloc::sync::Arc;

            let response_pdu = vec![0x01, 0x01, 0b00001101];
            let mut transport = MockTransport::new();
            let mut adu_buf = [0u8; 32];
            let n = RtuAdu::new(0x01, response_pdu.clone())
                .encode(&mut adu_buf)
                .unwrap();
            transport.response = Some(adu_buf[..n].to_vec());

            let metrics = Arc::new(Metrics::new());
            let mut adapter = RtuAduAdapter::with_config(transport, ClientConfig::default());
            adapter.set_metrics(Arc::clone(&metrics));

            let request_pdu = {
                let req = ReadCoilsRequest::new(0, 8).unwrap();
                let mut buf = [0u8; 5];
                let n = req.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };
            let response = adapter.send_receive(0x01, &request_pdu).unwrap();

            assert_eq!(response, response_pdu);
            assert_eq!(metrics.requests_sent(), 1);
            assert_eq!(metrics.responses_received(), 1);
        }
    }

    #[cfg(feature = "async")]
    mod async_tests {
        use alloc::vec::Vec;
        use core::time::Duration;

        use crate::client::{AsyncAduAdapter, ClientConfig};
        use crate::function_codes::read_coils::ReadCoilsRequest;
        use crate::rtu::RtuAdu;
        use crate::transport::{AsyncTransport, TransportError};

        struct MockAsyncTransport {
            sent: Vec<Vec<u8>>,
        }

        impl MockAsyncTransport {
            fn new() -> Self {
                Self { sent: Vec::new() }
            }
        }

        impl AsyncTransport for MockAsyncTransport {
            async fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
                self.sent.push(data.to_vec());
                Ok(())
            }

            async fn recv(
                &mut self,
                _buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                Err(TransportError::Disconnected)
            }
        }

        #[tokio::test]
        async fn broadcast_skips_recv_and_returns_empty_pdu() {
            use super::super::AsyncRtuAduAdapter;

            let request_pdu = {
                let req = ReadCoilsRequest::new(0, 8).unwrap();
                let mut buf = [0u8; 5];
                let n = req.encode(&mut buf).unwrap();
                buf[..n].to_vec()
            };

            let mut adapter =
                AsyncRtuAduAdapter::with_config(MockAsyncTransport::new(), ClientConfig::default());
            let response = adapter.send_receive(0, &request_pdu).await.unwrap();

            assert!(response.is_empty());
            assert_eq!(adapter.transport.sent.len(), 1);

            let sent = RtuAdu::decode(&adapter.transport.sent[0]).unwrap();
            assert_eq!(sent.address, 0);
            assert_eq!(sent.pdu, request_pdu);
        }
    }
}
