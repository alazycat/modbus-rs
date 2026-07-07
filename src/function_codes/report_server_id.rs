use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x11 Report Server ID.
///
/// The request carries no payload beyond the function code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReportServerIdRequest;

impl ReportServerIdRequest {
    pub const FUNCTION_CODE: u8 = 0x11;

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

/// Response PDU for FC 0x11 Report Server ID.
///
/// Returns the byte count followed by the server-specific identification data.
/// The interpretation of the data (including any run/status byte) is
/// device-specific and left to the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportServerIdResponse {
    pub data: Vec<u8>,
}

impl ReportServerIdResponse {
    /// Create a new response.
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.data.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReportServerIdRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..2 + byte_count].copy_from_slice(&self.data);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReportServerIdRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let data = buf[2..2 + byte_count].to_vec();
        Ok(Self { data })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, [0x11]);

        let decoded = ReportServerIdRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x12];
        assert!(matches!(
            ReportServerIdRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReportServerIdResponse::new(vec![0x01, 0x02, 0x03, 0xFF]);
        let mut buf = [0u8; 6];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(buf, [0x11, 0x04, 0x01, 0x02, 0x03, 0xFF]);

        let decoded = ReportServerIdResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_empty_data() {
        let resp = ReportServerIdResponse::new(vec![]);
        let mut buf = [0u8; 2];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf, [0x11, 0x00]);

        let decoded = ReportServerIdResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x12, 0x02, 0x01, 0x02];
        assert!(matches!(
            ReportServerIdResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_decode_rejects_truncated_byte_count() {
        let buf = [0x11, 0x04, 0x01];
        assert!(matches!(
            ReportServerIdResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReportServerIdRequest;
        let mut buf = [0u8; 0];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
