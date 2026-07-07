use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x05 Write Single Coil.
///
/// Writes `value` to the coil at `output_address`. The only valid values
/// are `0x0000` (OFF) and `0xFF00` (ON).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteSingleCoilRequest {
    pub output_address: u16,
    pub value: u16,
}

impl WriteSingleCoilRequest {
    pub const FUNCTION_CODE: u8 = 0x05;
    pub const OFF: u16 = 0x0000;
    pub const ON: u16 = 0xFF00;

    /// Create a new request, validating the coil value.
    pub fn new(output_address: u16, value: u16) -> Result<Self, DecodeError> {
        if value != Self::OFF && value != Self::ON {
            return Err(DecodeError::InvalidValue);
        }
        Ok(Self {
            output_address,
            value,
        })
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.output_address.to_be_bytes());
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
        let output_address = u16::from_be_bytes([buf[1], buf[2]]);
        let value = u16::from_be_bytes([buf[3], buf[4]]);
        Self::new(output_address, value)
    }
}

/// Response PDU for FC 0x05 Write Single Coil.
///
/// The response echoes the request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteSingleCoilResponse {
    pub output_address: u16,
    pub value: u16,
}

impl WriteSingleCoilResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = WriteSingleCoilRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.output_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.value.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != WriteSingleCoilRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let output_address = u16::from_be_bytes([buf[1], buf[2]]);
        let value = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self {
            output_address,
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip_on() {
        let req = WriteSingleCoilRequest::new(0x00AC, WriteSingleCoilRequest::ON).unwrap();
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x05, 0x00, 0xAC, 0xFF, 0x00]);

        let decoded = WriteSingleCoilRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_roundtrip_off() {
        let req = WriteSingleCoilRequest::new(0x00AC, WriteSingleCoilRequest::OFF).unwrap();
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x05, 0x00, 0xAC, 0x00, 0x00]);

        let decoded = WriteSingleCoilRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_rejects_invalid_value() {
        assert!(WriteSingleCoilRequest::new(0, 0x1234).is_err());
        assert!(WriteSingleCoilRequest::new(0, 0xFF01).is_err());
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x00, 0xAC, 0xFF, 0x00];
        assert!(matches!(
            WriteSingleCoilRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = WriteSingleCoilResponse {
            output_address: 0x00AC,
            value: WriteSingleCoilRequest::ON,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x05, 0x00, 0xAC, 0xFF, 0x00]);

        let decoded = WriteSingleCoilResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x00, 0xAC, 0xFF, 0x00];
        assert!(matches!(
            WriteSingleCoilResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = WriteSingleCoilRequest::new(0, WriteSingleCoilRequest::ON).unwrap();
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
