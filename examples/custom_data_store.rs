//! Custom file-backed [`DataStore`] example.
//!
//! Build with: `cargo build --example custom_data_store --features sync,tcp,config`
//! Run with a path to a JSON file; the file is created if it does not exist.
//!
//! ```sh
//! cargo run --example custom_data_store --features sync,tcp,config -- store.json
//! ```
//!
//! This example demonstrates how to implement [`DataStore`] without forking the
//! crate. The store keeps the four Modbus tables in memory and persists the
//! entire snapshot to disk after every write. This is intentionally simple and
//! not optimized for high throughput; a real implementation might use a WAL,
//! append-only log, or embed a real database.

use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use modbus::exception::ExceptionCode;
use modbus::server::DataStore;
use modbus::tcp_server::TcpServer;

/// A simple file-backed data store.
///
/// All data is held in memory and persisted as JSON after each write. Reads are
/// served from memory.
#[derive(Debug, Clone)]
struct FileBackedStore {
    path: PathBuf,
    coils: Vec<bool>,
    discrete_inputs: Vec<bool>,
    holding_registers: Vec<u16>,
    input_registers: Vec<u16>,
}

/// Serializable snapshot of the store tables.
#[derive(Debug, Serialize, Deserialize)]
struct Snapshot {
    coils: Vec<bool>,
    discrete_inputs: Vec<bool>,
    holding_registers: Vec<u16>,
    input_registers: Vec<u16>,
}

impl FileBackedStore {
    /// Create a new store with the given table sizes and persist it to `path`.
    pub fn new(
        path: impl AsRef<Path>,
        coils: usize,
        discrete_inputs: usize,
        holding_registers: usize,
        input_registers: usize,
    ) -> Result<Self, io::Error> {
        let store = Self {
            path: path.as_ref().to_path_buf(),
            coils: vec![false; coils],
            discrete_inputs: vec![false; discrete_inputs],
            holding_registers: vec![0; holding_registers],
            input_registers: vec![0; input_registers],
        };
        store.save()?;
        Ok(store)
    }

    /// Load a store from `path`, or create it with the given sizes if missing.
    pub fn load_or_create(
        path: impl AsRef<Path>,
        coils: usize,
        discrete_inputs: usize,
        holding_registers: usize,
        input_registers: usize,
    ) -> Result<Self, io::Error> {
        let path = path.as_ref();
        if path.exists() {
            let mut file = fs::File::open(path)?;
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let snapshot: Snapshot = serde_json::from_str(&contents).map_err(|e| {
                io::Error::new(io::ErrorKind::InvalidData, format!("invalid store file: {e}"))
            })?;
            let store = Self {
                path: path.to_path_buf(),
                coils: snapshot.coils,
                discrete_inputs: snapshot.discrete_inputs,
                holding_registers: snapshot.holding_registers,
                input_registers: snapshot.input_registers,
            };
            // Normalize sizes in case the file was edited by hand.
            store.save()?;
            Ok(store)
        } else {
            Self::new(path, coils, discrete_inputs, holding_registers, input_registers)
        }
    }

    /// Persist the current tables to disk.
    fn save(&self) -> Result<(), io::Error> {
        let snapshot = Snapshot {
            coils: self.coils.clone(),
            discrete_inputs: self.discrete_inputs.clone(),
            holding_registers: self.holding_registers.clone(),
            input_registers: self.input_registers.clone(),
        };
        let contents = serde_json::to_string_pretty(&snapshot)?;
        let mut file = fs::File::create(&self.path)?;
        file.write_all(contents.as_bytes())?;
        file.flush()?;
        Ok(())
    }
}

impl DataStore for FileBackedStore {
    fn read_coils(&self, address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        MemoryStoreView::read_coils(&self.coils, address, quantity)
    }

    fn read_discrete_inputs(
        &self,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ExceptionCode> {
        MemoryStoreView::read_coils(&self.discrete_inputs, address, quantity)
    }

    fn read_holding_registers(
        &self,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ExceptionCode> {
        MemoryStoreView::read_registers(&self.holding_registers, address, quantity)
    }

    fn read_input_registers(
        &self,
        address: u16,
        quantity: u16,
    ) -> Result<Vec<u8>, ExceptionCode> {
        MemoryStoreView::read_registers(&self.input_registers, address, quantity)
    }

    fn write_coil(&mut self, address: u16, value: bool) -> Result<(), ExceptionCode> {
        MemoryStoreView::write_coil(&mut self.coils, address, value)?;
        self.save().map_err(|_| ExceptionCode::ServerDeviceFailure)
    }

    fn write_register(&mut self, address: u16, value: u16) -> Result<(), ExceptionCode> {
        MemoryStoreView::write_register(&mut self.holding_registers, address, value)?;
        self.save().map_err(|_| ExceptionCode::ServerDeviceFailure)
    }

    fn write_coils(&mut self, address: u16, values: &[bool]) -> Result<(), ExceptionCode> {
        MemoryStoreView::write_coils(&mut self.coils, address, values)?;
        self.save().map_err(|_| ExceptionCode::ServerDeviceFailure)
    }

    fn write_registers(
        &mut self,
        address: u16,
        values: &[u16],
    ) -> Result<(), ExceptionCode> {
        MemoryStoreView::write_registers(&mut self.holding_registers, address, values)?;
        self.save().map_err(|_| ExceptionCode::ServerDeviceFailure)
    }
}

/// Helper implementing the same bit/word conversions as [`MemoryStore`].
struct MemoryStoreView;

impl MemoryStoreView {
    fn read_coils(table: &[bool], address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let start = address as usize;
        let end = start.saturating_add(quantity as usize);
        if end > table.len() || quantity == 0 {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        let mut bytes = vec![0u8; (quantity as usize).div_ceil(8)];
        for (i, value) in table[start..end].iter().enumerate() {
            if *value {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }
        Ok(bytes)
    }

    fn read_registers(table: &[u16], address: u16, quantity: u16) -> Result<Vec<u8>, ExceptionCode> {
        let start = address as usize;
        let end = start.saturating_add(quantity as usize);
        if end > table.len() || quantity == 0 {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        let mut bytes = Vec::with_capacity(quantity as usize * 2);
        for value in &table[start..end] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        Ok(bytes)
    }

    fn write_coil(table: &mut [bool], address: u16, value: bool) -> Result<(), ExceptionCode> {
        let idx = address as usize;
        if idx >= table.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        table[idx] = value;
        Ok(())
    }

    fn write_register(
        table: &mut [u16],
        address: u16,
        value: u16,
    ) -> Result<(), ExceptionCode> {
        let idx = address as usize;
        if idx >= table.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        table[idx] = value;
        Ok(())
    }

    fn write_coils(
        table: &mut [bool],
        address: u16,
        values: &[bool],
    ) -> Result<(), ExceptionCode> {
        let start = address as usize;
        let end = start.saturating_add(values.len());
        if end > table.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        table[start..end].copy_from_slice(values);
        Ok(())
    }

    fn write_registers(
        table: &mut [u16],
        address: u16,
        values: &[u16],
    ) -> Result<(), ExceptionCode> {
        let start = address as usize;
        let end = start.saturating_add(values.len());
        if end > table.len() {
            return Err(ExceptionCode::IllegalDataAddress);
        }
        table[start..end].copy_from_slice(values);
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "modbus_store.json".to_string());

    let store = FileBackedStore::load_or_create(&path, 16, 0, 10, 0)?;
    println!("File-backed Modbus store using {path}");

    let mut server = TcpServer::new(store);
    let listener = std::net::TcpListener::bind("127.0.0.1:502")?;
    println!("Modbus TCP server listening on 127.0.0.1:502");

    for stream in listener.incoming() {
        let mut stream = stream?;
        if let Err(e) = server.serve(&mut stream, 1) {
            eprintln!("connection closed: {e}");
        }
    }

    Ok(())
}
