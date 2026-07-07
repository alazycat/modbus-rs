use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x0F Write Multiple Coils.
///
/// Writes `quantity` contiguous coils starting at `starting_address`.
/// Coil values are packed one bit per byte, LSB of the first byte contains
/// the coil at `starting_address`. The final byte is zero-padded when the
/// coil count is not a multiple of eight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteMultipleCoilsRequest {
    pub starting_address: u16,
    pub quantity: u16,
    pub outputs: Vec<u8>,
}

impl WriteMultipleCoilsRequest {
    pub const FUNCTION_CODE: u8 = 0x0F;
    pub const MIN_QUANTITY: u16 = 1;
    pub const MAX_QUANTITY: u16 = 0x07B0; // 1968

    /// Create a new request, validating quantity and output length.
    pub fn new(
        starting_address: u16,
        quantity: u16,
        outputs: Vec<u8>,
    ) -> Result<Self, DecodeError> {
        if !(Self::MIN_QUANTITY..=Self::MAX_QUANTITY).contains(&quantity) {
            return Err(DecodeError::InvalidQuantity);
        }
        let expected_bytes = (quantity as usize).div_ceil(8);
        if outputs.len() != expected_bytes {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self {
            starting_address,
            quantity,
            outputs,
        })
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.outputs.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 6 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.quantity.to_be_bytes());
        buf[5] = byte_count as u8;
        buf[6..6 + byte_count].copy_from_slice(&self.outputs);
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
        let outputs = buf[6..6 + byte_count].to_vec();
        Self::new(starting_address, quantity, outputs)
    }
}

/// Response PDU for FC 0x0F Write Multiple Coils.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteMultipleCoilsResponse {
    pub starting_address: u16,
    pub quantity: u16,
}

impl WriteMultipleCoilsResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = WriteMultipleCoilsRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.quantity.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != WriteMultipleCoilsRequest::FUNCTION_CODE {
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
        // Modbus spec example: 10 coils starting at 19, values 0xCD 0x01
        let outputs = vec![0xCD, 0x01];
        let req = WriteMultipleCoilsRequest::new(0x0013, 10, outputs.clone()).unwrap();
        let mut buf = [0u8; 8];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(buf, [0x0F, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD, 0x01]);

        let decoded = WriteMultipleCoilsRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        let one = vec![0x01];
        assert!(WriteMultipleCoilsRequest::new(0, 0, one.clone()).is_err());
        assert!(WriteMultipleCoilsRequest::new(0, 1, one.clone()).is_ok());
        assert!(WriteMultipleCoilsRequest::new(0, 1968, vec![0u8; 246]).is_ok());
        assert!(WriteMultipleCoilsRequest::new(0, 1969, vec![0u8; 247]).is_err());
    }

    #[test]
    fn request_rejects_mismatched_output_length() {
        // 10 coils need 2 bytes, not 1
        assert!(WriteMultipleCoilsRequest::new(0, 10, vec![0xCD]).is_err());
        // 8 coils need 1 byte, not 2
        assert!(WriteMultipleCoilsRequest::new(0, 8, vec![0xCD, 0x00]).is_err());
    }

    #[test]
    fn request_final_byte_zero_padded() {
        // 11 coils -> 2 bytes; second byte's high bits are padding
        let outputs = vec![0xCD, 0x01];
        let req = WriteMultipleCoilsRequest::new(0, 11, outputs).unwrap();
        let mut buf = [0u8; 8];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(buf[5], 2);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x00, 0x13, 0x00, 0x0A, 0x02, 0xCD, 0x01];
        assert!(matches!(
            WriteMultipleCoilsRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = WriteMultipleCoilsResponse {
            starting_address: 0x0013,
            quantity: 10,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x0F, 0x00, 0x13, 0x00, 0x0A]);

        let decoded = WriteMultipleCoilsResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x01, 0x00, 0x13, 0x00, 0x0A];
        assert!(matches!(
            WriteMultipleCoilsResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = WriteMultipleCoilsRequest::new(0, 1, vec![0x01]).unwrap();
        let mut buf = [0u8; 5];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
