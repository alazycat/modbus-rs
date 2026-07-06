# PRD: Rust Modbus Protocol Stack

## Introduction

Build a complete, production-quality Rust implementation of the Modbus protocol
(Application Protocol V1.1b3) that can be used both in resource-constrained
embedded environments and in full-featured desktop/server applications. The stack
will expose a `no_std` PDU/framing core, optional `std`-based async
client/server runtimes, support for RTU/ASCII/TCP/UDP transports, and high-level
data-type helpers.

## Goals

- Provide a `no_std` + `alloc` compatible core for PDU encoding/decoding and
  transport framing.
- Support Modbus RTU, ASCII, TCP, and UDP transports.
- Implement all public Modbus function codes defined in V1.1b3.
- Provide async client and server runtimes for `std` environments.
- Provide register-to-data-type conversion helpers (`u16`, `u32`, `u64`, `i16`,
  `i32`, `i64`, `f32`, `f64`, strings).
- Achieve unit-test coverage for every function code and framer, plus
  integration tests against simulated devices.
- Remain dependency-light; no heavy async runtime required in the core crate.

## User Stories

### US-001: PDU core and bit-access function codes
**Description:** As a library user, I want to encode/decode Modbus requests and
responses for coils and discrete inputs so that I can read and write single-bit
data.

**Acceptance Criteria:**
- [ ] Request/response types for FC `0x01`, `0x02`, `0x05`, `0x0F` exist and
      round-trip through encode/decode.
- [ ] Coil packing follows spec (LSB first, zero-padded final byte).
- [ ] Quantity and address bounds are validated.
- [ ] Unit tests pass.

### US-002: Register access function codes
**Description:** As a library user, I want to encode/decode holding/input
register requests and responses so that I can read and write 16-bit word data.

**Acceptance Criteria:**
- [ ] Request/response types for FC `0x03`, `0x04`, `0x06`, `0x10` exist and
      round-trip.
- [ ] Big-endian register encoding per spec.
- [ ] Quantity and address bounds validated.
- [ ] Unit tests pass.

### US-003: Advanced register operations
**Description:** As a library user, I want mask-write, read/write multiple, and
FIFO queue operations so that I can perform atomic and complex register
manipulations.

**Acceptance Criteria:**
- [ ] Request/response types for FC `0x16`, `0x17`, `0x18` exist and round-trip.
- [ ] Bounds and structure validation per spec.
- [ ] Unit tests pass.

### US-004: File records and diagnostics
**Description:** As a library user, I want file record access and diagnostic
function codes so that I can support legacy and debugging workflows.

**Acceptance Criteria:**
- [ ] Request/response types for FC `0x07`, `0x08`, `0x0B`, `0x0C`, `0x11`,
      `0x14`, `0x15` exist.
- [ ] Diagnostic sub-function codes supported.
- [ ] Unit tests pass.

### US-005: Encapsulated Interface Transport (MEI)
**Description:** As a library user, I want to encode/decode FC `0x2B` MEI frames
so that I can read device identification and handle CANopen references.

**Acceptance Criteria:**
- [ ] Request/response types for FC `0x2B` with MEI types `0x0D` and `0x0E`
      exist.
- [ ] Device identification object decoding works.
- [ ] Unit tests pass.

### US-006: Exception responses
**Description:** As a library user, I want to create and parse exception
responses for all standard exception codes so that error handling is consistent.

**Acceptance Criteria:**
- [ ] `ExceptionCode` enum covers `0x01`–`0x0B`.
- [ ] Exception response encoding adds `0x80` to function code.
- [ ] Unit tests pass.

### US-007: RTU framing and CRC
**Description:** As a library user, I want to build and parse RTU frames with
CRC-16 validation so that I can communicate over serial binary Modbus.

**Acceptance Criteria:**
- [ ] RTU ADU builder/parser implemented.
- [ ] CRC-16 computation and verification correct (known vectors).
- [ ] Broadcast address 0 handled.
- [ ] Unit tests pass.

### US-008: ASCII framing and LRC
**Description:** As a library user, I want to build and parse ASCII frames with
LRC validation so that I can communicate over legacy serial ASCII Modbus.

**Acceptance Criteria:**
- [ ] ASCII ADU builder/parser implemented with colon start and CR/LF delimiter.
- [ ] LRC-8 computation and verification correct.
- [ ] Hex ASCII encoding/decoding correct.
- [ ] Unit tests pass.

### US-009: TCP and UDP MBAP framing
**Description:** As a library user, I want to build and parse MBAP headers for
TCP and UDP so that I can communicate over Ethernet.

**Acceptance Criteria:**
- [ ] MBAP header encode/decode (transaction ID, protocol ID, length, unit ID).
- [ ] Distinguish TCP stream framing from UDP datagram framing.
- [ ] Unit tests pass.

### US-010: Data-type conversion helpers
**Description:** As a library user, I want to convert between register arrays
and common data types so that I don't reimplement endianness logic.

**Acceptance Criteria:**
- [ ] Helpers for `u16`, `u32`, `u64`, `i16`, `i32`, `i64`, `f32`, `f64`, and
      strings.
- [ ] Configurable endianness and word order.
- [ ] Unit tests pass.

### US-011: Async TCP client
**Description:** As an application developer, I want an async Modbus TCP client
so that I can read and write data from a remote device.

**Acceptance Criteria:**
- [ ] Connect to TCP endpoint, send requests, receive responses.
- [ ] Transaction ID tracking per MBAP.
- [ ] Timeout and retry configuration.
- [ ] Integration test with in-process server passes.

### US-012: Async RTU/ASCII client
**Description:** As an application developer, I want an async serial client so
that I can communicate over RS-485/RS-232.

**Acceptance Criteria:**
- [ ] RTU and ASCII serial client implementations.
- [ ] Configurable baud rate, parity, stop bits (via serial abstraction).
- [ ] Integration test with loopback or mock transport passes.

### US-013: Async TCP server
**Description:** As an application developer, I want to host a Modbus TCP server
so that clients can query my application.

**Acceptance Criteria:**
- [ ] TCP server accepts multiple concurrent clients.
- [ ] User-provided data store handler trait.
- [ ] Sends correct responses and exception responses.
- [ ] Integration test passes.

### US-014: Async RTU/ASCII server
**Description:** As an application developer, I want to host a Modbus serial
server so that I can emulate a slave device.

**Acceptance Criteria:**
- [ ] RTU and ASCII serial server implementations.
- [ ] Address filtering (respond only to matching slave ID or broadcast).
- [ ] Integration test passes.

### US-015: UDP transport support
**Description:** As an application developer, I want Modbus over UDP so that I
can use connectionless Ethernet transport.

**Acceptance Criteria:**
- [ ] UDP client and server using MBAP-like header.
- [ ] Unit and integration tests pass.

### US-016: Integration test harness
**Description:** As a maintainer, I want automated integration tests so that
client/server pairs work end-to-end.

**Acceptance Criteria:**
- [ ] In-process TCP client/server integration tests.
- [ ] In-memory serial transport integration tests.
- [ ] CI runs all tests.

## Functional Requirements

- FR-1: The core crate must be `no_std` + `alloc` compatible.
- FR-2: The core crate must encode/decode all public Modbus function codes
  (`0x01`, `0x02`, `0x03`, `0x04`, `0x05`, `0x06`, `0x07`, `0x08`, `0x0B`,
  `0x0C`, `0x0F`, `0x10`, `0x11`, `0x14`, `0x15`, `0x16`, `0x17`, `0x18`,
  `0x2B`).
- FR-3: The core crate must encode/decode exception responses for codes
  `0x01`–`0x0B`.
- FR-4: The RTU framer must append and verify CRC-16.
- FR-5: The ASCII framer must use colon start, CR/LF end, and LRC-8.
- FR-6: The TCP/UDP framer must build/parse the 7-byte MBAP header.
- FR-7: The helpers crate must provide register-to-primitive conversions with
  configurable endianness.
- FR-8: The async TCP client must manage transaction IDs and timeouts.
- FR-9: The async TCP server must support multiple concurrent clients and
  user-defined data stores.
- FR-10: The async serial client/server must support RTU and ASCII modes.
- FR-11: The UDP client/server must reuse the MBAP header semantics.
- FR-12: All public APIs must be documented with rustdoc examples.

## Non-Goals

- Modbus Plus (MB+) or other proprietary transports.
- Security features such as TLS or authentication.
- Built-in simulator GUI or REPL.
- Vendor-specific or user-defined function codes.
- Automatic discovery of devices on a network.
- Persistent data storage backends (only in-memory data store examples).

## Design Considerations

- Workspace layout to separate `no_std` core from `std` runtimes.
- Client/server runtimes use `tokio` as the async runtime.
- Serial transport abstracts over `tokio-serial` or a user-provided byte stream.
- Public API prefers explicit request/response structs over a single generic
  type for clarity.

## Technical Considerations

- Core crate avoids `std` to support embedded targets.
- Client/server crates require `std` and `tokio`.
- MSRV: Rust 1.70 or later.
- Error types implement `std::error::Error` behind the `std` feature.
- Integration tests use in-memory/mock transports to avoid hardware dependencies.

## Success Metrics

- 100% of public function codes have passing round-trip unit tests.
- Client/server integration tests pass for TCP, RTU, ASCII, and UDP.
- `cargo clippy --all-targets` passes without warnings.
- Core crate builds with `--no-default-features` (no-std).
- Public API fully documented.

## Open Questions

- Should the client/server runtime be a separate crate or an optional feature of
  a single crate?
- Should we provide synchronous client wrappers in addition to async?
- Should we include a minimal command-line tool for testing?
