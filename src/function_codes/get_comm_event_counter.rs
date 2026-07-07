use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x0B Get Comm Event Counter.
///
/// The request carries no payload beyond the function code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCommEventCounterRequest;

impl GetCommEventCounterRequest {
    pub const FUNCTION_CODE: u8 = 0x0B;

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

/// Response PDU for FC 0x0B Get Comm Event Counter.
///
/// Returns a 16-bit status word and a 16-bit event count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GetCommEventCounterResponse {
    pub status: u16,
    pub event_count: u16,
}

impl GetCommEventCounterResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = GetCommEventCounterRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.status.to_be_bytes());
        buf[3..5].copy_from_slice(&self.event_count.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != GetCommEventCounterRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let status = u16::from_be_bytes([buf[1], buf[2]]);
        let event_count = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self {
            status,
            event_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, [0x0B]);

        let decoded = GetCommEventCounterRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x0C];
        assert!(matches!(
            GetCommEventCounterRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = GetCommEventCounterResponse {
            status: 0xFFFF,
            event_count: 0x1234,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x0B, 0xFF, 0xFF, 0x12, 0x34]);

        let decoded = GetCommEventCounterResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x0C, 0xFF, 0xFF, 0x12, 0x34];
        assert!(matches!(
            GetCommEventCounterResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = GetCommEventCounterRequest;
        let mut buf = [0u8; 0];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
