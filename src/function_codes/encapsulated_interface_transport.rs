use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// MEI type for the CANopen General Reference request/response.
pub const MEI_TYPE_CANOPEN_GENERAL_REFERENCE: u8 = 0x0D;

/// MEI type for the Read Device Identification request/response.
pub const MEI_TYPE_READ_DEVICE_IDENTIFICATION: u8 = 0x0E;

/// Read Device Identification code: request the basic identification stream.
pub const READ_DEVICE_ID_CODE_BASIC: u8 = 0x01;

/// Read Device Identification code: request the regular identification stream.
pub const READ_DEVICE_ID_CODE_REGULAR: u8 = 0x02;

/// Read Device Identification code: request the extended identification stream.
pub const READ_DEVICE_ID_CODE_EXTENDED: u8 = 0x03;

/// Read Device Identification code: request one specific identification object.
pub const READ_DEVICE_ID_CODE_SPECIFIC: u8 = 0x04;

/// Request PDU for FC 0x2B Encapsulated Interface Transport.
///
/// The MEI type selects the encapsulated protocol; the remaining bytes are the
/// MEI-specific payload. This covers both MEI type 0x0D (CANopen General
/// Reference) and 0x0E (Read Device Identification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncapsulatedInterfaceTransportRequest {
    pub mei_type: u8,
    pub data: Vec<u8>,
}

impl EncapsulatedInterfaceTransportRequest {
    pub const FUNCTION_CODE: u8 = 0x2B;

    /// Create a new request.
    pub fn new(mei_type: u8, data: Vec<u8>) -> Self {
        Self { mei_type, data }
    }

    /// Encode the request into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 2 + self.data.len() {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::FUNCTION_CODE;
        buf[1] = self.mei_type;
        buf[2..2 + self.data.len()].copy_from_slice(&self.data);
        Ok(2 + self.data.len())
    }

    /// Decode a request from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let mei_type = buf[1];
        let data = buf[2..].to_vec();
        Ok(Self::new(mei_type, data))
    }
}

/// Response PDU for FC 0x2B Encapsulated Interface Transport.
///
/// The response echoes the MEI type and carries the MEI-specific payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncapsulatedInterfaceTransportResponse {
    pub mei_type: u8,
    pub data: Vec<u8>,
}

impl EncapsulatedInterfaceTransportResponse {
    /// Create a new response.
    pub fn new(mei_type: u8, data: Vec<u8>) -> Self {
        Self { mei_type, data }
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 2 + self.data.len() {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = EncapsulatedInterfaceTransportRequest::FUNCTION_CODE;
        buf[1] = self.mei_type;
        buf[2..2 + self.data.len()].copy_from_slice(&self.data);
        Ok(2 + self.data.len())
    }

    /// Decode a response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != EncapsulatedInterfaceTransportRequest::FUNCTION_CODE {
            return Err(DecodeError::UnknownFunctionCode);
        }
        let mei_type = buf[1];
        let data = buf[2..].to_vec();
        Ok(Self::new(mei_type, data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip_read_device_id() {
        let req = EncapsulatedInterfaceTransportRequest::new(
            MEI_TYPE_READ_DEVICE_IDENTIFICATION,
            vec![READ_DEVICE_ID_CODE_BASIC, 0x00],
        );
        let mut buf = [0u8; 4];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 4);
        assert_eq!(buf, [0x2B, 0x0E, 0x01, 0x00]);

        let decoded = EncapsulatedInterfaceTransportRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_roundtrip_canopen() {
        let req = EncapsulatedInterfaceTransportRequest::new(
            MEI_TYPE_CANOPEN_GENERAL_REFERENCE,
            vec![0x00, 0x10, 0x20, 0x05, 0x02, 0xAA, 0xBB],
        );
        let mut buf = [0u8; 9];
        let n = req.encode(&mut buf).unwrap();
        assert_eq!(n, 9);

        let decoded = EncapsulatedInterfaceTransportRequest::decode(&buf).unwrap();
        assert_eq!(decoded, req);
    }

    #[test]
    fn request_decode_rejects_wrong_function_code() {
        let buf = [0x2C, 0x0E, 0x01, 0x00];
        assert!(matches!(
            EncapsulatedInterfaceTransportRequest::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn response_roundtrip_read_device_id() {
        let resp = EncapsulatedInterfaceTransportResponse::new(
            MEI_TYPE_READ_DEVICE_IDENTIFICATION,
            vec![
                READ_DEVICE_ID_CODE_BASIC,
                0x01, // conformity level
                0x00, // more follows
                0x00, // next object id
                0x01, // number of objects
                0x01, // object id
                0x02, // object length
                0x41, 0x42, // object value "AB"
            ],
        );
        let mut buf = [0u8; 11];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 11);

        let decoded = EncapsulatedInterfaceTransportResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_roundtrip_canopen() {
        let resp = EncapsulatedInterfaceTransportResponse::new(
            MEI_TYPE_CANOPEN_GENERAL_REFERENCE,
            vec![0x00, 0x10, 0x20, 0x05, 0x02, 0xAA, 0xBB],
        );
        let mut buf = [0u8; 9];
        let n = resp.encode(&mut buf).unwrap();
        assert_eq!(n, 9);

        let decoded = EncapsulatedInterfaceTransportResponse::decode(&buf).unwrap();
        assert_eq!(decoded, resp);
    }

    #[test]
    fn response_decode_rejects_wrong_function_code() {
        let buf = [0x2C, 0x0E, 0x01, 0x00];
        assert!(matches!(
            EncapsulatedInterfaceTransportResponse::decode(&buf),
            Err(DecodeError::UnknownFunctionCode)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let req = EncapsulatedInterfaceTransportRequest::new(
            MEI_TYPE_READ_DEVICE_IDENTIFICATION,
            vec![READ_DEVICE_ID_CODE_BASIC, 0x00],
        );
        let mut buf = [0u8; 1];
        assert!(matches!(
            req.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
