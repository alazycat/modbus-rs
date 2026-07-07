use crate::error::{DecodeError, EncodeError};

/// Standard MODBUS exception codes (0x01-0x0B).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExceptionCode {
    /// Function code received in the query is not an allowable action for the server.
    IllegalFunction = 0x01,
    /// The data address received in the query is not an allowable address.
    IllegalDataAddress = 0x02,
    /// A value contained in the query data field is not an allowable value.
    IllegalDataValue = 0x03,
    /// An unrecoverable error occurred while the server was attempting to perform the action.
    ServerDeviceFailure = 0x04,
    /// The server has accepted the request and is processing it, but this will take a long time.
    Acknowledge = 0x05,
    /// The server is engaged in processing a long-duration program command.
    ServerDeviceBusy = 0x06,
    /// Reserved exception code 0x07.
    Reserved0x07 = 0x07,
    /// The server attempted to read record file, but detected a parity error in the memory.
    MemoryParityError = 0x08,
    /// Reserved exception code 0x09.
    Reserved0x09 = 0x09,
    /// The gateway was unable to allocate an internal communication path to the target device.
    GatewayPathUnavailable = 0x0A,
    /// The gateway received no response from the target device.
    GatewayTargetDeviceFailedToRespond = 0x0B,
}

impl ExceptionCode {
    /// Convert a raw exception code into an `ExceptionCode`.
    pub fn from_u8(code: u8) -> Option<Self> {
        match code {
            0x01 => Some(Self::IllegalFunction),
            0x02 => Some(Self::IllegalDataAddress),
            0x03 => Some(Self::IllegalDataValue),
            0x04 => Some(Self::ServerDeviceFailure),
            0x05 => Some(Self::Acknowledge),
            0x06 => Some(Self::ServerDeviceBusy),
            0x07 => Some(Self::Reserved0x07),
            0x08 => Some(Self::MemoryParityError),
            0x09 => Some(Self::Reserved0x09),
            0x0A => Some(Self::GatewayPathUnavailable),
            0x0B => Some(Self::GatewayTargetDeviceFailedToRespond),
            _ => None,
        }
    }

    /// Return the raw exception code value.
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Exception response PDU.
///
/// The encoded function code is the original function code with the high bit
/// set (`0x80`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExceptionResponse {
    /// Original function code without the exception flag.
    pub function_code: u8,
    pub exception_code: ExceptionCode,
}

impl ExceptionResponse {
    pub const EXCEPTION_FLAG: u8 = 0x80;

    /// Create a new exception response.
    pub fn new(function_code: u8, exception_code: ExceptionCode) -> Self {
        Self {
            function_code: function_code & 0x7F,
            exception_code,
        }
    }

    /// Encode the response into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        if buf.len() < 2 {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = self.function_code | Self::EXCEPTION_FLAG;
        buf[1] = self.exception_code.as_u8();
        Ok(2)
    }

    /// Decode an exception response from `buf`.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        let raw_function_code = buf[0];
        if raw_function_code & Self::EXCEPTION_FLAG == 0 {
            return Err(DecodeError::InvalidValue);
        }
        let function_code = raw_function_code & !Self::EXCEPTION_FLAG;
        let exception_code =
            ExceptionCode::from_u8(buf[1]).ok_or(DecodeError::InvalidValue)?;
        Ok(Self {
            function_code,
            exception_code,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_all_standard_codes() {
        let codes = [
            ExceptionCode::IllegalFunction,
            ExceptionCode::IllegalDataAddress,
            ExceptionCode::IllegalDataValue,
            ExceptionCode::ServerDeviceFailure,
            ExceptionCode::Acknowledge,
            ExceptionCode::ServerDeviceBusy,
            ExceptionCode::Reserved0x07,
            ExceptionCode::MemoryParityError,
            ExceptionCode::Reserved0x09,
            ExceptionCode::GatewayPathUnavailable,
            ExceptionCode::GatewayTargetDeviceFailedToRespond,
        ];
        for (idx, code) in codes.iter().enumerate() {
            let resp = ExceptionResponse::new(0x03, *code);
            let mut buf = [0u8; 2];
            let n = resp.encode(&mut buf).unwrap();
            assert_eq!(n, 2);
            assert_eq!(buf[0], 0x83);
            assert_eq!(buf[1], (idx + 1) as u8);

            let decoded = ExceptionResponse::decode(&buf).unwrap();
            assert_eq!(decoded, resp);
        }
    }

    #[test]
    fn decode_rejects_non_exception_function_code() {
        let buf = [0x03, 0x02];
        assert!(matches!(
            ExceptionResponse::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn decode_rejects_unknown_exception_code() {
        let buf = [0x83, 0x0C];
        assert!(matches!(
            ExceptionResponse::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn decode_rejects_truncated_buffer() {
        let buf = [0x83];
        assert!(matches!(
            ExceptionResponse::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let resp = ExceptionResponse::new(0x01, ExceptionCode::IllegalFunction);
        let mut buf = [0u8; 1];
        assert!(matches!(
            resp.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }

    #[test]
    fn new_masks_high_bit_of_function_code() {
        let resp = ExceptionResponse::new(0x83, ExceptionCode::IllegalFunction);
        assert_eq!(resp.function_code, 0x03);
        let mut buf = [0u8; 2];
        resp.encode(&mut buf).unwrap();
        assert_eq!(buf[0], 0x83);
    }
}
