use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x06 Write Single Register.
///
/// Writes `value` to the holding register at `register_address`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteSingleRegisterRequest {
    pub register_address: u16,
    pub value: u16,
}

impl WriteSingleRegisterRequest {
    pub const FUNCTION_CODE: u8 = 0x06;

    /// Create a new request.
    pub fn new(register_address: u16, value: u16) -> Self {
        Self {
            register_address,
            value,
        }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.register_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.value.to_be_bytes());
        Ok(5)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let register_address = u16::from_be_bytes([buf[1], buf[2]]);
        let value = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self::new(register_address, value))
    }
}

/// Response PDU for FC 0x06 Write Single Register.
///
/// The response echoes the request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteSingleRegisterResponse {
    pub register_address: u16,
    pub value: u16,
}

impl WriteSingleRegisterResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = WriteSingleRegisterRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.register_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.value.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != WriteSingleRegisterRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let register_address = u16::from_be_bytes([buf[1], buf[2]]);
        let value = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self {
            register_address,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = WriteSingleRegisterRequest::new(0x0001, 0x0003);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x06, 0x00, 0x01, 0x00, 0x03]);

        let decoded = WriteSingleRegisterRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x05, 0x00, 0x01, 0x00, 0x03];
        assert!(matches!(
            WriteSingleRegisterRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = WriteSingleRegisterResponse {
            register_address: 0x0001,
            value: 0x0003,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x06, 0x00, 0x01, 0x00, 0x03]);

        let decoded = WriteSingleRegisterResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x05, 0x00, 0x01, 0x00, 0x03];
        assert!(matches!(
            WriteSingleRegisterResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = WriteSingleRegisterRequest::new(0, 0);
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
