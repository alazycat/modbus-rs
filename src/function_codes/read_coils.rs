use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x01 Read Coils.
///
/// Reads the status of `quantity` contiguous coils starting at
/// `starting_address`. Coils are addressed from zero in the PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadCoilsRequest {
    pub starting_address: u16,
    pub quantity: u16,
}

impl ReadCoilsRequest {
    pub const FUNCTION_CODE: u8 = 0x01;
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
        if buf.len() < 5 || buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::InvalidLength);
        }
        let starting_address = u16::from_be_bytes([buf[1], buf[2]]);
        let quantity = u16::from_be_bytes([buf[3], buf[4]]);
        Self::new(starting_address, quantity)
    }
}

/// Response PDU for FC 0x01 Read Coils.
///
/// Coil status is packed one bit per byte, LSB of the first byte contains
/// the coil at `starting_address`. If the coil count is not a multiple of
/// eight, the remaining high-order bits of the final byte are zero-filled.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadCoilsResponse {
    pub coil_status: Vec<u8>,
}

impl ReadCoilsResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.coil_status.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadCoilsRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..2 + byte_count].copy_from_slice(&self.coil_status);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadCoilsRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let coil_status = buf[2..2 + byte_count].to_vec();
        Ok(Self { coil_status })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReadCoilsRequest::new(0x0013, 0x0013).unwrap();
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x01, 0x00, 0x13, 0x00, 0x13]);

        let decoded = ReadCoilsRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        assert!(ReadCoilsRequest::new(0, 0).is_err());
        assert!(ReadCoilsRequest::new(0, 1).is_ok());
        assert!(ReadCoilsRequest::new(0, 2000).is_ok());
        assert!(ReadCoilsRequest::new(0, 2001).is_err());
    }

    #[test]
    fn response_roundtrip() {
        // Example from Modbus spec: outputs 27-20 = 0xCD, 35-28 = 0x6B, 38-36 = 0x05
        let resp = ReadCoilsResponse {
            coil_status: vec![0xCD, 0x6B, 0x05],
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x01, 0x03, 0xCD, 0x6B, 0x05]);

        let decoded = ReadCoilsResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x02, 0x01, 0x00];
        assert!(matches!(
            ReadCoilsResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadCoilsRequest::new(0, 1).unwrap();
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
