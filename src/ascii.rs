//! ASCII ADU framing (colon start, hex-encoded address + PDU + LRC-8, CR/LF end).

#![cfg(feature = "ascii")]

use alloc::vec::Vec;

use crate::error::{DecodeError, EncodeError};

const HEX_CHARS: [u8; 16] = [
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'A', b'B', b'C', b'D', b'E', b'F',
];

fn encode_hex_byte(byte: u8) -> [u8; 2] {
    [
        HEX_CHARS[(byte >> 4) as usize],
        HEX_CHARS[(byte & 0x0F) as usize],
    ]
}

fn decode_hex_char(c: u8) -> Result<u8, DecodeError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        _ => Err(DecodeError::InvalidValue),
    }
}

fn decode_hex_byte(high: u8, low: u8) -> Result<u8, DecodeError> {
    let high = decode_hex_char(high)?;
    let low = decode_hex_char(low)?;
    Ok((high << 4) | low)
}

/// Compute the ASCII mode LRC-8 over `data`.
pub fn lrc8(data: &[u8]) -> u8 {
    let sum = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    sum.wrapping_neg()
}

/// A complete ASCII ADU frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AsciiAdu {
    pub address: u8,
    pub pdu: Vec<u8>,
}

impl AsciiAdu {
    /// Start-of-frame character.
    pub const START: u8 = b':';

    /// End-of-frame marker.
    pub const END: &[u8] = b"\r\n";

    /// Create a new ADU.
    pub fn new(address: u8, pdu: Vec<u8>) -> Self {
        Self { address, pdu }
    }

    /// Encode the ADU into `buf` and return the number of bytes written.
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError> {
        let pdu_len = self.pdu.len();
        let out_len = 7 + pdu_len * 2;
        if buf.len() < out_len {
            return Err(EncodeError::BufferTooSmall);
        }
        buf[0] = Self::START;
        let mut offset = 1;

        for &byte in core::iter::once(&self.address).chain(self.pdu.iter()) {
            let hex = encode_hex_byte(byte);
            buf[offset] = hex[0];
            buf[offset + 1] = hex[1];
            offset += 2;
        }

        let mut sum = self.address;
        for &b in &self.pdu {
            sum = sum.wrapping_add(b);
        }
        let lrc = sum.wrapping_neg();
        let hex = encode_hex_byte(lrc);
        buf[offset] = hex[0];
        buf[offset + 1] = hex[1];
        offset += 2;

        buf[offset] = b'\r';
        buf[offset + 1] = b'\n';
        offset += 2;
        Ok(offset)
    }

    /// Decode an ADU from `buf`, verifying the LRC-8.
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError> {
        if buf.len() < 9 {
            return Err(DecodeError::InvalidLength);
        }
        if buf[0] != Self::START {
            return Err(DecodeError::InvalidValue);
        }
        if !buf.ends_with(Self::END) {
            return Err(DecodeError::InvalidValue);
        }
        let hex = &buf[1..buf.len() - 2];
        if hex.len() & 1 != 0 {
            return Err(DecodeError::InvalidLength);
        }
        if hex.len() < 4 {
            return Err(DecodeError::InvalidLength);
        }

        let mut bytes = Vec::with_capacity(hex.len() / 2);
        for pair in hex.chunks(2) {
            bytes.push(decode_hex_byte(pair[0], pair[1])?);
        }

        let received_lrc = bytes.pop().ok_or(DecodeError::InvalidLength)?;
        if bytes.len() < 2 {
            return Err(DecodeError::InvalidLength);
        }
        let address = bytes[0];
        let pdu = bytes[1..].to_vec();
        let computed_lrc = lrc8(&bytes);
        if received_lrc != computed_lrc {
            return Err(DecodeError::InvalidValue);
        }

        Ok(Self::new(address, pdu))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lrc_matches_expected_value() {
        let data = [0x01, 0x03, 0x00, 0x00, 0x00, 0x0A];
        assert_eq!(lrc8(&data), 0xF2);
    }

    #[test]
    fn roundtrip() {
        let adu = AsciiAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 17];
        let n = adu.encode(&mut buf).unwrap();
        assert_eq!(n, 17);
        assert_eq!(
            &buf[..n],
            b":01030000000AF2\r\n"
        );

        let decoded = AsciiAdu::decode(&buf[..n]).unwrap();
        assert_eq!(decoded, adu);
    }

    #[test]
    fn decode_rejects_missing_start() {
        let buf = b"01030000000AF2\r\n";
        assert!(matches!(AsciiAdu::decode(buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_missing_crlf() {
        let buf = b":01030000000AF2";
        assert!(matches!(AsciiAdu::decode(buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_bad_lrc() {
        let buf = b":01030000000AF3\r\n";
        assert!(matches!(AsciiAdu::decode(buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_invalid_hex() {
        let buf = b":01030000000AG2\r\n";
        assert!(matches!(AsciiAdu::decode(buf), Err(DecodeError::InvalidValue)));
    }

    #[test]
    fn decode_rejects_truncated_frame() {
        let buf = b":01F2\r\n";
        assert!(matches!(AsciiAdu::decode(buf), Err(DecodeError::InvalidLength)));
    }

    #[test]
    fn encode_rejects_too_small_buffer() {
        let adu = AsciiAdu::new(0x01, vec![0x03, 0x00, 0x00, 0x00, 0x0A]);
        let mut buf = [0u8; 8];
        assert!(matches!(adu.encode(&mut buf), Err(EncodeError::BufferTooSmall)));
    }
}
