//! Data store abstraction and an in-memory implementation for the synchronous
//! Modbus server.

#![cfg(any(feature = "sync", feature = "async"))]

use alloc::vec;
use alloc::vec::Vec;

use crate::exception::ExceptionCode;

/// A Modbus data store.
///
/// Implementations provide access to the four primary data tables: coils,
/// discrete inputs, holding registers, and input registers. All operations
/// return [`ExceptionCode::IllegalDataAddress`] when the requested address
/// range is out of bounds.
///
/// Advanced/diagnostic function codes (07/08/0B/0C/11/18) have default
/// implementations that return [`ExceptionCode::IllegalFunction`] so that
/// existing stores continue to compile; override the methods to support them.
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
    fn read_holding_registers(&self, address: u16, quantity: u16)
        -> Result<Vec<u8>, ExceptionCode>;

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

    /// Return the exception status byte for FC 0x07 Read Exception Status.
    fn read_exception_status(&self) -> Result<u8, ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    /// Handle FC 0x08 Diagnostics and return the echoed `(sub_function, data)`.
    fn diagnostics(&mut self, sub_function: u16, data: u16) -> Result<(u16, u16), ExceptionCode> {
        let _ = sub_function;
        let _ = data;
        Err(ExceptionCode::IllegalFunction)
    }

    /// Return `(status, event_count)` for FC 0x0B Get Comm Event Counter.
    fn get_comm_event_counter(&self) -> Result<(u16, u16), ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    /// Return `(status, event_count, message_count, events)` for FC 0x0C
    /// Get Comm Event Log.
    fn get_comm_event_log(&self) -> Result<(u16, u16, u16, Vec<u8>), ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    /// Return server identification data for FC 0x11 Report Server ID.
    fn report_server_id(&self) -> Result<Vec<u8>, ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }

    /// Return `(fifo_count, register_values)` for FC 0x18 Read FIFO Queue.
    fn read_fifo_queue(&self, _fifo_pointer_address: u16) -> Result<(u16, Vec<u8>), ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }
}

/// A simple in-memory [`DataStore`].
#[derive(Debug, Clone)]
pub struct MemoryStore {
    coils: Vec<bool>,
    discrete_inputs: Vec<bool>,
    holding_registers: Vec<u16>,
    input_registers: Vec<u16>,
    exception_status: u8,
    comm_event_counter: (u16, u16),
    comm_event_log: (u16, u16, u16, Vec<u8>),
    server_id: Vec<u8>,
    fifo_queue: (u16, Vec<u8>),
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
            exception_status: 0,
            comm_event_counter: (0, 0),
            comm_event_log: (0, 0, 0, vec![]),
            server_id: vec![],
            fifo_queue: (0, vec![]),
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

    fn read_holding_registers(
        &self,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ExceptionCode> {
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

    fn read_exception_status(&self) -> Result<u8, ExceptionCode> {
        Ok(self.exception_status)
    }

    fn diagnostics(&mut self, sub_function: u16, data: u16) -> Result<(u16, u16), ExceptionCode> {
        Ok((sub_function, data))
    }

    fn get_comm_event_counter(&self) -> Result<(u16, u16), ExceptionCode> {
        Ok(self.comm_event_counter)
    }

    fn get_comm_event_log(&self) -> Result<(u16, u16, u16, Vec<u8>), ExceptionCode> {
        let (status, event_count, message_count, events) = &self.comm_event_log;
        Ok((*status, *event_count, *message_count, events.clone()))
    }

    fn report_server_id(&self) -> Result<Vec<u8>, ExceptionCode> {
        Ok(self.server_id.clone())
    }

    fn read_fifo_queue(&self, _fifo_pointer_address: u16) -> Result<(u16, Vec<u8>), ExceptionCode> {
        let (count, values) = &self.fifo_queue;
        Ok((*count, values.clone()))
    }
}

impl MemoryStore {
    /// Write multiple input register values starting at `address`.
    ///
    /// Input registers are read-only over the wire; this helper is useful for
    /// setting up an in-memory store before serving requests.
    pub fn write_input_registers(
        &mut self,
        address: u16,
        values: &[u16],
    ) -> Result<(), ExceptionCode> {
        let end = address as usize + values.len();
        if end > self.input_registers.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        self.input_registers[address as usize..end].copy_from_slice(values);
        Ok(())
    }

    /// Set the value returned by FC 0x07 Read Exception Status.
    pub fn set_exception_status(&mut self, status: u8) {
        self.exception_status = status;
    }

    /// Set the values returned by FC 0x0B Get Comm Event Counter.
    pub fn set_comm_event_counter(&mut self, status: u16, event_count: u16) {
        self.comm_event_counter = (status, event_count);
    }

    /// Set the values returned by FC 0x0C Get Comm Event Log.
    pub fn set_comm_event_log(
        &mut self,
        status: u16,
        event_count: u16,
        message_count: u16,
        events: Vec<u8>,
    ) {
        self.comm_event_log = (status, event_count, message_count, events);
    }

    /// Set the data returned by FC 0x11 Report Server ID.
    pub fn set_server_id(&mut self, data: Vec<u8>) {
        self.server_id = data;
    }

    /// Set the data returned by FC 0x18 Read FIFO Queue.
    pub fn set_fifo_queue(&mut self, fifo_count: u16, register_values: Vec<u8>) {
        self.fifo_queue = (fifo_count, register_values);
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
