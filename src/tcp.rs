//! TCP ADU framing with the MBAP header.

#![cfg(feature = "tcp")]

use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

#[cfg(any(feature = "sync", feature = "async"))]
use crate::transport::TransportError;

/// The MODBUS protocol identifier used in the MBAP header (always 0 for MODBUS).
pub const MODBUS_PROTOCOL_ID: u16 = 0x0000;

/// A complete TCP ADU frame with MBAP header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TcpAdu {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub unit_id: u8,
    pub pdu: Vec<u8>,
}

impl TcpAdu {
    /// MBAP header size in bytes.
    pub const HEADER_SIZE: usize = 7;

    /// Create a new TCP ADU.
    pub fn new(transaction_id: u16, unit_id: u8, pdu: Vec<u8>) -> Self {
        Self {
            transaction_id,
            protocol_id: MODBUS_PROTOCOL_ID,
            unit_id,
            pdu,
        }
    }

    /// Encode the ADU into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let length = 1 + self.pdu.len();
        if length > u16::MAX as usize {
            return Err(EncodeError::BufferTooSmall);
        }
        let frame_len = Self::HEADER_SIZE + self.pdu.len();
        if buf.len() < frame_len {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0..2].copy_from_slice(&self.transaction_id.to_be_bytes());
        buf[2..4].copy_from_slice(&self.protocol_id.to_be_bytes());
        buf[4..6].copy_from_slice(&(length as u16).to_be_bytes());
        buf[6] = self.unit_id;
        buf[7..7 + self.pdu.len()].copy_from_slice(&self.pdu);
        Ok(frame_len)
    }

    /// Decode an ADU from `buf`, validating the MBAP header.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < Self::HEADER_SIZE {
            return Err(DecodeError::InvalidLength);
        }
        let transaction_id = u16::from_be_bytes([buf[0], buf[1]]);
        let protocol_id = u16::from_be_bytes([buf[2], buf[3]]);
        let length = u16::from_be_bytes([buf[4], buf[5]]) as usize;
        let unit_id = buf[6];

        if protocol_id != MODBUS_PROTOCOL_ID {
            return Err(DecodeError::InvalidValue);
        }
        if length == 0 {
            return Err(DecodeError::InvalidValue);
        }
        let pdu_len = length - 1;
        if buf.len() != Self::HEADER_SIZE + pdu_len {
            return Err(DecodeError::InvalidLength);
        }
        let pdu = buf[7..7 + pdu_len].to_vec();
        Ok(Self {
            transaction_id,
            protocol_id,
            unit_id,
            pdu,
        })
    }
}

/// Compute the total TCP ADU frame length from a 7-byte MBAP header.
///
/// Validates that the protocol identifier is `MODBUS_PROTOCOL_ID` and that the
/// length field is non-zero. Returns the full frame length including the MBAP
/// header on success.
#[cfg(any(feature = "sync", feature = "async"))]
pub fn tcp_frame_len(header: &[u8; TcpAdu::HEADER_SIZE]) -> Result<usize, TransportError> {
    let protocol_id = u16::from_be_bytes([header[2], header[3]]);
    if protocol_id != MODBUS_PROTOCOL_ID {
        return Err(TransportError::Disconnected);
    }

    let length = u16::from_be_bytes([header[4], header[5]]) as usize;
    if length == 0 {
        return Err(TransportError::Disconnected);
    }

    let pdu_len = length - 1;
    Ok(TcpAdu::HEADER_SIZE + pdu_len)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let adu = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 12];
        let n = adu.encode(&mut buf).unwrap();
        assert_eq!(n, 12);
        assert_eq!(
            buf,
            [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A]
        );

        let decoded = TcpAdu::decode(&buf).unwrap();
        assert_eq!(decoded, adu);
    }

    #[test]
    fn decode_rejects_non_zero_protocol_id() {
        let buf = [
            0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A,
        ];
        assert!(matches!(
            TcpAdu::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn decode_rejects_zero_length() {
        let buf = [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0A];
        assert!(matches!(
            TcpAdu::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn decode_rejects_truncated_frame() {
        let buf = [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03];
        assert!(matches!(
            TcpAdu::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn decode_rejects_extra_bytes() {
        let buf = [
            0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xFF,
        ];
        assert!(matches!(
            TcpAdu::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let adu = TcpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 4];
        assert!(matches!(
            adu.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }

    #[test]
    #[cfg(any(feature = "sync", feature = "async"))]
    fn tcp_frame_len_computes_full_frame() {
        let header = [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A];
        assert_eq!(tcp_frame_len(&header).unwrap(), 12);
    }

    #[test]
    #[cfg(any(feature = "sync", feature = "async"))]
    fn tcp_frame_len_rejects_non_zero_protocol_id() {
        let header = [0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x0A];
        assert!(matches!(
            tcp_frame_len(&header),
            Err(TransportError::Disconnected)
        ));
    }

    #[test]
    #[cfg(any(feature = "sync", feature = "async"))]
    fn tcp_frame_len_rejects_zero_length() {
        let header = [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0A];
        assert!(matches!(
            tcp_frame_len(&header),
            Err(TransportError::Disconnected)
        ));
    }
}
