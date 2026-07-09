//! Runtime metrics for Modbus clients and servers.
//!
//! This module is enabled by the `metrics` feature. The [`Metrics`] struct holds
//! counters for requests, responses, errors, and retries. Counters are updated
//! atomically so a single [`Metrics`] instance can be shared across tasks or
//! threads.

#![cfg(feature = "metrics")]

use core::sync::atomic::{AtomicU64, Ordering};

/// Counters that track Modbus client/server activity.
#[derive(Debug, Default)]
pub struct Metrics {
    requests_sent: AtomicU64,
    responses_received: AtomicU64,
    requests_received: AtomicU64,
    responses_sent: AtomicU64,
    errors: AtomicU64,
    retries: AtomicU64,
}

impl Metrics {
    /// Create a new, zero-initialized metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return the number of request ADUs sent by a client.
    pub fn requests_sent(&self) -> u64 {
        self.requests_sent.load(Ordering::Relaxed)
    }

    /// Return the number of response ADUs received by a client.
    pub fn responses_received(&self) -> u64 {
        self.responses_received.load(Ordering::Relaxed)
    }

    /// Return the number of request PDUs received by a server.
    pub fn requests_received(&self) -> u64 {
        self.requests_received.load(Ordering::Relaxed)
    }

    /// Return the number of response PDUs sent by a server.
    pub fn responses_sent(&self) -> u64 {
        self.responses_sent.load(Ordering::Relaxed)
    }

    /// Return the number of errors encountered by the client or server.
    pub fn errors(&self) -> u64 {
        self.errors.load(Ordering::Relaxed)
    }

    /// Return the number of retry attempts made by a retry adapter.
    pub fn retries(&self) -> u64 {
        self.retries.load(Ordering::Relaxed)
    }

    pub(crate) fn record_request_sent(&self) {
        self.requests_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_response_received(&self) {
        self.responses_received.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_request_received(&self) {
        self.requests_received.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_response_sent(&self) {
        self.responses_sent.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn record_retry(&self) {
        self.retries.fetch_add(1, Ordering::Relaxed);
    }
}

impl Clone for Metrics {
    fn clone(&self) -> Self {
        Self {
            requests_sent: AtomicU64::new(self.requests_sent.load(Ordering::Relaxed)),
            responses_received: AtomicU64::new(self.responses_received.load(Ordering::Relaxed)),
            requests_received: AtomicU64::new(self.requests_received.load(Ordering::Relaxed)),
            responses_sent: AtomicU64::new(self.responses_sent.load(Ordering::Relaxed)),
            errors: AtomicU64::new(self.errors.load(Ordering::Relaxed)),
            retries: AtomicU64::new(self.retries.load(Ordering::Relaxed)),
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::sync::Arc;
    use super::*;

    #[test]
    fn counters_start_at_zero() {
        let metrics = Metrics::new();
        assert_eq!(metrics.requests_sent(), 0);
        assert_eq!(metrics.responses_received(), 0);
        assert_eq!(metrics.requests_received(), 0);
        assert_eq!(metrics.responses_sent(), 0);
        assert_eq!(metrics.errors(), 0);
        assert_eq!(metrics.retries(), 0);
    }

    #[test]
    fn record_increments_counters() {
        let metrics = Metrics::new();
        metrics.record_request_sent();
        metrics.record_response_received();
        metrics.record_request_received();
        metrics.record_response_sent();
        metrics.record_error();
        metrics.record_retry();

        assert_eq!(metrics.requests_sent(), 1);
        assert_eq!(metrics.responses_received(), 1);
        assert_eq!(metrics.requests_received(), 1);
        assert_eq!(metrics.responses_sent(), 1);
        assert_eq!(metrics.errors(), 1);
        assert_eq!(metrics.retries(), 1);
    }

    #[test]
    fn clone_copies_counts() {
        let metrics = Metrics::new();
        metrics.record_request_sent();
        let cloned = metrics.clone();
        assert_eq!(cloned.requests_sent(), 1);
    }

    #[test]
    fn arc_wrapping() {
        let metrics = Arc::new(Metrics::new());
        metrics.record_response_received();
        assert_eq!(metrics.responses_received(), 1);
    }
}
