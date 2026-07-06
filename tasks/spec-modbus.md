# SPEC: Rust Modbus Protocol Stack

> Derived from: `tasks/prd-modbus.md`  
> Generated: 2026-07-06  
> Target: new greenfield crate `modbus`

## 1. Summary

### 1.1 What This SPEC Covers

This SPEC defines how to implement the Rust Modbus protocol stack described in
the PRD as a **single crate** with Cargo feature flags. It covers:

- A `no_std` + `alloc` PDU core (function-code request/response types,
  exception codes).
- Transport framers for RTU (CRC-16), ASCII (LRC-8), TCP (MBAP), and UDP
  (MBAP-like).
- Synchronous and asynchronous client APIs.
- Synchronous and asynchronous server APIs with a pluggable `DataStore` trait.
- Register-to-primitive data-type conversion helpers.
- A minimal CLI binary for ad-hoc testing.
- Unit and integration tests, including optional loopback serial tests.

### 1.2 PRD Reference

- Source: `tasks/prd-modbus.md`
- User Stories covered: US-001 through US-016
- Functional Requirements covered: FR-1 through FR-12

### 1.3 Design Decisions Summary

| Decision | Choice | Rationale |
|---|---|---|
| Crate layout | Single crate with feature flags | Simpler publishing and dependency management; aligns with user choice |
| Async runtime | Tokio for `async` feature | De-facto Rust async ecosystem; sync API remains runtime-free |
| Serial I/O | Trait abstraction + optional `tokio-serial` impl | Keeps core testable/mockable while providing out-of-the-box serial support |
| Server data model | `DataStore` trait + in-memory default | Separates protocol logic from application memory layout |
| Error handling | `thiserror`/`core` enum per layer | Rich errors with `std::error::Error` behind `std` feature |
| MSRV | Rust 1.70 | Balances modern features with stability |

---

## 2. Architecture

### 2.1 System Context

The crate is a library consumed by two classes of users:

1. **Embedded/protocol developers** — use the `core` + `rtu`/`tcp` features
   directly; no `std`, no runtime.
2. **Application developers** — enable `std`, `client`, `server`, `async` to
   build full clients or servers.

A small CLI binary is provided under the `cli` feature for manual testing.

### 2.2 Component Design

```
┌─────────────────────────────────────────────────────────────┐
│                         modbus crate                         │
├─────────────┬─────────────┬─────────────┬───────────────────┤
│   pdu/core  │    adu      │   client    │      server       │
│  (no_std)   │  (no_std)   │  (std)      │      (std)        │
├─────────────┼─────────────┼─────────────┼───────────────────┤
│  exception  │    rtu      │    sync     │       sync        │
│  function   │    ascii    │    async    │       async       │
│  helpers    │    tcp      │             │                   │
│             │    udp      │             │                   │
└─────────────┴─────────────┴─────────────┴───────────────────┘
```

### 2.3 Module Interactions

1. **PDU layer** produces/consumes byte slices representing Modbus PDUs.
2. **ADU layer** wraps PDUs with transport-specific headers/trailers.
3. **Client** builds a Request PDU → ADU → sends over transport → parses
   response ADU → PDU.
4. **Server** receives ADU → parses PDU → dispatches to `DataStore` → builds
   response PDU → ADU.

### 2.4 File Structure

```
src/
├── lib.rs                  feature flags, re-exports
├── error.rs                EncodeError, DecodeError, AduError, ClientError, ServerError
├── function.rs             FunctionCode enum + constants
├── exception.rs            ExceptionCode enum + response encoding
├── pdu.rs                  Request / Response enums
├── function_codes/
│   ├── mod.rs
│   ├── read_coils.rs
│   ├── read_discrete_inputs.rs
│   ├── read_holding_registers.rs
│   ├── read_input_registers.rs
│   ├── write_single_coil.rs
│   ├── write_single_register.rs
│   ├── write_multiple_coils.rs
│   ├── write_multiple_registers.rs
│   ├── read_exception_status.rs
│   ├── diagnostics.rs
│   ├── get_comm_event_counter.rs
│   ├── get_comm_event_log.rs
│   ├── report_server_id.rs
│   ├── read_file_record.rs
│   ├── write_file_record.rs
│   ├── mask_write_register.rs
│   ├── read_write_multiple_registers.rs
│   ├── read_fifo_queue.rs
│   └── encapsulated_interface_transport.rs
├── adu.rs                  ADU trait + framing helpers
├── rtu.rs                  RTU frame + CRC-16
├── ascii.rs                ASCII frame + LRC-8
├── tcp.rs                  MBAP header for TCP
├── udp.rs                  MBAP header for UDP
├── helpers.rs              register ↔ primitive conversions
├── client/
│   ├── mod.rs
│   ├── sync.rs             sync Client<T: Transport>
│   └── async.rs            async AsyncClient<T: AsyncTransport>
├── server/
│   ├── mod.rs
│   ├── store.rs            DataStore trait + in-memory impl
│   ├── sync.rs             sync Server
│   └── async.rs            async Server
├── transport/
│   ├── mod.rs              Transport / AsyncTransport traits
│   └── serial.rs           optional tokio-serial wrapper
└── bin/
    └── cli.rs              CLI binary (feature = "cli")
tests/
├── roundtrip_pdu.rs
├── framing.rs
├── helpers.rs
├── sync_client_server.rs
├── async_client_server.rs
└── serial_loopback.rs      feature = "loopback-tests"
examples/
├── tcp_client.rs
├── tcp_server.rs
└── rtu_client.rs
```

---

## 3. Data Model

### 3.1 Data Store Model

The server does not own a fixed memory map. Instead, it relies on a
`DataStore` trait:

```rust
pub trait DataStore {
    fn read_coils(&self, addr: u16, qty: u16) -> Result<Vec<bool>, ExceptionCode>;
    fn read_discrete_inputs(&self, addr: u16, qty: u16) -> Result<Vec<bool>, ExceptionCode>;
    fn write_coil(&mut self, addr: u16, value: bool) -> Result<(), ExceptionCode>;
    fn write_coils(&mut self, addr: u16, values: &[bool]) -> Result<(), ExceptionCode>;

    fn read_holding_registers(&self, addr: u16, qty: u16) -> Result<Vec<u16>, ExceptionCode>;
    fn read_input_registers(&self, addr: u16, qty: u16) -> Result<Vec<u16>, ExceptionCode>;
    fn write_register(&mut self, addr: u16, value: u16) -> Result<(), ExceptionCode>;
    fn write_registers(&mut self, addr: u16, values: &[u16]) -> Result<(), ExceptionCode>;

    fn read_exception_status(&self) -> Result<u8, ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }
    fn diagnostics(&self, _sf: u16, _data: &[u8]) -> Result<Vec<u8>, ExceptionCode> {
        Err(ExceptionCode::IllegalFunction)
    }
    // ... other functions with default IllegalFunction
}
```

### 3.2 In-Memory Default Store

```rust
pub struct MemoryStore {
    coils: Vec<bool>,
    discrete_inputs: Vec<bool>,
    holding_registers: Vec<u16>,
    input_registers: Vec<u16>,
}
```

Constructor takes capacity per table.

### 3.3 Relationships

- Client owns a `Transport` and optionally a unit/slave ID.
- Server owns a `DataStore` and a transport listener.
- CLI wires a `MemoryStore` to an async TCP/RTU server, or acts as a client.

---

## 4. API Design

### 4.1 Core PDU API

Every function code has a request and response struct:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ReadCoilsRequest {
    pub starting_address: u16,
    pub quantity: u16,
}

impl ReadCoilsRequest {
    pub const FUNCTION_CODE: u8 = 0x01;
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
    pub fn decode(buf: &[u8]) -> Result<Self, DecodeError>;
}
```

Aggregated enums:

```rust
pub enum Request {
    ReadCoils(ReadCoilsRequest),
    ReadDiscreteInputs(ReadDiscreteInputsRequest),
    // ...
    EncapsulatedInterfaceTransport(EncapsulatedInterfaceTransportRequest),
}

pub enum Response {
    ReadCoils(ReadCoilsResponse),
    // ...
    Exception(ExceptionResponse),
}
```

### 4.2 ADU API

```rust
pub trait Adu {
    type Error;
    fn encode(&self, buf: &mut [u8]) -> Result<usize, Self::Error>;
    fn decode(buf: &[u8]) -> Result<Self, Self::Error>;
}

pub struct RtuAdu { pub address: u8, pub pdu: Vec<u8> }
pub struct AsciiAdu { pub address: u8, pub pdu: Vec<u8> }
pub struct TcpAdu { pub transaction_id: u16, pub unit_id: u8, pub pdu: Vec<u8> }
pub struct UdpAdu { pub transaction_id: u16, pub unit_id: u8, pub pdu: Vec<u8> }
```

### 4.3 Client API

Sync:

```rust
pub struct Client<T: Transport> { /* ... */ }

impl<T: Transport> Client<T> {
    pub fn new(transport: T, config: ClientConfig) -> Self;
    pub fn read_coils(&mut self, slave: u8, addr: u16, qty: u16)
        -> Result<Vec<bool>, ClientError>;
    // ... one method per public function code
}
```

Async:

```rust
pub struct AsyncClient<T: AsyncTransport> { /* ... */ }

impl<T: AsyncTransport + Unpin> AsyncClient<T> {
    pub fn new(transport: T, config: ClientConfig) -> Self;
    pub async fn read_coils(&mut self, slave: u8, addr: u16, qty: u16)
        -> Result<Vec<bool>, ClientError>;
}
```

### 4.4 Server API

```rust
pub struct ServerConfig {
    pub slave_id: u8,
    pub broadcast_supported: bool,
}

pub struct Server<S: DataStore, T: Listener> { /* ... */ }

impl<S: DataStore, T: Listener> Server<S, T> {
    pub fn new(store: S, config: ServerConfig) -> Self;
    pub fn run(&mut self) -> Result<(), ServerError>;          // sync
    pub async fn run(&mut self) -> Result<(), ServerError>;     // async
}
```

### 4.5 Transport Traits

```rust
pub trait Transport {
    fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
    fn recv(&mut self, buf: &mut [u8], timeout: Duration) -> Result<usize, TransportError>;
}

#[cfg(feature = "async")]
pub trait AsyncTransport {
    async fn send(&mut self, data: &[u8]) -> Result<(), TransportError>;
    async fn recv(&mut self, buf: &mut [u8], timeout: Duration)
        -> Result<usize, TransportError>;
}
```

### 4.6 Helpers API

```rust
pub enum Endian { Big, Little }
pub enum WordOrder { Ab, Ba }

pub fn u16_to_registers(v: u16) -> [u8; 2];
pub fn u32_to_registers(v: u32, order: WordOrder) -> [u16; 2];
pub fn f32_to_registers(v: f32, endian: Endian, order: WordOrder) -> [u16; 2];
pub fn registers_to_string(regs: &[u16]) -> Result<String, HelperError>;
// etc.
```

---

## 5. Business Logic

### 5.1 Request/Response Dispatch

Pseudocode for server dispatch:

```text
receive ADU
parse ADU -> (address, pdu_bytes)
if address != self.slave_id and address != 0:
    ignore (RTU/ASCII) or return gateway exception (TCP)
decode pdu_bytes -> Request
match Request:
    ReadCoils(req) =>
        validate quantity 1..=2000
        store.read_coils(req.starting_address, req.quantity)
        build ReadCoilsResponse -> encode -> ADU response
    // ...
    _ => return ExceptionResponse(IllegalFunction)
```

### 5.2 Validation Rules

- Coil/input quantities: 1–2000.
- Register quantities: 1–125 for reads, 1–123 for writes (per function code).
- Starting address + quantity must not overflow `u16::MAX + 1`.
- PDU length must not exceed 253 bytes.
- MBAP length field = unit ID (1) + PDU length.

### 5.3 Edge Cases

- **Broadcast (address 0)**: server executes write but does not respond; client
  must not wait for response.
- **TCP transaction ID**: server echoes transaction ID; client matches responses
  by transaction ID.
- **ASCII frame**: ignore leading whitespace before colon; validate CR/LF
  terminator.
- **Partial RTU frame**: caller handles serial timing; parser returns
  `WouldBlock` if incomplete.

---

## 6. Error Handling

### 6.1 Error Taxonomy

| Layer | Error Type | Conditions |
|---|---|---|
| PDU encode | `EncodeError::BufferTooSmall` | Output buffer too short |
| PDU decode | `DecodeError::InvalidLength` | Unexpected byte count |
| PDU decode | `DecodeError::UnknownFunctionCode` | Function code not supported |
| ADU | `AduError::CrcMismatch` | RTU CRC check failed |
| ADU | `AduError::LrcMismatch` | ASCII LRC check failed |
| ADU | `AduError::InvalidAscii` | Non-hex character in ASCII frame |
| Client | `ClientError::Timeout` | No response within configured timeout |
| Client | `ClientError::Exception(ExceptionResponse)` | Server returned exception |
| Server | `ServerError::Transport(...)` | Listener I/O failure |

### 6.2 Retry Strategy

Client retries are out of scope for the core library; callers configure
transport-level retries or implement their own.

---

## 7. Security

### 7.1 Input Validation

- All decoded lengths are checked against the remaining buffer size.
- All quantities are checked against spec limits.
- Server `DataStore` implementations are responsible for access control.

### 7.2 Data Protection

- No encryption or authentication is provided; out of scope per PRD.
- Transport uses plain TCP/UDP/serial as specified by Modbus.

---

## 8. Performance

### 8.1 Expected Load

- Embedded targets: single-frame operations, no heap pressure in hot path.
- Server applications: up to thousands of concurrent TCP connections (bounded
  by tokio runtime).

### 8.2 Optimization Strategy

- Core encoding uses stack buffers where possible; `alloc::vec::Vec` only when
  response size is dynamic.
- Server dispatches requests without per-request allocations for fixed-size
  responses.
- Async server uses one tokio task per TCP connection.

### 8.3 No-std Constraints

- No `std::collections::HashMap` in core; use `Vec` and fixed-size arrays.
- No `std::io`; define custom transport traits.

---

## 9. Testing Strategy

### 9.1 Unit Tests

One test per function code verifying encode → decode round-trip and edge-case
bounds.

### 9.2 Framer Tests

- RTU CRC known vectors.
- ASCII LRC known vectors.
- MBAP header encode/decode.
- Malformed frame rejection.

### 9.3 Client/Server Integration Tests

- In-memory transport implementing `Transport`/`AsyncTransport`.
- TCP client/server using `tokio::net::TcpListener`/`TcpStream`.
- Serial mock transport using `VecDeque<u8>`.

### 9.4 Loopback Serial Tests

Behind feature `loopback-tests`:

- Requires two physical or virtual serial ports cross-connected.
- Tests RTU and ASCII round-trip at 9600 baud.
- Marked `#[ignore]` unless feature enabled.

### 9.5 Acceptance Criteria Mapping

| US/FR | Test | Type | Description |
|---|---|---|---|
| US-001 | `read_coils_roundtrip` | unit | FC 01 encode/decode |
| US-007 | `rtu_crc_known_vectors` | unit | CRC-16 validation |
| US-011 | `tcp_client_read_holding_registers` | integration | Full client/server TCP |
| US-012 | `rtu_client_loopback` | integration | Real serial loopback |
| FR-1 | `no_std_build` | build | `cargo build --no-default-features` |

---

## 10. Implementation Plan

### 10.1 Phases

| Phase | Focus | PRD Items | Output |
|---|---|---|---|
| 1 | PDU core + all function codes | US-001..006 | `no_std` core builds, all PDU tests pass |
| 2 | ADU framers | US-007..009 | RTU/ASCII/TCP/UDP framing |
| 3 | Helpers | US-010 | Endianness/word-order helpers |
| 4 | Sync client | US-011, US-012 (partial) | TCP/RTU/ASCII sync client |
| 5 | Async client | US-011, US-012 (partial) | Tokio-based async client |
| 6 | Sync server | US-013, US-014 (partial) | In-memory sync server |
| 7 | Async server | US-013, US-014 (partial) | Tokio-based async server |
| 8 | UDP | US-015 | UDP client/server |
| 9 | CLI + examples | US-016 | `modbus-cli` binary |
| 10 | Integration + loopback | US-016 | Full test suite green |

### 10.2 Dependencies by Feature

| Feature | Dependencies |
|---|---|
| `core` (default) | `alloc` |
| `std` | `std` |
| `async` | `tokio` |
| `serial` | `tokio-serial` |
| `helpers` | `alloc` |
| `cli` | `clap`, `tokio`, `tracing` |

### 10.3 Cargo.toml Sketch

```toml
[package]
name = "modbus"
version = "0.1.0"
edition = "2021"

[features]
default = ["std", "rtu", "ascii", "tcp", "udp", "sync", "helpers"]
std = []
rtu = []
ascii = []
tcp = []
udp = []
sync = ["std"]
async = ["std", "dep:tokio"]
serial = ["async", "dep:tokio-serial"]
helpers = []
cli = ["std", "async", "tcp", "rtu", "dep:clap", "dep:tracing"]
loopback-tests = ["rtu", "ascii", "serial"]

[dependencies]
tokio = { version = "1", features = ["net", "rt", "io-util", "time"], optional = true }
tokio-serial = { version = "5", optional = true }
clap = { version = "4", features = ["derive"], optional = true }
tracing = { version = "0.1", optional = true }

[dev-dependencies]
tokio = { version = "1", features = ["full"] }
tokio-test = "0.4"
```

---

## 11. Open Questions & Risks

### 11.1 Unresolved Questions

- Should the CLI use subcommands (`client`, `server`) or positional args?
- Should `DataStore` methods take `&self` or `&mut self`? Mixed immutable reads
  and mutable writes may require interior mutability in user implementations.

### 11.2 Technical Risks

| Risk | Impact | Mitigation |
|---|---|---|
| Single crate becomes large | Medium | Keep modules isolated; consider splitting into workspace later if compile times suffer |
| `no_std` + async trait compatibility | Low | Use `async-trait` or RPITIT with MSRV 1.75 if needed |
| Serial loopback tests flaky | Low | Mark `#[ignore]` by default; run only in CI with virtual ports |

### 11.3 Assumptions

- Tokio remains the async runtime of choice.
- Users who need only PDU parsing will disable default features.
- Serial loopback tests require OS-level virtual serial ports or physical hardware.
