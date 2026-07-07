use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x0C Get Comm Event Log.
///
/// The request carries no payload beyond the function code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCommEventLogRequest;

impl GetCommEventLogRequest {
    pub const FUNCTION_CODE: u8 = 0x0C;

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.is_empty() {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        Ok(1)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.is_empty() {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        Ok(Self)
    }
}

/// Response PDU for FC 0x0C Get Comm Event Log.
///
/// Returns a status word, event count, message count, and up to 64 bytes of
/// event data. The byte count field equals 6 plus the number of event bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetCommEventLogResponse {
    pub status: u16,
    pub event_count: u16,
    pub message_count: u16,
    pub events: Vec<u8>,
}

impl GetCommEventLogResponse {
    pub const MAX_EVENTS: usize = 64;

    /// Create a new response, validating event length.
    pub fn new(
        status: u16,
        event_count: u16,
        message_count: u16,
        events: Vec<u8>,
    ) -> Result<Self, DecodeError> {
        if events.len() > Self::MAX_EVENTS {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self {
            status,
            event_count,
            message_count,
            events,
        })
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = 6 + self.events.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = GetCommEventLogRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..4].copy_from_slice(&self.status.to_be_bytes());
        buf[4..6].copy_from_slice(&self.event_count.to_be_bytes());
        buf[6..8].copy_from_slice(&self.message_count.to_be_bytes());
        buf[8..8 + self.events.len()].copy_from_slice(&self.events);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 8 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != GetCommEventLogRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if byte_count < 6 {
            return Err(DecodeError::InvalidLength);
        }
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let status = u16::from_be_bytes([buf[2], buf[3]]);
        let event_count = u16::from_be_bytes([buf[4], buf[5]]);
        let message_count = u16::from_be_bytes([buf[6], buf[7]]);
        let events = buf[8..2 + byte_count].to_vec();
        Self::new(status, event_count, message_count, events)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, [0x0C]);

        let decoded = GetCommEventLogRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x0B];
        assert!(matches!(
            GetCommEventLogRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let events = vec![0x20, 0x00];
        let resp = GetCommEventLogResponse::new(
            0x00FF,
            0x0001,
            0x0120,
            events.clone(),
        )
        .unwrap();
        let mut buf = [0u8; 10];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 10);
        assert_eq!(buf, [0x0C, 0x08, 0x00, 0xFF, 0x00, 0x01, 0x01, 0x20, 0x20, 0x00]);

        let decoded = GetCommEventLogResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_empty_events() {
        let resp = GetCommEventLogResponse::new(0, 0, 0, vec![]).unwrap();
        let mut buf = [0u8; 8];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(buf, [0x0C, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);

        let decoded = GetCommEventLogResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_rejects_too_many_events() {
        assert!(
            GetCommEventLogResponse::new(0, 0, 0, vec![0u8; 65]).is_err()
        );
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x0B, 0x08, 0x00, 0xFF, 0x00, 0x01, 0x01, 0x20, 0x20, 0x00];
        assert!(matches!(
            GetCommEventLogResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_decode_rejects_truncated_byte_count() {
        let buf = [0x0C, 0x08, 0x00, 0xFF, 0x00, 0x01, 0x01];
        assert!(matches!(
            GetCommEventLogResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = GetCommEventLogRequest;
        let mut buf = [0u8; 0];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
