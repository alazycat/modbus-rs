use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x02 Read Discrete Inputs.
///
/// Reads the status of `quantity` contiguous discrete inputs starting at
/// `starting_address`. Discrete inputs are addressed from zero in the PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadDiscreteInputsRequest {
    pub starting_address: u16,
    pub quantity: u16,
}

impl ReadDiscreteInputsRequest {
    pub const FUNCTION_CODE: u8 = 0x02;
    pub const MIN_QUANTITY: u16 = 1;
    pub const MAX_QUANTITY: u16 = 2000;

    /// Create a new request, validating the quantity.
    pub fn new(starting_address: u16, quantity: u16) -> Result<Self, DecodeError> {
        if !(Self::MIN_QUANTITY..=Self::MAX_QUANTITY).contains(&quantity) {
            return Err(DecodeError::InvalidQuantity);
        }
        Ok(Self {
            starting_address,
            quantity,
        })
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.quantity.to_be_bytes());
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
        let starting_address = u16::from_be_bytes([buf[1], buf[2]]);
        let quantity = u16::from_be_bytes([buf[3], buf[4]]);
        Self::new(starting_address, quantity)
    }
}

/// Response PDU for FC 0x02 Read Discrete Inputs.
///
/// Input status is packed one bit per byte, LSB of the first byte contains
/// the input at `starting_address`. If the input count is not a multiple of
/// eight, the remaining high-order bits of the final byte are zero-filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadDiscreteInputsResponse {
    pub input_status: Vec<u8>,
}

impl ReadDiscreteInputsResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.input_status.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadDiscreteInputsRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..2 + byte_count].copy_from_slice(&self.input_status);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadDiscreteInputsRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let input_status = buf[2..2 + byte_count].to_vec();
        Ok(Self { input_status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReadDiscreteInputsRequest::new(0x00C4, 0x0016).unwrap();
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x02, 0x00, 0xC4, 0x00, 0x16]);

        let decoded = ReadDiscreteInputsRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        assert!(ReadDiscreteInputsRequest::new(0, 0).is_err());
        assert!(ReadDiscreteInputsRequest::new(0, 1).is_ok());
        assert!(ReadDiscreteInputsRequest::new(0, 2000).is_ok());
        assert!(ReadDiscreteInputsRequest::new(0, 2001).is_err());
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x00, 0xC4, 0x00, 0x16];
        assert!(matches!(
            ReadDiscreteInputsRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReadDiscreteInputsResponse {
            input_status: vec![0xAC, 0xDB, 0x35],
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x02, 0x03, 0xAC, 0xDB, 0x35]);

        let decoded = ReadDiscreteInputsResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x01, 0x00];
        assert!(matches!(
            ReadDiscreteInputsResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadDiscreteInputsRequest::new(0, 1).unwrap();
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
