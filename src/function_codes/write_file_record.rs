use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Sub-request within a FC 0x15 Write File Record request.
///
/// The response echoes the request, so this type is also used for the
/// sub-response payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteFileRecordSubRequest {
    pub reference_type: u8,
    pub file_number: u16,
    pub record_number: u16,
    pub record_length: u16,
    pub record_data: Vec<u8>,
}

/// Alias for the echoed sub-response in FC 0x15 Write File Record.
pub type WriteFileRecordSubResponse = WriteFileRecordSubRequest;

impl WriteFileRecordSubRequest {
    pub const REFERENCE_TYPE: u8 = 0x06;

    /// Create a new sub-request.
    ///
    /// `record_data` must contain an even number of bytes; each pair encodes
    /// one 16-bit register value in big-endian order.
    pub fn new(file_number: u16, record_number: u16, record_data: Vec<u8>) -> Self {
        let record_length = (record_data.len() / 2) as u16;
        Self {
            reference_type: Self::REFERENCE_TYPE,
            file_number,
            record_number,
            record_length,
            record_data,
        }
    }

    /// Encode this sub-request into `buf` and return bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let data_len = self.record_data.len();
        let sub_len = 7 + data_len;
        if sub_len > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < sub_len {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = self.reference_type;
        buf[1..3].copy_from_slice(&self.file_number.to_be_bytes());
        buf[3..5].copy_from_slice(&self.record_number.to_be_bytes());
        buf[5..7].copy_from_slice(&self.record_length.to_be_bytes());
        buf[7..7 + data_len].copy_from_slice(&self.record_data);
        Ok(sub_len)
    }

    /// Decode a sub-request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 7 {
            return Err(DecodeError::InvalidLength);
        }
        let reference_type = buf[0];
        if reference_type != Self::REFERENCE_TYPE {
            return Err(DecodeError::InvalidValue);
        }
        let file_number = u16::from_be_bytes([buf[1], buf[2]]);
        let record_number = u16::from_be_bytes([buf[3], buf[4]]);
        let record_length = u16::from_be_bytes([buf[5], buf[6]]);
        let data_len = record_length as usize * 2;
        if buf.len() < 7 + data_len {
            return Err(DecodeError::InvalidLength);
        }
        let record_data = buf[7..7 + data_len].to_vec();
        Ok(Self {
            reference_type,
            file_number,
            record_number,
            record_length,
            record_data,
        })
    }
}

/// Request PDU for FC 0x15 Write File Record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteFileRecordRequest {
    pub sub_requests: Vec<WriteFileRecordSubRequest>,
}

impl WriteFileRecordRequest {
    pub const FUNCTION_CODE: u8 = 0x15;

    /// Create a new request.
    pub fn new(sub_requests: Vec<WriteFileRecordSubRequest>) -> Self {
        Self { sub_requests }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count: usize = self.sub_requests.iter().map(|s| 7 + s.record_data.len()).sum();
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
            let n = sub.encode(&mut buf[offset..])?;
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
        let mut sub_requests = Vec::new();
        let mut offset = 2;
        let end = 2 + byte_count;
        while offset < end {
            let sub = WriteFileRecordSubRequest::decode(&buf[offset..end])?;
            offset += 7 + sub.record_data.len();
            sub_requests.push(sub);
        }
        if offset != end {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self::new(sub_requests))
    }
}

/// Response PDU for FC 0x15 Write File Record.
///
/// The normal response is an echo of the request payload, so encoding and
/// decoding are identical to the request aside from the function code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteFileRecordResponse {
    pub sub_responses: Vec<WriteFileRecordSubResponse>,
}

impl WriteFileRecordResponse {
    /// Create a new response.
    pub fn new(sub_responses: Vec<WriteFileRecordSubResponse>) -> Self {
        Self { sub_responses }
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let byte_count: usize = self.sub_responses.iter().map(|s| 7 + s.record_data.len()).sum();
        if byte_count > u8::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        if buf.len() < 2 + byte_count {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = WriteFileRecordRequest::FUNCTION_CODE;
        buf[1] = byte_count as u8;
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
        if buf[0] != WriteFileRecordRequest::FUNCTION_CODE {
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
            let sub = WriteFileRecordSubResponse::decode(&buf[offset..end])?;
            offset += 7 + sub.record_data.len();
            sub_responses.push(sub);
        }
        if offset != end {
            return Err(DecodeError::InvalidLength);
        }
        Ok(Self::new(sub_responses))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = WriteFileRecordRequest::new(vec![WriteFileRecordSubRequest::new(
            0x0004,
            0x0001,
            vec![0x0D, 0xFE, 0x00, 0x20],
        )]);
        let mut buf = [0u8; 13];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 13);
        assert_eq!(
            buf,
            [
                0x15, 0x0B, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x0D, 0xFE, 0x00, 0x20
            ]
        );

        let decoded = WriteFileRecordRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_multiple_sub_requests() {
        let req = WriteFileRecordRequest::new(vec![
            WriteFileRecordSubRequest::new(0x0001, 0x0002, vec![0x00, 0x01]),
            WriteFileRecordSubRequest::new(0x0004, 0x0005, vec![0x00, 0x02, 0x00, 0x03]),
        ]);
        let mut buf = [0u8; 22];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 22);

        let decoded = WriteFileRecordRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [
            0x14, 0x0B, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x0D, 0xFE, 0x00, 0x20,
        ];
        assert!(matches!(
            WriteFileRecordRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn request_decode_rejects_bad_reference_type() {
        let buf = [
            0x15, 0x0B, 0x05, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x0D, 0xFE, 0x00, 0x20,
        ];
        assert!(matches!(
            WriteFileRecordRequest::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = WriteFileRecordResponse::new(vec![WriteFileRecordSubResponse::new(
            0x0004,
            0x0001,
            vec![0x0D, 0xFE, 0x00, 0x20],
        )]);
        let mut buf = [0u8; 13];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 13);
        assert_eq!(
            buf,
            [
                0x15, 0x0B, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x0D, 0xFE, 0x00, 0x20
            ]
        );

        let decoded = WriteFileRecordResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_multiple_sub_responses() {
        let resp = WriteFileRecordResponse::new(vec![
            WriteFileRecordSubResponse::new(0x0001, 0x0002, vec![0x00, 0x01]),
            WriteFileRecordSubResponse::new(0x0004, 0x0005, vec![0x00, 0x02, 0x00, 0x03]),
        ]);
        let mut buf = [0u8; 22];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 22);

        let decoded = WriteFileRecordResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [
            0x14, 0x0B, 0x06, 0x00, 0x04, 0x00, 0x01, 0x00, 0x02, 0x0D, 0xFE, 0x00, 0x20,
        ];
        assert!(matches!(
            WriteFileRecordResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = WriteFileRecordRequest::new(vec![]);
        let mut buf = [0u8; 1];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
