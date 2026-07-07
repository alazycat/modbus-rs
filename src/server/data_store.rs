//! Data store abstraction and an in-memory implementation for the synchronous
//! Modbus server.

#![cfg(feature = "sync")]

use alloc::vec;
use alloc::vec::Vec;

use crate::exception::ExceptionCode;

/// A Modbus data store.
///
/// Implementations provide access to the four primary data tables: coils,
/// discrete inputs, holding registers, and input registers. All operations
/// return [`ExceptionCode::IllegalDataAddress`] when the requested address
/// range is out of bounds.
pub trait DataStore {
    /// Read `quantity` coil values starting at `address`.
    ///
    /// The returned bytes pack one bit per coil, with the LSB of the first
    /// byte holding the coil at `address`. Trailing high bits are zero-filled.
    fn read_coils(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode>;

    /// Read `quantity` discrete input values starting at `address`.
    fn read_discrete_inputs(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode>;

    /// Read `quantity` holding registers starting at `address`.
    ///
    /// Each register occupies two big-endian bytes in the returned vector.
    fn read_holding_registers(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode>;

    /// Read `quantity` input registers starting at `address`.
    fn read_input_registers(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode>;

    /// Write a single coil value at `address`.
    fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ExceptionCode>;

    /// Write a single holding register value at `address`.
    fn write_register(&mut self, address: u16, value: u16) -> Result<(), ExceptionCode>;

    /// Write multiple coil values starting at `address`.
    fn write_coils(&mut self, address: u16, values: &[bool]) -> Result<(), ExceptionCode>;

    /// Write multiple holding register values starting at `address`.
    fn write_registers(&mut self, address: u16, values: &[u16]) -> Result<(), ExceptionCode>;
}

/// A simple in-memory [`DataStore`].
#[derive(Debug, Clone)]
pub struct MemoryStore {
    coils: Vec<bool>,
    discrete_inputs: Vec<bool>,
    holding_registers: Vec<u16>,
    input_registers: Vec<u16>,
}

impl MemoryStore {
    /// Create a new store with the given table sizes.
    pub fn new(
        num_coils: u16,
        num_discrete_inputs: u16,
        num_holding_registers: u16,
        num_input_registers: u16,
    ) -> Self {
        Self {
            coils: vec![false; num_coils as usize],
            discrete_inputs: vec![false; num_discrete_inputs as usize],
            holding_registers: vec![0; num_holding_registers as usize],
            input_registers: vec![0; num_input_registers as usize],
        }
    }
}

impl DataStore for MemoryStore {
    fn read_coils(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let end = address as usize + quantity as usize;
        if end > self.coils.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        Ok(pack_bits(&self.coils[address as usize..end]))
    }

    fn read_discrete_inputs(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let end = address as usize + quantity as usize;
        if end > self.discrete_inputs.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        Ok(pack_bits(&self.discrete_inputs[address as usize..end]))
    }

    fn read_holding_registers(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let end = address as usize + quantity as usize;
        if end > self.holding_registers.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        let mut bytes = Vec::with_capacity(quantity as usize * 2);
        for &value in &self.holding_registers[address as usize..end] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        Ok(bytes)
    }

    fn read_input_registers(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let end = address as usize + quantity as usize;
        if end > self.input_registers.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        let mut bytes = Vec::with_capacity(quantity as usize * 2);
        for &value in &self.input_registers[address as usize..end] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        Ok(bytes)
    }

    fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ExceptionCode> {
        let slot = self
            .coils
            .get_mut(address as usize)
            .ok_or(ExceptionCode::IllegalDataAddress)?;
        *slot = value;
        Ok(())
    }

    fn write_register(&mut self, address: u16, value: u16) -> Result<(), ExceptionCode> {
        let slot = self
            .holding_registers
            .get_mut(address as usize)
            .ok_or(ExceptionCode::IllegalDataAddress)?;
        *slot = value;
        Ok(())
    }

    fn write_coils(&mut self, address: u16, values: &[bool]) -> Result<(), ExceptionCode> {
        let end = address as usize + values.len();
        if end > self.coils.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        self.coils[address as usize..end].copy_from_slice(values);
        Ok(())
    }

    fn write_registers(&mut self, address: u16, values: &[u16]) -> Result<(), ExceptionCode> {
        let end = address as usize + values.len();
        if end > self.holding_registers.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        self.holding_registers[address as usize..end].copy_from_slice(values);
        Ok(())
    }
}

fn pack_bits(bits: &[bool]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(bits.len().div_ceil(8));
    for (i, &bit) in bits.iter().enumerate() {
        if i % 8 == 0 {
            bytes.push(0);
        }
        if bit {
            let last = bytes.last_mut().expect("byte was just pushed");
            *last |= 1 << (i % 8);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_coils_packs_bits() {
        let mut store = MemoryStore::new(10, 0, 0, 0);
        store.write_coils(0, &[true, false, true, true]).unwrap();
        let bytes = store.read_coils(0, 4).unwrap();
        assert_eq!(bytes, vec![0b00001101]);
    }

    #[test]
    fn read_coils_zero_fills_trailing_bits() {
        let mut store = MemoryStore::new(10, 0, 0, 0);
        store.write_coils(0, &[true, true]).unwrap();
        let bytes = store.read_coils(0, 5).unwrap();
        assert_eq!(bytes, vec![0b00000011]);
    }

    #[test]
    fn read_holding_registers_big_endian() {
        let mut store = MemoryStore::new(0, 0, 2, 0);
        store.write_registers(0, &[0x0123, 0x4567]).unwrap();
        let bytes = store.read_holding_registers(0, 2).unwrap();
        assert_eq!(bytes, vec![0x01, 0x23, 0x45, 0x67]);
    }

    #[test]
    fn out_of_range_read_returns_illegal_data_address() {
        let store = MemoryStore::new(8, 0, 0, 0);
        assert_eq!(
            store.read_coils(0, 9),
            Err(ExceptionCode::IllegalDataAddress)
        );
    }

    #[test]
    fn out_of_range_write_returns_illegal_data_address() {
        let mut store = MemoryStore::new(0, 0, 1, 0);
        assert_eq!(
            store.write_register(1, 0x00FF),
            Err(ExceptionCode::IllegalDataAddress)
        );
    }
}
