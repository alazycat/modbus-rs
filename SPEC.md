# Spec: Rust Modbus Protocol Implementation

## Objective

Implement a Rust library (`modbus`) that provides a complete, no-std compatible
implementation of the **Modbus Application Protocol** as defined in
*MODBUS Application Protocol Specification V1.1b3* (April 26, 2012),
plus the two most common transport encodings:

- **PDU layer**: encode/decode every public function code request/response and
  exception response.
- **RTU serial line framing**: slave address + PDU + CRC-16.
- **TCP framing**: MBAP header + PDU.

The library is intended to be embedded in both clients and servers. It is
**not** a full networked runtime; transport I/O is left to the caller.

## Tech Stack

- Language: Rust (Edition 2021)
- `no_std` + `alloc` by default; an optional `std` feature enables `std::error::Error`
- No external runtime dependencies for the core protocol
- Dev dependency: standard Rust test harness (`cargo test`)

## Commands

```sh
# Build
cargo build

# Run tests
cargo test

# Build no-std variant
cargo build --no-default-features

# Format / lint
cargo fmt
cargo clippy --all-targets
```

## Project Structure

```
modbus/
├── Cargo.toml
├── SPEC.md                 ← this document
├── src/
│   ├── lib.rs              ← crate root, feature flags, re-exports
│   ├── pdu.rs              ← PDU types, Request/Response enums
│   ├── function_codes.rs   ← per-function request/response structs
│   ├── exception.rs        ← ExceptionCode enum + encoding
│   ├── adu.rs              ← ADU trait + RTU/TCP wrappers
│   ├── rtu.rs              ← RTU frame builder/parser + CRC-16
│   ├── tcp.rs              ← MBAP header builder/parser
│   └── util.rs             ← big-endian helpers, coil packing
└── tests/
    ├── roundtrip.rs        ← encode/decode round trips for every function code
    ├── rtu.rs              ← RTU framing + CRC tests
    └── tcp.rs              ← MBAP header tests
```

## Code Style

- Errors are represented as `#[derive(Debug, Clone, PartialEq)]` enums.
- All multi-byte values are big-endian on the wire (per spec §4.2).
- PDU addresses are `u16` and zero-based; quantities are `u16`.
- Coils/discrete inputs are packed LSB-first per spec §6.1.
- Function code constants live in `function_codes.rs` and use `u8` hex values.

Example shape:

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

## Testing Strategy

- One unit test per public function code covering a representative encode/decode
  round trip.
- Boundary tests for maximum quantities and zero-padded coil bytes.
- Exception response encode/decode tests.
- RTU CRC tests using known vectors.
- MBAP header encode/decode tests.
- `cargo test` must pass before any task is considered complete.

## Boundaries

- **Always do**: validate input ranges against spec limits, return spec-mandated
  exception codes for invalid requests, keep the core crate `no_std` compatible.
- **Ask first**: adding external dependencies, changing the public API shape,
  implementing a networked client/server runtime on top of the library.
- **Never do**: implement vendor-specific function codes, add transport I/O
  (serial/TCP sockets) in the core crate, or skip tests for new function codes.

## Success Criteria

1. `cargo test` passes with 100% of public function codes (§5.1) covered by
   round-trip tests.
2. The following function codes are implemented:
   - Bit access: `0x01` Read Coils, `0x02` Read Discrete Inputs,
     `0x05` Write Single Coil, `0x0F` Write Multiple Coils.
   - 16-bit access: `0x03` Read Holding Registers, `0x04` Read Input Registers,
     `0x06` Write Single Register, `0x10` Write Multiple Registers,
     `0x17` Read/Write Multiple Registers, `0x16` Mask Write Register,
     `0x18` Read FIFO Queue.
   - File record access: `0x14` Read File Record, `0x15` Write File Record.
   - Diagnostics/serial: `0x07` Read Exception Status, `0x08` Diagnostics,
     `0x0B` Get Comm Event Counter, `0x0C` Get Comm Event Log,
     `0x11` Report Server ID.
   - Encapsulated: `0x2B` Encapsulated Interface Transport with
     `0x0E` Read Device Identification and `0x0D` CANopen General Reference.
3. Exception responses for codes `0x01`–`0x0B` encode and decode correctly.
4. RTU frames compute and verify CRC-16 correctly.
5. TCP MBAP headers encode/decode correctly.
6. `cargo build --no-default-features` succeeds (no-std core).

## Comparison with Other Language Implementations

A survey of popular open-source Modbus libraries shows what "complete"
typically means in practice. The table below is derived from public documentation
and source code of representative implementations.

| Implementation | Language | Transports | Function Codes | Client | Server | Notable extras |
|---|---|---|---|---|---|---|
| **libmodbus** | C | RTU, TCP | 01–07, 0F, 10, 11, 16, 17 | ✅ | ❌ | Raw request API, widely used |
| **pymodbus** | Python | TCP, UDP, RTU, ASCII, TLS | Full public set incl. 08, 14, 15, 18, 2B/MEI | ✅ sync+async | ✅ sync+async | Simulator, REPL, custom FCs |
| **NModbus** | C# | RTU, ASCII, TCP, UDP | 01–06, 0F, 10, 17 | ✅ | ✅ | Custom FC handlers |
| **jamod** | Java | TCP, RTU, ASCII | 01–06, 0F, 10 | ✅ | ✅ | TCP connection pooling |
| **modbus4j** | Java | ASCII, RTU, TCP, UDP | 01–06, 0F–11, 16, 17 | ✅ | ✅ | Auto request partitioning |
| **tokio-modbus** | Rust | TCP, RTU | 01–06, 0F, 10, 17 (client API) | ✅ async | ✅ skeleton | Most popular Rust crate |
| **voltage_modbus** | Rust | TCP, RTU | 01–06, 0F, 10 | ✅ async | ❌ | `no_std` core, pipelining |
| **rmodbus** | Rust | TCP, UDP, RTU, ASCII | Any (low-level frame builder) | ❌ | ❌ | `no_std`, codec-only |
| **modbus-go** | Go | TCP, TLS, RTU-over-TCP, UDP, RTU, ASCII | Claims full V1.1b3 public set incl. FC43 | ✅ | ✅ | Endianness helpers, broadcast |
| **goburrow/modbus** | Go | TCP, RTU, ASCII (Linux) | 01–06, 0F, 10, 16, 17, 18 | ✅ | ❌ | Fail-fast design |

### Observations

1. **Transport coverage**: RTU and TCP are the baseline for any "complete"
   implementation. ASCII appears in C#, Java, Python, Go and the Rust `rmodbus`
   crate, but is increasingly legacy. UDP and TLS are optional extras.
2. **Function-code coverage**: Most libraries implement at least the eight common
   data-access codes (01–06, 0F, 10). Libraries that advertise "full protocol"
   also cover diagnostics (08), file records (14/15), FIFO (18), mask write (16),
   read/write multiple (17) and MEI/device identification (2B/0E).
3. **Client/server runtime**: A true "complete" stack in other ecosystems almost
   always includes a working client and often a server. This spec deliberately
   keeps the core crate transport-agnostic; a runtime can be added later as a
   separate crate or optional feature.
4. **No-std**: Only a few Rust crates (`rmodbus`, `voltage_modbus`) target
   embedded/no-std use. This is a differentiator for the Rust implementation.

### Implications for this project

This spec already matches or exceeds the function-code coverage of libmodbus,
tokio-modbus and goburrow/modbus, and matches pymodbus/adibhanna on the PDU
layer. The main gaps versus a "kitchen-sink" implementation are:

- **ASCII mode**: easy to add later as a small framer module.
- **UDP transport**: rarely required; can reuse the TCP PDU path if needed.
- **Client/server runtime**: intentionally out of scope for the core crate.
- **Simulator/REPL**: tooling, not protocol.

## Open Questions

1. Do you need an async TCP client/server runtime built on top of this library,
   or is the framing library sufficient?
2. Should the crate expose C-compatible FFI bindings, or is it Rust-only?
3. Do you want support for Modbus ASCII mode in addition to RTU binary mode?

## Assumptions I'm Making

1. "Complete Modbus" in this context means the application-layer PDU plus RTU
   and TCP framing, **not** a full networked device runtime.
2. The crate should be usable from `no_std` environments (embedded targets).
3. Serial-line-only function codes (e.g. `0x07`, `0x08`, `0x0B`, `0x0C`, `0x11`)
   are still implemented at the PDU layer even though their framing is normally
   RTU.
4. Modbus ASCII mode is out of scope unless explicitly requested.
5. User-defined and reserved function codes are out of scope.

→ Correct me now or I'll proceed with these.
