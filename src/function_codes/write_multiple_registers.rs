use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x10 Write Multiple Registers.
///
/// Writes `quantity` contiguous holding registers starting at
/// `starting_address`. Register values are big-endian, two bytes each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteMultipleRegistersRequest {
    pub starting_address: u16,
    pub quantity: u16,
    pub register_values: Vec<u8>,
}

impl WriteMultipleRegistersRequest {
    pub const FUNCTION_CODE: u8 = 0x10;
    pub const MIN_QUANTITY: u16 = 1;
    pub const MAX_QUANTITY: u16 = 0x007B; // 123

    /// Create a new request, validating quantity and register value length.
    pub fn new(
        starting_address: u16,
        quantity: u16,
        register_values: Vec<u8>,
    ) -> Result<Self, DecodeError> {
        if !(Self::MIN_QUANTITY..=Self::MAX_QUANTITY).contains(&quantity) {
            return Err(DecodeError::InvalidQuantity);
        }
        let expected_bytes = (quantity as usize) * 2;
        if register_values.len() != expected_bytes {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self {
            starting_address,
            quantity,
            register_values,
        })
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.register_values.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if !byte_count.is_multiple_of(2) {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 6 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.quantity.to_be_bytes());
        buf[5] = byte_count as u8;
        buf[6..6 + byte_count].copy_from_slice(&self.register_values);
        Ok(6 + byte_count)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 6 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let starting_address = u16::from_be_bytes([buf[1], buf[2]]);
        let quantity = u16::from_be_bytes([buf[3], buf[4]]);
        let byte_count = buf[5] as usize;
        if buf.len() < 6 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let register_values = buf[6..6 + byte_count].to_vec();
        Self::new(starting_address, quantity, register_values)
    }
}

/// Response PDU for FC 0x10 Write Multiple Registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteMultipleRegistersResponse {
    pub starting_address: u16,
    pub quantity: u16,
}

impl WriteMultipleRegistersResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = WriteMultipleRegistersRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.quantity.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != WriteMultipleRegistersRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let starting_address = u16::from_be_bytes([buf[1], buf[2]]);
        let quantity = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self {
            starting_address,
            quantity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        // Spec example: 2 registers starting at 1, values 0x000A, 0x0102
        let values = vec![0x00, 0x0A, 0x01, 0x02];
        let req = WriteMultipleRegistersRequest::new(0x0001, 2, values.clone()).unwrap();
        let mut buf = [0u8; 10];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 10);
        assert_eq!(buf, [0x10, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02]);

        let decoded = WriteMultipleRegistersRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        let two_bytes = vec![0x00, 0x00];
        assert!(WriteMultipleRegistersRequest::new(0, 0, two_bytes.clone()).is_err());
        assert!(WriteMultipleRegistersRequest::new(0, 1, two_bytes.clone()).is_ok());
        let max_bytes = vec![0u8; 246];
        assert!(WriteMultipleRegistersRequest::new(0, 123, max_bytes.clone()).is_ok());
        let too_many = vec![0u8; 248];
        assert!(WriteMultipleRegistersRequest::new(0, 124, too_many).is_err());
    }

    #[test]
    fn request_rejects_mismatched_value_length() {
        // 2 registers need 4 bytes, not 2
        assert!(WriteMultipleRegistersRequest::new(0, 2, vec![0x00, 0x00]).is_err());
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x01, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02];
        assert!(matches!(
            WriteMultipleRegistersRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = WriteMultipleRegistersResponse {
            starting_address: 0x0001,
            quantity: 2,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x10, 0x00, 0x01, 0x00, 0x02]);

        let decoded = WriteMultipleRegistersResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x01, 0x00, 0x02];
        assert!(matches!(
            WriteMultipleRegistersResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = WriteMultipleRegistersRequest::new(0, 1, vec![0x00, 0x00]).unwrap();
        let mut buf = [0u8; 5];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
