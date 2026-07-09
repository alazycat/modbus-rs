use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Sub-request within a FC 0x14 Read File Record request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadFileRecordSubRequest {
    pub reference_type: u8,
    pub file_number: u16,
    pub record_number: u16,
    pub record_length: u16,
}

impl ReadFileRecordSubRequest {
    pub const REQUEST_REFERENCE_TYPE: u8 = 0x06;

    /// Create a new sub-request, validating the reference type.
    pub fn new(file_number: u16, record_number: u16, record_length: u16) -> Self {
        Self {
            reference_type: Self::REQUEST_REFERENCE_TYPE,
            file_number,
            record_number,
            record_length,
        }
    }

    /// Encode this sub-request into `buf` and return bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 7 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = self.reference_type;
        buf[1..3].copy_from_slice(&self.file_number.to_be_bytes());
        buf[3..5].copy_from_slice(&self.record_number.to_be_bytes());
        buf[5..7].copy_from_slice(&self.record_length.to_be_bytes());
        Ok(7)
    }

    /// Decode a sub-request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 7 {
            return Err(DecodeError::InvalidLength);
        }
        let reference_type = buf[0];
        let file_number = u16::from_be_bytes([buf[1], buf[2]]);
        let record_number = u16::from_be_bytes([buf[3], buf[4]]);
        let record_length = u16::from_be_bytes([buf[5], buf[6]]);
        Ok(Self {
            reference_type,
            file_number,
            record_number,
            record_length,
        })
    }
}

/// Request PDU for FC 0x14 Read File Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadFileRecordRequest {
    pub sub_requests: Vec<ReadFileRecordSubRequest>,
}

impl ReadFileRecordRequest {
    pub const FUNCTION_CODE: u8 = 0x14;

    /// Create a new request.
    pub fn new(sub_requests: Vec<ReadFileRecordSubRequest>) -> Self {
        Self { sub_requests }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let sub_requests_len = self.sub_requests.len() * 7;
        let byte_count = sub_requests_len;
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1] = byte_count as u8;
        let mut offset = 2;
        for sub in &self.sub_requests {
            let n = sub.encode(&mut buf[offset..offset + 7])?;
            offset += n;
        }
        Ok(offset)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        if !byte_count.is_multiple_of(7) {
            return Err(DecodeError::InvalidLength);
        }
        let mut sub_requests = Vec::new();
        let mut offset = 2;
        while offset < 2 + byte_count {
            let sub = ReadFileRecordSubRequest::decode(&buf[offset..offset + 7])?;
            sub_requests.push(sub);
            offset += 7;
        }
        Ok(Self::new(sub_requests))
    }
}

/// Sub-response within a FC 0x14 Read File Record response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadFileRecordSubResponse {
    pub data: Vec<u8>,
}

impl ReadFileRecordSubResponse {
    pub const RESPONSE_REFERENCE_TYPE: u8 = 0x06;

    /// Create a new sub-response.
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }

    /// Encode this sub-response into `buf` and return bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let data_len = self.data.len();
        // file_response_length = 1 (reference type) + data length
        let file_response_length = 1 + data_len;
        if file_response_length > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 1 + file_response_length {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = file_response_length as u8;
        buf[1] = Self::RESPONSE_REFERENCE_TYPE;
        buf[2..2 + data_len].copy_from_slice(&self.data);
        Ok(1 + file_response_length)
    }

    /// Decode a sub-response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        let file_response_length = buf[0] as usize;
        if file_response_length < 1 {
            return Err(DecodeError::InvalidLength);
        }
        if buf.len() < 1 + file_response_length {
            return Err(DecodeError::InvalidLength);
        }
        let _reference_type = buf[1];
        let data = buf[2..1 + file_response_length].to_vec();
        Ok(Self::new(data))
    }
}

/// Response PDU for FC 0x14 Read File Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadFileRecordResponse {
    pub sub_responses: Vec<ReadFileRecordSubResponse>,
}

impl ReadFileRecordResponse {
    /// Create a new response.
    pub fn new(sub_responses: Vec<ReadFileRecordSubResponse>) -> Self {
        Self { sub_responses }
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let mut total = 0usize;
        for sub in &self.sub_responses {
            total += 1 + 1 + sub.data.len();
        }
        if total > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + total {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadFileRecordRequest::FUNCTION_CODE;
        buf[1] = total as u8;
        let mut offset = 2;
        for sub in &self.sub_responses {
            let n = sub.encode(&mut buf[offset..])?;
            offset += n;
        }
        Ok(offset)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadFileRecordRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let byte_count = buf[1] as usize;
        if buf.len() < 2 + byte_count {
            return Err(DecodeError::InvalidLength);
        }
        let mut sub_responses = Vec::new();
        let mut offset = 2;
        let end = 2 + byte_count;
        while offset < end {
            let sub = ReadFileRecordSubResponse::decode(&buf[offset..end])?;
            let n = 1 + 1 + sub.data.len();
            sub_responses.push(sub);
            offset += n;
        }
        Ok(Self { sub_responses })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req =
            ReadFileRecordRequest::new(vec![ReadFileRecordSubRequest::new(0x0004, 0x0001, 0x0002)]);
        let mut buf = [0u8; 9];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 9);
        assert_eq!(buf, [0x14, 0x07, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02]);

        let decoded = ReadFileRecordRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_multiple_sub_requests() {
        let req = ReadFileRecordRequest::new(vec![
            ReadFileRecordSubRequest::new(0x0001, 0x0002, 0x0003),
            ReadFileRecordSubRequest::new(0x0004, 0x0005, 0x0006),
        ]);
        let mut buf = [0u8; 16];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 16);

        let decoded = ReadFileRecordRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x15, 0x07, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02];
        assert!(matches!(
            ReadFileRecordRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReadFileRecordResponse::new(vec![ReadFileRecordSubResponse::new(vec![
            0x0D, 0xFE, 0x00, 0x20,
        ])]);
        let mut buf = [0u8; 8];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(buf, [0x14, 0x06, 0x05, 0x06, 0x0D, 0xFE, 0x00, 0x20]);

        let decoded = ReadFileRecordResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_multiple_sub_responses() {
        let resp = ReadFileRecordResponse::new(vec![
            ReadFileRecordSubResponse::new(vec![0x00, 0x01]),
            ReadFileRecordSubResponse::new(vec![0x00, 0x02, 0x00, 0x03]),
        ]);
        let mut buf = [0u8; 14];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 12);

        let decoded = ReadFileRecordResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x15, 0x06, 0x05, 0x06, 0x0D, 0xFE, 0x00, 0x20];
        assert!(matches!(
            ReadFileRecordResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadFileRecordRequest::new(vec![]);
        let mut buf = [0u8; 1];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
