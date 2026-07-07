use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x16 Mask Write Register.
///
/// Modifies the holding register at `reference_address` using:
/// `new_value = (old_value & and_mask) | (or_mask & !and_mask)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskWriteRegisterRequest {
    pub reference_address: u16,
    pub and_mask: u16,
    pub or_mask: u16,
}

impl MaskWriteRegisterRequest {
    pub const FUNCTION_CODE: u8 = 0x16;

    /// Create a new request.
    pub fn new(reference_address: u16, and_mask: u16, or_mask: u16) -> Self {
        Self {
            reference_address,
            and_mask,
            or_mask,
        }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 7 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.reference_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.and_mask.to_be_bytes());
        buf[5..7].copy_from_slice(&self.or_mask.to_be_bytes());
        Ok(7)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 7 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let reference_address = u16::from_be_bytes([buf[1], buf[2]]);
        let and_mask = u16::from_be_bytes([buf[3], buf[4]]);
        let or_mask = u16::from_be_bytes([buf[5], buf[6]]);
        Ok(Self::new(reference_address, and_mask, or_mask))
    }
}

/// Response PDU for FC 0x16 Mask Write Register.
///
/// The response echoes the request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MaskWriteRegisterResponse {
    pub reference_address: u16,
    pub and_mask: u16,
    pub or_mask: u16,
}

impl MaskWriteRegisterResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 7 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = MaskWriteRegisterRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.reference_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.and_mask.to_be_bytes());
        buf[5..7].copy_from_slice(&self.or_mask.to_be_bytes());
        Ok(7)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 7 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != MaskWriteRegisterRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let reference_address = u16::from_be_bytes([buf[1], buf[2]]);
        let and_mask = u16::from_be_bytes([buf[3], buf[4]]);
        let or_mask = u16::from_be_bytes([buf[5], buf[6]]);
        Ok(Self {
            reference_address,
            and_mask,
            or_mask,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = MaskWriteRegisterRequest::new(0x0004, 0x00F2, 0x0025);
        let mut buf = [0u8; 7];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 7);
        assert_eq!(buf, [0x16, 0x00, 0x04, 0x00, 0xF2, 0x00, 0x25]);

        let decoded = MaskWriteRegisterRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x04, 0x00, 0xF2, 0x00, 0x25];
        assert!(matches!(
            MaskWriteRegisterRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = MaskWriteRegisterResponse {
            reference_address: 0x0004,
            and_mask: 0x00F2,
            or_mask: 0x0025,
        };
        let mut buf = [0u8; 7];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 7);
        assert_eq!(buf, [0x16, 0x00, 0x04, 0x00, 0xF2, 0x00, 0x25]);

        let decoded = MaskWriteRegisterResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x04, 0x00, 0xF2, 0x00, 0x25];
        assert!(matches!(
            MaskWriteRegisterResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = MaskWriteRegisterRequest::new(0, 0, 0);
        let mut buf = [0u8; 6];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
