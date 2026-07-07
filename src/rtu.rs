//! RTU ADU framing (slave address + PDU + CRC-16).

#![cfg(any(feature = "rtu", feature = "async"))]

use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

/// Compute the Modbus RTU CRC-16 over `data`.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc ^= byte as u16;
        for _ in 0..8 {
            if crc & 0x0001 != 0 {
                crc = (crc >> 1) ^ 0xA001;
            } else {
                crc >>= 1;
            }
        }
    }
    crc
}

/// A complete RTU ADU frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RtuAdu {
    pub address: u8,
    pub pdu: Vec<u8>,
}

impl RtuAdu {
    /// The broadcast address (0). Servers do not respond to broadcast frames.
    pub const BROADCAST_ADDRESS: u8 = 0x00;

    /// Minimum frame size: address + function code + 2 CRC bytes.
    pub const MIN_FRAME_SIZE: usize = 4;

    /// Create a new ADU.
    pub fn new(address: u8, pdu: Vec<u8>) -> Self {
        Self { address, pdu }
    }

    /// Returns `true` if this frame is addressed to all slaves (broadcast).
    pub fn is_broadcast(&self) -> bool {
        self.address == Self::BROADCAST_ADDRESS
    }

    /// Encode the ADU into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let frame_len = Self::MIN_FRAME_SIZE - 1 + self.pdu.len();
        if buf.len() < frame_len {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = self.address;
        buf[1..1 + self.pdu.len()].copy_from_slice(&self.pdu);
        let crc = crc16(&buf[..1 + self.pdu.len()]);
        buf[1 + self.pdu.len()] = (crc & 0xFF) as u8;
        buf[2 + self.pdu.len()] = ((crc >> 8) & 0xFF) as u8;
        Ok(frame_len)
    }

    /// Decode an ADU from `buf`, verifying the CRC-16.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < Self::MIN_FRAME_SIZE {
            return Err(DecodeError::InvalidLength);
        }
        let address = buf[0];
        let pdu_end = buf.len() - 2;
        let pdu = buf[1..pdu_end].to_vec();
        let received_crc = u16::from_le_bytes([buf[pdu_end], buf[pdu_end + 1]]);
        let computed_crc = crc16(&buf[..pdu_end]);
        if received_crc != computed_crc {
            return Err(DecodeError::InvalidValue);
        }
        Ok(Self::new(address, pdu))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_crc_vector() {
        // Address 1, FC 3, read 10 registers from address 0.
        let frame = RtuAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 8];
        let n = frame.encode(&mut buf).unwrap();
        assert_eq!(n, 8);
        assert_eq!(buf, [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xC5, 0xCD]);

        let decoded = RtuAdu::decode(&buf).unwrap();
        assert_eq!(decoded, frame);
    }

    #[test]
    fn crc_matches_expected_value() {
        // CRC for the known vector excluding the CRC field.
        let data = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert_eq!(crc16(&data), 0xCDC5);
    }

    #[test]
    fn broadcast_address_is_valid() {
        let frame = RtuAdu::new(
            RtuAdu::BROADCAST_ADDRESS,
            vec![0x0F, 0x00, 0x10, 0x00, 0x02],
        );
        assert!(frame.is_broadcast());

        let mut buf = [0u8; 8];
        let n = frame.encode(&mut buf).unwrap();
        assert_eq!(n, 8);

        let decoded = RtuAdu::decode(&buf).unwrap();
        assert!(decoded.is_broadcast());
        assert_eq!(decoded.pdu, frame.pdu);
    }

    #[test]
    fn decode_rejects_bad_crc() {
        let mut buf = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A, 0xCD, 0xC5];
        buf[6] = 0x00;
        assert!(matches!(
            RtuAdu::decode(&buf),
            Err(DecodeError::InvalidValue)
        ));
    }

    #[test]
    fn decode_rejects_truncated_frame() {
        let buf = [0x01, 0x03, 0xCD];
        assert!(matches!(
            RtuAdu::decode(&buf),
            Err(DecodeError::InvalidLength)
        ));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let frame = RtuAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 4];
        assert!(matches!(
            frame.encode(&mut buf),
            Err(EncodeError::BufferTooSmall)
        ));
    }
}
