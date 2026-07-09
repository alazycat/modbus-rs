//! Retry adapters for synchronous and asynchronous ADU adapters.
//!
//! These adapters wrap an existing ADU adapter and reconnect on transient
//! failures according to a [`RetryPolicy`]. They implement the same
//! `AduAdapter` / `AsyncAduAdapter` traits as the underlying adapter, so they
//! can be used anywhere a regular adapter is expected.

#![cfg(any(feature = "sync", feature = "async"))]

use crate::client::{ClientError, RetryPolicy};

/// A synchronous retry adapter.
#[cfg(feature = "sync")]
#[derive(Debug)]
pub struct RetryAdapter<A, F> {
    adapter: A,
    factory: F,
    policy: RetryPolicy,
}

#[cfg(feature = "sync")]
impl<A, F> RetryAdapter<A, F> {
    /// Create a retry adapter around `adapter`.
    ///
    /// `factory` is called to recreate the adapter after a disconnect. It must
    /// return a fresh adapter ready for I/O.
    pub fn new(adapter: A, factory: F, policy: RetryPolicy) -> Self {
        Self {
            adapter,
            factory,
            policy,
        }
    }
}

#[cfg(feature = "sync")]
impl<A, F> crate::client::AduAdapter for RetryAdapter<A, F>
where
    A: crate::client::AduAdapter,
    F: FnMut() -> Result<A, ClientError>,
{
    fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError> {
        let mut attempts = 0u32;
        let mut backoff = self.policy.initial_backoff;

        loop {
            match self.adapter.send_receive(unit_id, request_pdu) {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if attempts >= self.policy.max_retries || !(self.policy.retryable)(&err) {
                        return Err(err);
                    }
                    attempts += 1;
                    std::thread::sleep(backoff);
                    self.adapter = (self.factory)()?;
                    backoff = backoff
                        .saturating_add(backoff)
                        .min(self.policy.max_backoff);
                }
            }
        }
    }
}

/// An asynchronous retry adapter.
#[cfg(feature = "async")]
#[derive(Debug)]
pub struct AsyncRetryAdapter<A, F> {
    adapter: A,
    factory: F,
    policy: RetryPolicy,
}

#[cfg(feature = "async")]
impl<A, F> AsyncRetryAdapter<A, F> {
    /// Create an async retry adapter around `adapter`.
    ///
    /// `factory` is called to recreate the adapter after a disconnect. It must
    /// return a fresh adapter ready for I/O.
    pub fn new(adapter: A, factory: F, policy: RetryPolicy) -> Self {
        Self {
            adapter,
            factory,
            policy,
        }
    }
}

#[cfg(feature = "async")]
impl<A, F, Fut> crate::client::AsyncAduAdapter for AsyncRetryAdapter<A, F>
where
    A: crate::client::AsyncAduAdapter,
    F: FnMut() -> Fut,
    Fut: core::future::Future<Output = Result<A, ClientError>>,
{
    async fn send_receive(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<alloc::vec::Vec<u8>, ClientError> {
        let mut attempts = 0u32;
        let mut backoff = self.policy.initial_backoff;

        loop {
            match self.adapter.send_receive(unit_id, request_pdu).await {
                Ok(response) => return Ok(response),
                Err(err) => {
                    if attempts >= self.policy.max_retries || !(self.policy.retryable)(&err) {
                        return Err(err);
                    }
                    attempts += 1;
                    tokio::time::sleep(backoff).await;
                    self.adapter = (self.factory)().await?;
                    backoff = backoff
                        .saturating_add(backoff)
                        .min(self.policy.max_backoff);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "sync")]
    mod sync_tests {
        use super::*;
        use crate::client::AduAdapter;
        use crate::transport::TransportError;
        use core::time::Duration;

        struct MockAdapter {
            responses: alloc::vec::Vec<Result<alloc::vec::Vec<u8>, ClientError>>,
        }

        impl MockAdapter {
            fn new(responses: alloc::vec::Vec<Result<alloc::vec::Vec<u8>, ClientError>>) -> Self {
                Self { responses }
            }
        }

        impl AduAdapter for MockAdapter {
            fn send_receive(
                &mut self,
                _unit_id: u8,
                _request_pdu: &[u8],
            ) -> Result<alloc::vec::Vec<u8>, ClientError> {
                self.responses.remove(0)
            }
        }

        #[test]
        fn succeeds_without_retry() {
            let adapter = MockAdapter::new(alloc::vec![Ok(alloc::vec![0x03, 0x02, 0x00, 0x0A])]);
            let mut retry = RetryAdapter::new(
                adapter,
                || -> Result<MockAdapter, ClientError> { unreachable!("factory should not be called") },
                RetryPolicy::default(),
            );

            let response = retry.send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A]).unwrap();
            assert_eq!(response, alloc::vec![0x03, 0x02, 0x00, 0x0A]);
        }

        #[test]
        fn retries_on_disconnected_and_reconnects() {
            let mut retry = RetryAdapter::new(
                MockAdapter::new(alloc::vec![Err(ClientError::Transport(
                    TransportError::Disconnected,
                ))]),
                || {
                    Ok(MockAdapter::new(alloc::vec![Ok(alloc::vec![
                        0x03, 0x02, 0x00, 0x0A,
                    ])]))
                },
                RetryPolicy {
                    max_retries: 2,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let response = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .unwrap();
            assert_eq!(response, alloc::vec![0x03, 0x02, 0x00, 0x0A]);
        }

        #[test]
        fn gives_up_after_max_retries() {
            let mut call_count = 0usize;
            let mut retry = RetryAdapter::new(
                MockAdapter::new(alloc::vec![Err(ClientError::Transport(
                    TransportError::Disconnected,
                ))]),
                || {
                    call_count += 1;
                    Ok(MockAdapter::new(alloc::vec![Err(ClientError::Transport(
                        TransportError::Disconnected,
                    ))]))
                },
                RetryPolicy {
                    max_retries: 2,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let err = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .unwrap_err();
            assert!(matches!(err, ClientError::Transport(TransportError::Disconnected)));
            // Initial failure + 2 retries = 3 attempts total.
            assert_eq!(call_count, 2);
        }

        #[test]
        fn non_retryable_error_propagates_immediately() {
            let mut call_count = 0usize;
            let mut retry = RetryAdapter::new(
                MockAdapter::new(alloc::vec![Err(ClientError::InvalidResponse)]),
                || {
                    call_count += 1;
                    unreachable!("factory should not be called for non-retryable errors")
                },
                RetryPolicy {
                    max_retries: 3,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let err = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .unwrap_err();
            assert!(matches!(err, ClientError::InvalidResponse));
            assert_eq!(call_count, 0);
        }
    }

    #[cfg(feature = "async")]
    mod async_tests {
        use super::*;
        use crate::client::AsyncAduAdapter;
        use crate::transport::TransportError;
        use alloc::sync::Arc;
        use core::sync::atomic::{AtomicUsize, Ordering};
        use core::time::Duration;

        struct MockAsyncAdapter {
            responses: alloc::vec::Vec<Result<alloc::vec::Vec<u8>, ClientError>>,
        }

        impl MockAsyncAdapter {
            fn new(responses: alloc::vec::Vec<Result<alloc::vec::Vec<u8>, ClientError>>) -> Self {
                Self { responses }
            }
        }

        impl AsyncAduAdapter for MockAsyncAdapter {
            async fn send_receive(
                &mut self,
                _unit_id: u8,
                _request_pdu: &[u8],
            ) -> Result<alloc::vec::Vec<u8>, ClientError> {
                self.responses.remove(0)
            }
        }

        #[tokio::test]
        async fn succeeds_without_retry() {
            let adapter = MockAsyncAdapter::new(alloc::vec![Ok(alloc::vec![0x03, 0x02, 0x00, 0x0A])]);
            let mut retry = AsyncRetryAdapter::new(
                adapter,
                || async { unreachable!("factory should not be called") },
                RetryPolicy::default(),
            );

            let response = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .await
                .unwrap();
            assert_eq!(response, alloc::vec![0x03, 0x02, 0x00, 0x0A]);
        }

        #[tokio::test]
        async fn retries_on_disconnected_and_reconnects() {
            let mut retry = AsyncRetryAdapter::new(
                MockAsyncAdapter::new(alloc::vec![Err(ClientError::Transport(
                    TransportError::Disconnected,
                ))]),
                || async {
                    Ok(MockAsyncAdapter::new(alloc::vec![Ok(alloc::vec![
                        0x03, 0x02, 0x00, 0x0A,
                    ])]))
                },
                RetryPolicy {
                    max_retries: 2,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let response = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .await
                .unwrap();
            assert_eq!(response, alloc::vec![0x03, 0x02, 0x00, 0x0A]);
        }

        #[tokio::test]
        async fn gives_up_after_max_retries() {
            let call_count = Arc::new(AtomicUsize::new(0));
            let mut retry = AsyncRetryAdapter::new(
                MockAsyncAdapter::new(alloc::vec![Err(ClientError::Transport(
                    TransportError::Disconnected,
                ))]),
                {
                    let call_count = Arc::clone(&call_count);
                    move || {
                        let call_count = Arc::clone(&call_count);
                        async move {
                            call_count.fetch_add(1, Ordering::SeqCst);
                            Ok(MockAsyncAdapter::new(alloc::vec![Err(ClientError::Transport(
                                TransportError::Disconnected,
                            ))]))
                        }
                    }
                },
                RetryPolicy {
                    max_retries: 2,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let err = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .await
                .unwrap_err();
            assert!(matches!(
                err,
                ClientError::Transport(TransportError::Disconnected)
            ));
            assert_eq!(call_count.load(Ordering::SeqCst), 2);
        }

        #[tokio::test]
        async fn non_retryable_error_propagates_immediately() {
            let call_count = Arc::new(AtomicUsize::new(0));
            let mut retry = AsyncRetryAdapter::new(
                MockAsyncAdapter::new(alloc::vec![Err(ClientError::InvalidResponse)]),
                {
                    let call_count = Arc::clone(&call_count);
                    move || {
                        let call_count = Arc::clone(&call_count);
                        async move {
                            call_count.fetch_add(1, Ordering::SeqCst);
                            unreachable!("factory should not be called for non-retryable errors")
                        }
                    }
                },
                RetryPolicy {
                    max_retries: 3,
                    initial_backoff: Duration::ZERO,
                    max_backoff: Duration::ZERO,
                    ..RetryPolicy::default()
                },
            );

            let err = retry
                .send_receive(0x01, &[0x03, 0x00, 0x00, 0x00, 0x0A])
                .await
                .unwrap_err();
            assert!(matches!(err, ClientError::InvalidResponse));
            assert_eq!(call_count.load(Ordering::SeqCst), 0);
        }
    }
}
