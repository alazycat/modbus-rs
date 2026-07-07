use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x18 Read FIFO Queue.
///
/// Reads the FIFO queue pointed to by `fifo_pointer_address`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadFifoQueueRequest {
    pub fifo_pointer_address: u16,
}

impl ReadFifoQueueRequest {
    pub const FUNCTION_CODE: u8 = 0x18;

    /// Create a new request.
    pub fn new(fifo_pointer_address: u16) -> Self {
        Self {
            fifo_pointer_address,
        }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 3 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.fifo_pointer_address.to_be_bytes());
        Ok(3)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 3 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let fifo_pointer_address = u16::from_be_bytes([buf[1], buf[2]]);
        Ok(Self::new(fifo_pointer_address))
    }
}

/// Response PDU for FC 0x18 Read FIFO Queue.
///
/// The byte count field is two bytes (not one), followed by the FIFO count
/// (two bytes) and the queued register values as big-endian bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadFifoQueueResponse {
    pub fifo_count: u16,
    pub register_values: Vec<u8>,
}

impl ReadFifoQueueResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count = self.register_values.len() + 2; // +2 for FIFO count
        if byte_count > u16::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if !self.register_values.len().is_multiple_of(2) {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 3 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadFifoQueueRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&(byte_count as u16).to_be_bytes());
        buf[3..5].copy_from_slice(&self.fifo_count.to_be_bytes());
        buf[5..5 + self.register_values.len()].copy_from_slice(&self.register_values);
        Ok(3 + byte_count)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadFifoQueueRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = u16::from_be_bytes([buf[1], buf[2]]) as usize;
        if byte_count < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf.len() < 3 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let fifo_count = u16::from_be_bytes([buf[3], buf[4]]);
        let register_values = buf[5..3 + byte_count].to_vec();
        if !register_values.len().is_multiple_of(2) {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self {
            fifo_count,
            register_values,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReadFifoQueueRequest::new(0x04DE);
        let mut buf = [0u8; 3];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 3);
        assert_eq!(buf, [0x18, 0x04, 0xDE]);

        let decoded = ReadFifoQueueRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x04, 0xDE];
        assert!(matches!(
            ReadFifoQueueRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        // Spec example: byte count 8, fifo count 3, then 6 bytes of data
        let resp = ReadFifoQueueResponse {
            fifo_count: 3,
            register_values: vec![0x01, 0xB8, 0x01, 0x2C, 0x00, 0x7B],
        };
        let mut buf = [0u8; 11];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 11);
        assert_eq!(
            buf,
            [0x18, 0x00, 0x08, 0x00, 0x03, 0x01, 0xB8, 0x01, 0x2C, 0x00, 0x7B]
        );

        let decoded = ReadFifoQueueResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_empty_queue() {
        // byte count 2, fifo count 0, no register values
        let resp = ReadFifoQueueResponse {
            fifo_count: 0,
            register_values: vec![],
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x18, 0x00, 0x02, 0x00, 0x00]);

        let decoded = ReadFifoQueueResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_rejects_odd_register_values() {
        let resp = ReadFifoQueueResponse {
            fifo_count: 1,
            register_values: vec![0x01],
        };
        let mut buf = [0u8; 8];
        assert!(matches!(
            resp.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x02, 0x00, 0x00];
        assert!(matches!(
            ReadFifoQueueResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_decode_rejects_truncated_byte_count() {
        // byte count says 8 but only 4 bytes follow
        let buf = [0x18, 0x00, 0x08, 0x00, 0x03, 0x01, 0xB8];
        assert!(matches!(
            ReadFifoQueueResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadFifoQueueRequest::new(0);
        let mut buf = [0u8; 2];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
