//! Request/response hooks for server middleware.

use core::fmt::Debug;

use crate::exception::ExceptionResponse;

/// A hook invoked around server request dispatch.
///
/// Hooks are synchronous and `Send` so they can be reused across the sync and
/// async server paths. A server holds at most one hook; compose multiple hooks
/// inside a single implementation if needed.
pub trait RequestHook: Debug + Send {
    /// Called before a request PDU is dispatched.
    ///
    /// Returning `Err` short-circuits dispatch and produces an exception
    /// response instead of calling into the [`DataStore`](super::DataStore).
    fn before_request(
        &mut self,
        unit_id: u8,
        request_pdu: &[u8],
    ) -> Result<(), ExceptionResponse>;

    /// Called after a response PDU has been produced.
    ///
    /// This is invoked for both successful responses and responses produced by
    /// a `before_request` rejection. It is not called if encoding fails.
    fn after_response(&mut self, unit_id: u8, request_pdu: &[u8], response_pdu: &[u8]);
}

/// A no-op hook that never rejects and does nothing in `after_response`.
#[derive(Debug)]
pub struct NoopHook;

impl RequestHook for NoopHook {
    fn before_request(
        &mut self,
        _unit_id: u8,
        _request_pdu: &[u8],
    ) -> Result<(), ExceptionResponse> {
        Ok(())
    }

    fn after_response(&mut self, _unit_id: u8, _request_pdu: &[u8], _response_pdu: &[u8]) {}
}
