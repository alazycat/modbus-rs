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
    RtuAduAdapter,
    crate::rtu::RtuAdu,
    no_transaction
}

#[cfg(feature = "async")]
impl_adu_adapter! {
    [async] [.await],
    /// Asynchronous RTU ADU adapter.
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
        }

        impl MockTransport {
            fn new() -> Self {
                Self { sent: Vec::new() }
            }
        }

        impl Transport for MockTransport {
            fn send(&mut self, data: &[u8]) -> Result<(), TransportError> {
                self.sent.push(data.to_vec());
                Ok(())
            }

            fn recv(
                &mut self,
                _buf: &mut [u8],
                _timeout: Duration,
            ) -> Result<usize, TransportError> {
                Err(TransportError::Disconnected)
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
