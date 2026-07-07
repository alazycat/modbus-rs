use crate::error::{DecodeError, EncodeError};

/// Common sub-function codes for FC 0x08 Diagnostics.
pub const RETURN_QUERY_DATA: u16 = 0x0000;
pub const RESTART_COMMUNICATIONS_OPTION: u16 = 0x0001;
pub const RETURN_DIAGNOSTIC_REGISTER: u16 = 0x0002;
pub const CHANGE_ASCII_INPUT_DELIMITER: u16 = 0x0003;
pub const FORCE_LISTEN_ONLY_MODE: u16 = 0x0004;
pub const CLEAR_COUNTERS_AND_DIAGNOSTIC_REGISTER: u16 = 0x000A;
pub const RETURN_BUS_MESSAGE_COUNT: u16 = 0x000B;
pub const RETURN_BUS_COMMUNICATION_ERROR_COUNT: u16 = 0x000C;
pub const RETURN_BUS_EXCEPTION_ERROR_COUNT: u16 = 0x000D;
pub const RETURN_SLAVE_MESSAGE_COUNT: u16 = 0x000E;
pub const RETURN_SLAVE_NO_RESPONSE_COUNT: u16 = 0x000F;
pub const RETURN_SLAVE_NAK_COUNT: u16 = 0x0010;
pub const RETURN_SLAVE_BUSY_COUNT: u16 = 0x0011;
pub const RETURN_BUS_CHARACTER_OVERRUN_COUNT: u16 = 0x0012;
pub const CLEAR_OVERRUN_COUNTER_AND_FLAG: u16 = 0x0014;

/// Request PDU for FC 0x08 Diagnostics.
///
/// Carries a `sub_function` code and a 16-bit `data` field. The meaning of
/// `data` depends on the sub-function.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticsRequest {
    pub sub_function: u16,
    pub data: u16,
}

impl DiagnosticsRequest {
    pub const FUNCTION_CODE: u8 = 0x08;

    /// Create a new request.
    pub fn new(sub_function: u16, data: u16) -> Self {
        Self {
            sub_function,
            data,
        }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.sub_function.to_be_bytes());
        buf[3..5].copy_from_slice(&self.data.to_be_bytes());
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
        let sub_function = u16::from_be_bytes([buf[1], buf[2]]);
        let data = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self::new(sub_function, data))
    }
}

/// Response PDU for FC 0x08 Diagnostics.
///
/// The response echoes the request fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticsResponse {
    pub sub_function: u16,
    pub data: u16,
}

impl DiagnosticsResponse {
    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 5 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = DiagnosticsRequest::FUNCTION_CODE;
        buf[1..3].copy_from_slice(&self.sub_function.to_be_bytes());
        buf[3..5].copy_from_slice(&self.data.to_be_bytes());
        Ok(5)
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 5 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != DiagnosticsRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let sub_function = u16::from_be_bytes([buf[1], buf[2]]);
        let data = u16::from_be_bytes([buf[3], buf[4]]);
        Ok(Self {
            sub_function,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let req = DiagnosticsRequest::new(RETURN_QUERY_DATA, 0xA537);
        let mut buf = [0u8; 5];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x08, 0x00, 0x00, 0xA5, 0x37]);

        let decoded = DiagnosticsRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x00, 0xA5, 0x37];
        assert!(matches!(
            DiagnosticsRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip() {
        let resp = DiagnosticsResponse {
            sub_function: RETURN_BUS_MESSAGE_COUNT,
            data: 0x1234,
        };
        let mut buf = [0u8; 5];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(buf, [0x08, 0x00, 0x0B, 0x12, 0x34]);

        let decoded = DiagnosticsResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x06, 0x00, 0x0B, 0x12, 0x34];
        assert!(matches!(
            DiagnosticsResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = DiagnosticsRequest::new(RETURN_QUERY_DATA, 0);
        let mut buf = [0u8; 4];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
