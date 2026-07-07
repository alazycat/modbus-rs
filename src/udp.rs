//! UDP ADU framing with the MBAP header.
//!
//! MODBUS over UDP reuses the same MBAP header as TCP. The entire datagram is
//! treated as a single ADU, so the length field must exactly cover the unit ID
//! and PDU bytes.

#![cfg(feature = "udp")]

use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// The MODBUS protocol identifier used in the MBAP header (always 0 for MODBUS).
pub const MODBUS_PROTOCOL_ID: u16 = 0x0000;

/// A complete UDP ADU frame with MBAP header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdpAdu {
    pub transaction_id: u16,
    pub protocol_id: u16,
    pub unit_id: u8,
    pub pdu: Vec<u8>,
}

impl UdpAdu {
    /// MBAP header size in bytes.
    pub const HEADER_SIZE: usize = 7;

    /// Create a new UDP ADU.
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

    /// Decode an ADU from a complete UDP datagram, validating the MBAP header.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let adu = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 12];
        let n = adu.encode(&mut buf).unwrap();
        assert_eq!(n, 12);
        assert_eq!(
            buf,
            [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A]
        );

        let decoded = UdpAdu::decode(&buf).unwrap();
        assert_eq!(decoded, adu);
    }

    #[test]
    fn decode_rejects_non_zero_protocol_id() {
        let buf = [0x00, 0x01, 0x00, 0x01, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert!(matches!(UdpAdu::decode(&buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_zero_length() {
        let buf = [0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0A];
        assert!(matches!(UdpAdu::decode(&buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_datagram_shorter_than_length() {
        let buf = [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03];
        assert!(matches!(UdpAdu::decode(&buf), Err(DecodeError::InvalidLength)));
    }

    #[test]
    fn decode_rejects_datagram_longer_than_length() {
        let mut buf = [0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x0A, 0x03, 0x00, 0x00, 0x00, 0x0A];
        buf[5] = 0x02; // length claims unit ID + 1 PDU byte
        assert!(matches!(UdpAdu::decode(&buf), Err(DecodeError::InvalidLength)));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let adu = UdpAdu::new(0x0001, 0x0A, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 4];
        assert!(matches!(adu.encode(&mut buf), Err(EncodeError::BufferTooSmall)));
    }
}
