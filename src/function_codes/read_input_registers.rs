use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x04 Read Input Registers.
///
/// Reads the value of `quantity` contiguous input registers starting at
/// `starting_address`. Registers are addressed from zero in the PDU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadInputRegistersRequest {
    pub starting_address: u16,
    pub quantity: u16,
}

impl ReadInputRegistersRequest {
    pub const FUNCTION_CODE: u8 = 0x04;
    pub const MIN_QUANTITY: u16 = 1;
    pub const MAX_QUANTITY: u16 = 125;

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

/// Response PDU for FC 0x04 Read Input Registers.
///
/// Register values are stored as big-endian bytes: each register occupies
/// two bytes in `register_values`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadInputRegistersResponse {
    pub register_values: Vec<u8>,
}

impl ReadInputRegistersResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.register_values.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if !byte_count.is_multiple_of(2) {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadInputRegistersRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..2 + byte_count].copy_from_slice(&self.register_values);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadInputRegistersRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if !byte_count.is_multiple_of(2) {
            return Err(DecodeError::InvalidLength);
        }
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let register_values = buf[2..2 + byte_count].to_vec();
        Ok(Self { register_values })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReadInputRegistersRequest::new(0x0008, 2).unwrap();
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x04, 0x00, 0x08, 0x00, 0x02]);

        let decoded = ReadInputRegistersRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        assert!(ReadInputRegistersRequest::new(0, 0).is_err());
        assert!(ReadInputRegistersRequest::new(0, 1).is_ok());
        assert!(ReadInputRegistersRequest::new(0, 125).is_ok());
        assert!(ReadInputRegistersRequest::new(0, 126).is_err());
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x03, 0x00, 0x08, 0x00, 0x02];
        assert!(matches!(
            ReadInputRegistersRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReadInputRegistersResponse {
            register_values: vec![0x00, 0x0A, 0x00, 0x0B],
        };
        let mut buf = [0u8; 6];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(buf, [0x04, 0x04, 0x00, 0x0A, 0x00, 0x0B]);

        let decoded = ReadInputRegistersResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_rejects_odd_byte_count() {
        let resp = ReadInputRegistersResponse {
            register_values: vec![0x00, 0x0A, 0x00],
        };
        let mut buf = [0u8; 8];
        assert!(matches!(
            resp.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));

        let buf = [0x04, 0x03, 0x00, 0x0A, 0x00];
        assert!(matches!(
            ReadInputRegistersResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x03, 0x02, 0x00, 0x00];
        assert!(matches!(
            ReadInputRegistersResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadInputRegistersRequest::new(0, 1).unwrap();
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
