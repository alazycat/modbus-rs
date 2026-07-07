//! Data-type conversion helpers for Modbus register values.
//!
//! This module is available when the `helpers` feature is enabled. It provides
//! conversions between raw register data (`u16` words or byte slices) and
//! common primitive types, with explicit control over byte endianness and
//! register word order.

#![cfg(feature = "helpers")]

use alloc::string::String;
use alloc::vec::Vec;

/// Byte endianness used when interpreting register bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Endian {
    /// Big-endian byte order (most-significant byte first).
    Big,
    /// Little-endian byte order (least-significant byte first).
    Little,
}

/// Word order used when interpreting multi-register values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordOrder {
    /// The first register contains the most-significant word.
    MostSignificantFirst,
    /// The first register contains the least-significant word.
    LeastSignificantFirst,
}

/// Errors that can occur while converting register data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HelpersError {
    /// The input does not contain the expected number of registers or bytes.
    InvalidLength,
    /// The input does not represent a valid string.
    InvalidString,
}

#[cfg(feature = "std")]
impl core::fmt::Display for HelpersError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidLength => write!(f, "invalid register/byte length"),
            Self::InvalidString => write!(f, "invalid string data"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for HelpersError {}

fn u16_to_bytes_inner(value: u16, endian: Endian) -> [u8; 2] {
    match endian {
        Endian::Big => value.to_be_bytes(),
        Endian::Little => value.to_le_bytes(),
    }
}

fn u16_from_bytes_inner(bytes: &[u8], endian: Endian) -> Result<u16, HelpersError> {
    if bytes.len() != 2 {
        return Err(HelpersError::InvalidLength);
    }
    let bytes: [u8; 2] = [bytes[0], bytes[1]];
    Ok(match endian {
        Endian::Big => u16::from_be_bytes(bytes),
        Endian::Little => u16::from_le_bytes(bytes),
    })
}

fn ordered_register_bytes(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<Vec<u8>, HelpersError> {
    let ordered: Vec<u16> = match word_order {
        WordOrder::MostSignificantFirst => regs.to_vec(),
        WordOrder::LeastSignificantFirst => regs.iter().copied().rev().collect(),
    };
    let mut bytes = Vec::with_capacity(ordered.len() * 2);
    for word in ordered {
        bytes.extend_from_slice(&u16_to_bytes_inner(word, endian));
    }
    Ok(bytes)
}

/// Decode a `u16` from two bytes using the given endianness.
pub fn u16_from_bytes(bytes: &[u8], endian: Endian) -> Result<u16, HelpersError> {
    u16_from_bytes_inner(bytes, endian)
}

/// Encode a `u16` into two bytes using the given endianness.
pub fn u16_to_bytes(value: u16, endian: Endian) -> [u8; 2] {
    u16_to_bytes_inner(value, endian)
}

/// Decode an `i16` from two bytes using the given endianness.
pub fn i16_from_bytes(bytes: &[u8], endian: Endian) -> Result<i16, HelpersError> {
    u16_from_bytes_inner(bytes, endian).map(|v| v as i16)
}

/// Encode an `i16` into two bytes using the given endianness.
pub fn i16_to_bytes(value: i16, endian: Endian) -> [u8; 2] {
    u16_to_bytes_inner(value as u16, endian)
}

/// Decode a `u32` from two registers using the given endianness and word order.
pub fn u32_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<u32, HelpersError> {
    if regs.len() != 2 {
        return Err(HelpersError::InvalidLength);
    }
    let bytes = ordered_register_bytes(regs, endian, word_order)?;
    Ok(u32::from_be_bytes(
        bytes[..4].try_into().expect("length checked above"),
    ))
}

/// Encode a `u32` into two registers using the given endianness and word order.
pub fn u32_to_registers(value: u32, endian: Endian, word_order: WordOrder) -> [u16; 2] {
    let bytes = value.to_be_bytes();
    let words = [
        u16_from_bytes_inner(&bytes[0..2], endian).expect("exactly 2 bytes"),
        u16_from_bytes_inner(&bytes[2..4], endian).expect("exactly 2 bytes"),
    ];
    match word_order {
        WordOrder::MostSignificantFirst => words,
        WordOrder::LeastSignificantFirst => [words[1], words[0]],
    }
}

/// Decode an `i32` from two registers using the given endianness and word order.
pub fn i32_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<i32, HelpersError> {
    u32_from_registers(regs, endian, word_order).map(|v| v as i32)
}

/// Encode an `i32` into two registers using the given endianness and word order.
pub fn i32_to_registers(value: i32, endian: Endian, word_order: WordOrder) -> [u16; 2] {
    u32_to_registers(value as u32, endian, word_order)
}

/// Decode a `u64` from four registers using the given endianness and word order.
pub fn u64_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<u64, HelpersError> {
    if regs.len() != 4 {
        return Err(HelpersError::InvalidLength);
    }
    let bytes = ordered_register_bytes(regs, endian, word_order)?;
    Ok(u64::from_be_bytes(
        bytes[..8].try_into().expect("length checked above"),
    ))
}

/// Encode a `u64` into four registers using the given endianness and word order.
pub fn u64_to_registers(value: u64, endian: Endian, word_order: WordOrder) -> [u16; 4] {
    let bytes = value.to_be_bytes();
    let words = [
        u16_from_bytes_inner(&bytes[0..2], endian).expect("exactly 2 bytes"),
        u16_from_bytes_inner(&bytes[2..4], endian).expect("exactly 2 bytes"),
        u16_from_bytes_inner(&bytes[4..6], endian).expect("exactly 2 bytes"),
        u16_from_bytes_inner(&bytes[6..8], endian).expect("exactly 2 bytes"),
    ];
    match word_order {
        WordOrder::MostSignificantFirst => words,
        WordOrder::LeastSignificantFirst => [words[3], words[2], words[1], words[0]],
    }
}

/// Decode an `i64` from four registers using the given endianness and word order.
pub fn i64_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<i64, HelpersError> {
    u64_from_registers(regs, endian, word_order).map(|v| v as i64)
}

/// Encode an `i64` into four registers using the given endianness and word order.
pub fn i64_to_registers(value: i64, endian: Endian, word_order: WordOrder) -> [u16; 4] {
    u64_to_registers(value as u64, endian, word_order)
}

/// Decode an `f32` from two registers using the given endianness and word order.
pub fn f32_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<f32, HelpersError> {
    u32_from_registers(regs, endian, word_order).map(f32::from_bits)
}

/// Encode an `f32` into two registers using the given endianness and word order.
pub fn f32_to_registers(value: f32, endian: Endian, word_order: WordOrder) -> [u16; 2] {
    u32_to_registers(value.to_bits(), endian, word_order)
}

/// Decode an `f64` from four registers using the given endianness and word order.
pub fn f64_from_registers(
    regs: &[u16],
    endian: Endian,
    word_order: WordOrder,
) -> Result<f64, HelpersError> {
    u64_from_registers(regs, endian, word_order).map(f64::from_bits)
}

/// Encode an `f64` into four registers using the given endianness and word order.
pub fn f64_to_registers(value: f64, endian: Endian, word_order: WordOrder) -> [u16; 4] {
    u64_to_registers(value.to_bits(), endian, word_order)
}

/// Decode a NUL-terminated string from registers using the given endianness.
///
/// Each register is decoded into two bytes. Decoding stops at the first NUL
/// byte; trailing NULs are ignored.
pub fn string_from_registers(regs: &[u16], endian: Endian) -> Result<String, HelpersError> {
    let mut bytes = Vec::with_capacity(regs.len() * 2);
    for word in regs {
        bytes.extend_from_slice(&u16_to_bytes_inner(*word, endian));
    }
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    bytes.truncate(end);
    String::from_utf8(bytes).map_err(|_| HelpersError::InvalidString)
}

/// Encode a string into registers using the given endianness.
///
/// The string is encoded as UTF-8 bytes. If `pad_to` is greater than zero, the
/// result is padded with NUL registers until it contains exactly `pad_to`
/// registers; otherwise the result is padded only to an even byte count.
pub fn string_to_registers(
    s: &str,
    endian: Endian,
    pad_to: usize,
) -> Result<Vec<u16>, HelpersError> {
    let mut bytes = s.as_bytes().to_vec();
    let min_len = if pad_to > 0 {
        pad_to * 2
    } else {
        bytes.len().div_ceil(2) * 2
    };
    if bytes.len() > min_len {
        return Err(HelpersError::InvalidLength);
    }
    bytes.resize(min_len, 0);
    let mut regs = Vec::with_capacity(bytes.len() / 2);
    for chunk in bytes.chunks_exact(2) {
        regs.push(u16_from_bytes_inner(chunk, endian)?);
    }
    Ok(regs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u16_roundtrip_big_endian() {
        assert_eq!(u16_from_bytes(&[0x12, 0x34], Endian::Big).unwrap(), 0x1234);
        assert_eq!(u16_to_bytes(0x1234, Endian::Big), [0x12, 0x34]);
    }

    #[test]
    fn u16_roundtrip_little_endian() {
        assert_eq!(
            u16_from_bytes(&[0x34, 0x12], Endian::Little).unwrap(),
            0x1234
        );
        assert_eq!(u16_to_bytes(0x1234, Endian::Little), [0x34, 0x12]);
    }

    #[test]
    fn u16_rejects_wrong_length() {
        assert!(matches!(
            u16_from_bytes(&[0x12], Endian::Big),
            Err(HelpersError::InvalidLength)
        ));
    }

    #[test]
    fn u32_msf_be() {
        // regs[0] = high word, bytes within register are big-endian.
        let regs = [0x1234, 0x5678];
        assert_eq!(
            u32_from_registers(&regs, Endian::Big, WordOrder::MostSignificantFirst).unwrap(),
            0x1234_5678
        );
        assert_eq!(
            u32_to_registers(0x1234_5678, Endian::Big, WordOrder::MostSignificantFirst),
            regs
        );
    }

    #[test]
    fn u32_lsf_be() {
        // regs[0] = low word.
        let regs = [0x5678, 0x1234];
        assert_eq!(
            u32_from_registers(&regs, Endian::Big, WordOrder::LeastSignificantFirst).unwrap(),
            0x1234_5678
        );
        assert_eq!(
            u32_to_registers(0x1234_5678, Endian::Big, WordOrder::LeastSignificantFirst),
            regs
        );
    }

    #[test]
    fn u32_msf_le() {
        // regs[0] = high word, but bytes within each register are little-endian.
        let regs = [0x3412, 0x7856];
        assert_eq!(
            u32_from_registers(&regs, Endian::Little, WordOrder::MostSignificantFirst).unwrap(),
            0x1234_5678
        );
        assert_eq!(
            u32_to_registers(0x1234_5678, Endian::Little, WordOrder::MostSignificantFirst),
            regs
        );
    }

    #[test]
    fn u32_lsf_le() {
        let regs = [0x7856, 0x3412];
        assert_eq!(
            u32_from_registers(&regs, Endian::Little, WordOrder::LeastSignificantFirst).unwrap(),
            0x1234_5678
        );
        assert_eq!(
            u32_to_registers(
                0x1234_5678,
                Endian::Little,
                WordOrder::LeastSignificantFirst
            ),
            regs
        );
    }

    #[test]
    fn u32_rejects_wrong_length() {
        assert!(matches!(
            u32_from_registers(&[0x0000], Endian::Big, WordOrder::MostSignificantFirst),
            Err(HelpersError::InvalidLength)
        ));
    }

    #[test]
    fn u64_roundtrip_all_orders() {
        let value = 0x0123_4567_89AB_CDEFu64;
        for (endian, word_order) in [
            (Endian::Big, WordOrder::MostSignificantFirst),
            (Endian::Big, WordOrder::LeastSignificantFirst),
            (Endian::Little, WordOrder::MostSignificantFirst),
            (Endian::Little, WordOrder::LeastSignificantFirst),
        ] {
            let regs = u64_to_registers(value, endian, word_order);
            assert_eq!(
                u64_from_registers(&regs, endian, word_order).unwrap(),
                value,
                "roundtrip failed for {:?} {:?}",
                endian,
                word_order
            );
        }
    }

    #[test]
    fn signed_roundtrips() {
        assert_eq!(
            i16_from_bytes(&i16_to_bytes(-1i16, Endian::Big), Endian::Big).unwrap(),
            -1
        );
        assert_eq!(
            i32_from_registers(
                &i32_to_registers(-2i32, Endian::Big, WordOrder::MostSignificantFirst),
                Endian::Big,
                WordOrder::MostSignificantFirst
            )
            .unwrap(),
            -2
        );
        assert_eq!(
            i64_from_registers(
                &i64_to_registers(-3i64, Endian::Little, WordOrder::LeastSignificantFirst),
                Endian::Little,
                WordOrder::LeastSignificantFirst
            )
            .unwrap(),
            -3
        );
    }

    #[test]
    fn f32_roundtrip() {
        let value = 3.1415925f32;
        let regs = f32_to_registers(value, Endian::Big, WordOrder::MostSignificantFirst);
        assert_eq!(
            f32_from_registers(&regs, Endian::Big, WordOrder::MostSignificantFirst).unwrap(),
            value
        );
    }

    #[test]
    fn f64_roundtrip() {
        let value = 2.718281828459045f64;
        let regs = f64_to_registers(value, Endian::Little, WordOrder::LeastSignificantFirst);
        assert_eq!(
            f64_from_registers(&regs, Endian::Little, WordOrder::LeastSignificantFirst).unwrap(),
            value
        );
    }

    #[test]
    fn string_big_endian() {
        // "Hi" in big-endian registers: 'H'=0x48, 'i'=0x69 -> reg = 0x4869
        let regs = [0x4869, 0x2100]; // "Hi!" plus NUL
        assert_eq!(string_from_registers(&regs, Endian::Big).unwrap(), "Hi!");
    }

    #[test]
    fn string_little_endian() {
        // "Hi" in little-endian registers: 'H'=0x48, 'i'=0x69 -> reg = 0x6948
        let regs = [0x6948, 0x0021]; // "Hi!" plus NUL
        assert_eq!(string_from_registers(&regs, Endian::Little).unwrap(), "Hi!");
    }

    #[test]
    fn string_to_registers_roundtrip() {
        let regs = string_to_registers("Hi!", Endian::Big, 0).unwrap();
        assert_eq!(regs, [0x4869, 0x2100]);
        assert_eq!(string_from_registers(&regs, Endian::Big).unwrap(), "Hi!");
    }

    #[test]
    fn string_to_registers_padding() {
        let regs = string_to_registers("Hi", Endian::Big, 4).unwrap();
        assert_eq!(regs, [0x4869, 0x0000, 0x0000, 0x0000]);
    }

    #[test]
    fn string_rejects_invalid_utf8() {
        let regs = [0xFFFD];
        assert!(matches!(
            string_from_registers(&regs, Endian::Big),
            Err(HelpersError::InvalidString)
        ));
    }

    #[test]
    fn string_to_registers_rejects_too_long_for_pad() {
        assert!(matches!(
            string_to_registers("Hello!", Endian::Big, 1),
            Err(HelpersError::InvalidLength)
        ));
    }
}
