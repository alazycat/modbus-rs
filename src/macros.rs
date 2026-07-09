//! Macros used internally by the Modbus crate.
//!
//! Client-specific method generation macros live in [`crate::client::macros`].
//! This file keeps macros that are shared across transports, such as the
//! ADU adapter boilerplate used by both sync and async clients.

/// Generate the sync/async ADU adapter boilerplate for a Modbus protocol.
///
/// The macro emits the adapter struct, constructors, and the `AduAdapter`/
/// `AsyncAduAdapter` implementation. It supports two protocol shapes:
/// - `transaction` for TCP/UDP (MBAP header with transaction ID)
/// - `no_transaction` for RTU/ASCII (address field in ADU)
///
/// Example for RTU (sync):
///
/// ```ignore
/// impl_adu_adapter! {
///     [] [],
///     /// Synchronous RTU ADU adapter.
///     "rtu",
///     RtuAduAdapter,
///     crate::rtu::RtuAdu,
///     no_transaction
/// }
/// ```
///
/// Example for TCP (async):
///
/// ```ignore
/// impl_adu_adapter! {
///     [async] [.await],
///     /// Asynchronous TCP ADU adapter.
///     "tcp",
///     AsyncTcpAduAdapter,
///     crate::tcp::TcpAdu,
///     transaction
/// }
/// ```
#[macro_export]
macro_rules! impl_adu_adapter {
    // Public arms that select the sync or async transport/ADU traits.
    ([] [], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, no_transaction) => {
        impl_adu_adapter! {
            @internal [] [],
            $(#[$meta])* $protocol, $adapter, $adu,
            $crate::transport::Transport,
            $crate::client::AduAdapter,
            no_transaction
        }
    };
    ([async] [.await], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, no_transaction) => {
        impl_adu_adapter! {
            @internal [async] [.await],
            $(#[$meta])* $protocol, $adapter, $adu,
            $crate::transport::AsyncTransport,
            $crate::client::AsyncAduAdapter,
            no_transaction
        }
    };
    ([] [], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, transaction) => {
        impl_adu_adapter! {
            @internal [] [],
            $(#[$meta])* $protocol, $adapter, $adu,
            $crate::transport::Transport,
            $crate::client::AduAdapter,
            transaction
        }
    };
    ([async] [.await], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, transaction) => {
        impl_adu_adapter! {
            @internal [async] [.await],
            $(#[$meta])* $protocol, $adapter, $adu,
            $crate::transport::AsyncTransport,
            $crate::client::AsyncAduAdapter,
            transaction
        }
    };

    // Internal arm: protocol without transaction ID (RTU, ASCII).
    (@internal [$($async:tt)*] [$($await:tt)*], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, $transport:path, $trait:path, no_transaction) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $adapter<T: $transport> {
            transport: T,
            config: $crate::client::ClientConfig,
            #[cfg(feature = "metrics")]
            metrics: Option<alloc::sync::Arc<$crate::metrics::Metrics>>,
        }

        impl<T: $transport> $adapter<T> {
            /// Create an adapter with the default configuration.
            pub fn new(transport: T) -> Self {
                Self::with_config(transport, $crate::client::ClientConfig::default())
            }

            /// Create an adapter with a custom configuration.
            pub fn with_config(transport: T, config: $crate::client::ClientConfig) -> Self {
                Self {
                    transport,
                    config,
                    #[cfg(feature = "metrics")]
                    metrics: None,
                }
            }

            /// Attach a shared [`Metrics`] instance to this adapter.
            #[cfg(feature = "metrics")]
            pub fn set_metrics(&mut self, metrics: alloc::sync::Arc<$crate::metrics::Metrics>) {
                self.metrics = Some(metrics);
            }
        }

        impl<T: $transport> $trait for $adapter<T> {
            $($async)* fn send_receive(
                &mut self,
                unit_id: u8,
                request_pdu: &[u8],
            ) -> Result<Vec<u8>, $crate::client::ClientError> {
                let adu = <$adu>::new(unit_id, request_pdu.to_vec());
                let mut tx = [0u8; 512];
                let n = adu.encode(&mut tx).map_err($crate::client::ClientError::Encode)?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, unit_id, pdu_len = n, "sending ADU");
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_request_sent();
                }
                self.transport.send(&tx[..n]) $($await)* ?;

                // Broadcast requests (unit_id == 0) are sent to all devices and
                // do not produce a response, so skip the receive path.
                if unit_id == 0 {
                    #[cfg(feature = "tracing")]
                    tracing::trace!(protocol = $protocol, unit_id, "broadcast request, skipping receive");
                    return Ok(Vec::new());
                }

                let mut rx = [0u8; 512];
                let m = self.transport.recv(&mut rx, self.config.timeout) $($await)* ?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, unit_id, response_len = m, "received ADU");
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_response_received();
                }
                if m == 0 {
                    return Err($crate::client::ClientError::Transport(
                        $crate::transport::TransportError::Disconnected,
                    ));
                }
                let response = <$adu>::decode(&rx[..m]).map_err($crate::client::ClientError::Decode)?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, unit_id, response_address = response.address, "decoded response ADU");
                if response.address != unit_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.pdu.is_empty() {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                Ok(response.pdu)
            }
        }
    };

    // Internal arm: protocol with transaction ID (TCP, UDP).
    (@internal [$($async:tt)*] [$($await:tt)*], $(#[$meta:meta])* $protocol:literal, $adapter:ident, $adu:ty, $transport:path, $trait:path, transaction) => {
        $(#[$meta])*
        #[derive(Debug)]
        pub struct $adapter<T: $transport> {
            transport: T,
            config: $crate::client::ClientConfig,
            next_transaction_id: u16,
            #[cfg(feature = "metrics")]
            metrics: Option<alloc::sync::Arc<$crate::metrics::Metrics>>,
        }

        impl<T: $transport> $adapter<T> {
            /// Create an adapter with the default configuration.
            pub fn new(transport: T) -> Self {
                Self::with_config(transport, $crate::client::ClientConfig::default())
            }

            /// Create an adapter with a custom configuration.
            pub fn with_config(transport: T, config: $crate::client::ClientConfig) -> Self {
                Self {
                    transport,
                    config,
                    next_transaction_id: 1,
                    #[cfg(feature = "metrics")]
                    metrics: None,
                }
            }

            /// Attach a shared [`Metrics`] instance to this adapter.
            #[cfg(feature = "metrics")]
            pub fn set_metrics(&mut self, metrics: alloc::sync::Arc<$crate::metrics::Metrics>) {
                self.metrics = Some(metrics);
            }
        }

        impl<T: $transport> $trait for $adapter<T> {
            $($async)* fn send_receive(
                &mut self,
                unit_id: u8,
                request_pdu: &[u8],
            ) -> Result<Vec<u8>, $crate::client::ClientError> {
                let transaction_id = self.next_transaction_id;
                self.next_transaction_id = self.next_transaction_id.wrapping_add(1);

                let adu = <$adu>::new(transaction_id, unit_id, request_pdu.to_vec());
                let mut tx = [0u8; 512];
                let n = adu.encode(&mut tx).map_err($crate::client::ClientError::Encode)?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, transaction_id, unit_id, pdu_len = n, "sending ADU");
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_request_sent();
                }
                self.transport.send(&tx[..n]) $($await)* ?;

                let mut rx = [0u8; 512];
                let m = self.transport.recv(&mut rx, self.config.timeout) $($await)* ?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, transaction_id, unit_id, response_len = m, "received ADU");
                #[cfg(feature = "metrics")]
                if let Some(ref metrics) = self.metrics {
                    metrics.record_response_received();
                }
                if m == 0 {
                    return Err($crate::client::ClientError::Transport(
                        $crate::transport::TransportError::Disconnected,
                    ));
                }
                let response = <$adu>::decode(&rx[..m]).map_err($crate::client::ClientError::Decode)?;
                #[cfg(feature = "tracing")]
                tracing::trace!(protocol = $protocol, transaction_id, unit_id, response_transaction_id = response.transaction_id, "decoded response ADU");
                if response.transaction_id != transaction_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.unit_id != unit_id {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                if response.pdu.is_empty() {
                    return Err($crate::client::ClientError::InvalidResponse);
                }
                Ok(response.pdu)
            }
        }
    };
}

// Re-export the macro at the crate root so `crate::impl_adu_adapter!` works.
pub use impl_adu_adapter;
