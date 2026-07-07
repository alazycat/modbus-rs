use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x17 Read/Write Multiple Registers.
///
/// Atomically reads `read_quantity` registers starting at
/// `read_starting_address` and writes `write_quantity` registers starting at
/// `write_starting_address`. The write values are big-endian, two bytes each.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadWriteMultipleRegistersRequest {
    pub read_starting_address: u16,
    pub read_quantity: u16,
    pub write_starting_address: u16,
    pub write_quantity: u16,
    pub write_values: Vec<u8>,
}

impl ReadWriteMultipleRegistersRequest {
    pub const FUNCTION_CODE: u8 = 0x17;
    pub const MIN_READ_QUANTITY: u16 = 1;
    pub const MAX_READ_QUANTITY: u16 = 0x007D; // 125
    pub const MIN_WRITE_QUANTITY: u16 = 1;
    pub const MAX_WRITE_QUANTITY: u16 = 0x0079; // 121

    /// Create a new request, validating quantities and write value length.
    pub fn new(
        read_starting_address: u16,
        read_quantity: u16,
        write_starting_address: u16,
        write_quantity: u16,
        write_values: Vec<u8>,
    ) -> Result<Self, DecodeError> {
        if !(Self::MIN_READ_QUANTITY..=Self::MAX_READ_QUANTITY)
            .contains(&read_quantity)
        {
            return Err(DecodeError::InvalidQuantity);
        }
        if !(Self::MIN_WRITE_QUANTITY..=Self::MAX_WRITE_QUANTITY)
            .contains(&write_quantity)
        {
            return Err(DecodeError::InvalidQuantity);
        }
        let expected_bytes = (write_quantity as usize) * 2;
        if write_values.len() != expected_bytes {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self {
            read_starting_address,
            read_quantity,
            write_starting_address,
            write_quantity,
            write_values,
        })
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.write_values.len();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if !byte_count.is_multiple_of(2) {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 9 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.read_starting_address.to_be_bytes());
        buf[3..5].copy_from_slice(&self.read_quantity.to_be_bytes());
        buf[5..7].copy_from_slice(&self.write_starting_address.to_be_bytes());
        buf[7..9].copy_from_slice(&self.write_quantity.to_be_bytes());
        buf[9] = byte_count as u8;
        buf[10..10 + byte_count].copy_from_slice(&self.write_values);
        Ok(10 + byte_count)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 10 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let read_starting_address = u16::from_be_bytes([buf[1], buf[2]]);
        let read_quantity = u16::from_be_bytes([buf[3], buf[4]]);
        let write_starting_address = u16::from_be_bytes([buf[5], buf[6]]);
        let write_quantity = u16::from_be_bytes([buf[7], buf[8]]);
        let byte_count = buf[9] as usize;
        if buf.len() < 10 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let write_values = buf[10..10 + byte_count].to_vec();
        Self::new(
            read_starting_address,
            read_quantity,
            write_starting_address,
            write_quantity,
            write_values,
        )
    }
}

/// Response PDU for FC 0x17 Read/Write Multiple Registers.
///
/// Contains the read register values as big-endian bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadWriteMultipleRegistersResponse {
    pub register_values: Vec<u8>,
}

impl ReadWriteMultipleRegistersResponse {
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
        buf[0] = ReadWriteMultipleRegistersRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        buf[2..2 + byte_count].copy_from_slice(&self.register_values);
        Ok(2 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadWriteMultipleRegistersRequest::FUNCTION_CODE {
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
        let write_values = vec![0x00, 0x0A, 0x01, 0x02];
        let req = ReadWriteMultipleRegistersRequest::new(
            0x0003, // read start
            2,      // read qty
            0x000E, // write start
            2,      // write qty
            write_values.clone(),
        )
        .unwrap();
        let mut buf = [0u8; 14];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 14);
        assert_eq!(
            buf,
            [0x17, 0x00, 0x03, 0x00, 0x02, 0x00, 0x0E, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02]
        );

        let decoded = ReadWriteMultipleRegistersRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_quantity_bounds() {
        let values = vec![0x00, 0x00];
        assert!(ReadWriteMultipleRegistersRequest::new(0, 0, 0, 1, values.clone()).is_err());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 1, 0, 1, values.clone()).is_ok());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 125, 0, 1, values.clone()).is_ok());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 126, 0, 1, values.clone()).is_err());

        assert!(ReadWriteMultipleRegistersRequest::new(0, 1, 0, 0, values.clone()).is_err());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 1, 0, 1, values.clone()).is_ok());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 1, 0, 121, vec![0u8; 242]).is_ok());
        assert!(ReadWriteMultipleRegistersRequest::new(0, 1, 0, 122, vec![0u8; 244]).is_err());
    }

    #[test]
    fn request_rejects_mismatched_write_value_length() {
        // 2 write registers need 4 bytes, not 2
        assert!(
            ReadWriteMultipleRegistersRequest::new(0, 1, 0, 2, vec![0x00, 0x00]).is_err()
        );
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x03, 0x00, 0x02, 0x00, 0x0E, 0x00, 0x02, 0x04, 0x00, 0x0A, 0x01, 0x02];
        assert!(matches!(
            ReadWriteMultipleRegistersRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReadWriteMultipleRegistersResponse {
            register_values: vec![0x00, 0x0A, 0x01, 0x02],
        };
        let mut buf = [0u8; 6];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 6);
        assert_eq!(buf, [0x17, 0x04, 0x00, 0x0A, 0x01, 0x02]);

        let decoded = ReadWriteMultipleRegistersResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_rejects_odd_byte_count() {
        let resp = ReadWriteMultipleRegistersResponse {
            register_values: vec![0x00, 0x0A, 0x01],
        };
        let mut buf = [0u8; 8];
        assert!(matches!(
            resp.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));

        let buf = [0x17, 0x03, 0x00, 0x0A, 0x01];
        assert!(matches!(
            ReadWriteMultipleRegistersResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x02, 0x00, 0x00];
        assert!(matches!(
            ReadWriteMultipleRegistersResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadWriteMultipleRegistersRequest::new(0, 1, 0, 1, vec![0x00, 0x00]).unwrap();
        let mut buf = [0u8; 9];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
