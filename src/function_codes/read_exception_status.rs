use crate::error::{DecodeError, EncodeError};

/// Request PDU for FC 0x07 Read Exception Status.
///
/// The request carries no payload beyond the function code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadExceptionStatusRequest;

impl ReadExceptionStatusRequest {
    pub const FUNCTION_CODE: u8 = 0x07;

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.is_empty() {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        Ok(1)
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.is_empty() {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        Ok(Self)
    }
}

/// Response PDU for FC 0x07 Read Exception Status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadExceptionStatusResponse {
    pub data: u8,
}

impl ReadExceptionStatusResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 2 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = ReadExceptionStatusRequest::FUNCTION_CODE;
        buf[1] = self.data;
        Ok(2)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != ReadExceptionStatusRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        Ok(Self { data: buf[1] })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 1];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 1);
        assert_eq!(buf, [0x07]);

        let decoded = ReadExceptionStatusRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06];
        assert!(matches!(
            ReadExceptionStatusRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = ReadExceptionStatusResponse { data: 0x6D };
        let mut buf = [0u8; 2];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 2);
        assert_eq!(buf, [0x07, 0x6D]);

        let decoded = ReadExceptionStatusResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x6D];
        assert!(matches!(
            ReadExceptionStatusResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = ReadExceptionStatusRequest;
        let mut buf = [0u8; 0];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
