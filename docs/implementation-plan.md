# Implementation Plan: Close gaps with modbus-go (detailed)

## Overview

This plan turns the functional gaps identified in [`docs/comparison-with-modbus-go.md`](comparison-with-modbus-go.md) into small, implementable tasks. Each task is sized for a single focused session, touches at most a handful of files, and ends with passing tests.

## Architecture decisions

- **Vertical slices.** Each transport is implemented as a full slice (transport + ADU adapter + client connect helpers + server acceptor + tests) so every merged task leaves a working feature.
- **Reuse existing seams.** New transports implement the existing `Transport` / `AsyncTransport` traits; new clients reuse `ClientCore` / `AsyncClientCore` via `AduAdapter`.
- **Optional dependencies behind feature flags.** TLS, sync serial, configuration, and metrics are all optional features so the existing `no_std` + `alloc` path stays intact.
- **Retry/reconnect as an adapter wrapper.** Implement reconnect and retry as generic `RetryAdapter` / `AsyncRetryAdapter` so it works across all transports without per-transport duplication.
- **Typed client helpers delegate to `helpers`.** The client gains convenience methods like `read_holding_registers_u32`, but the conversion logic stays in `src/helpers.rs`.

---

## Phase 1 — Transport parity

### Task 1.1: Add `tls` feature and async TLS dependency

**Description:** Add the `tls` feature to `Cargo.toml` and pull in `tokio-rustls` (or `tokio-native-tls`) as an optional dependency.

**Acceptance criteria:**
- [ ] `tls` feature exists and enables `tcp` + `async`.
- [ ] `cargo check --features tls` succeeds with no new warnings.
- [ ] Dependency is optional and does not appear in default builds.

**Verification:**
- [ ] `cargo check --features tls` passes.
- [ ] `cargo tree --no-default-features --features tls` shows only expected deps.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 1.2: Implement `AsyncTlsTransport`

**Description:** Create `src/tls_transport.rs` and implement `AsyncTransport` for `AsyncTlsTransport<T>` where `T: AsyncRead + AsyncWrite + Unpin`.

**Acceptance criteria:**
- [ ] `AsyncTlsTransport` struct wraps a TLS stream and exposes `into_inner`, `stream`, `stream_mut`.
- [ ] `AsyncTransport::send` writes the full frame.
- [ ] `AsyncTransport::recv` reads a complete frame using the existing TCP/MBAP framing or raw frame semantics as appropriate.

**Verification:**
- [ ] `cargo check --features async,tcp,tls` succeeds.

**Dependencies:** Task 1.1

**Files likely touched:**
- `src/tls_transport.rs` (new)
- `src/lib.rs`
- `src/transport.rs` (if trait needs adjustment)

**Estimated scope:** S

---

### Task 1.3: Add async TLS client connect helper

**Description:** Add `AsyncTcpClient::connect_tls(addr, config, tls_config)` that opens a TCP connection, performs the TLS handshake, and wraps the stream in `AsyncTlsTransport`.

**Acceptance criteria:**
- [ ] Public method exists and returns `Result<AsyncTcpClient, ClientError>`.
- [ ] Uses the existing `AsyncTcpClient` / `AduAdapter` path.
- [ ] Compiles under `async,tcp,tls`.

**Verification:**
- [ ] `cargo check --features async,tcp,tls` succeeds.

**Dependencies:** Task 1.2

**Files likely touched:**
- `src/tcp_client.rs`

**Estimated scope:** S

---

### Task 1.4: Unit-test async TLS transport framing

**Description:** Write a test that sends and receives a Modbus TCP ADU through an in-memory TLS pair (e.g., `tokio::io::DuplexStream` wrapped with rustls test configs).

**Acceptance criteria:**
- [ ] Test lives in `src/tls_transport.rs` or `tests/tls_transport.rs`.
- [ ] Test verifies at least one request/response round-trip.

**Verification:**
- [ ] `cargo test --features async,tcp,tls tls` passes.

**Dependencies:** Task 1.3

**Files likely touched:**
- `src/tls_transport.rs` or `tests/tls_transport.rs`

**Estimated scope:** S

---

### Task 1.5: Add sync TLS dependency wiring

**Description:** Add the sync TLS crate (`rustls` or `native-tls`) as an optional dependency under the same `tls` feature, gated for `sync` builds.

**Acceptance criteria:**
- [ ] `cargo check --features sync,tcp,tls` succeeds.
- [ ] Dependency is optional.

**Verification:**
- [ ] `cargo check --features sync,tcp,tls` passes.

**Dependencies:** Task 1.1

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 1.6: Implement `TlsTransport` for sync

**Description:** Implement `Transport` for a synchronous TLS stream wrapper in `src/tls_transport.rs`.

**Acceptance criteria:**
- [ ] `TlsTransport<T>` struct exists with `Read + Write` delegation.
- [ ] `Transport::send` / `recv` implemented.

**Verification:**
- [ ] `cargo check --features sync,tcp,tls` succeeds.

**Dependencies:** Task 1.5

**Files likely touched:**
- `src/tls_transport.rs`

**Estimated scope:** S

---

### Task 1.7: Add sync TLS client connect helper

**Description:** Add `TcpClient::connect_tls(addr, config, tls_config)` for synchronous TLS connections.

**Acceptance criteria:**
- [ ] Public method exists and returns `Result<TcpClient, ClientError>`.
- [ ] Compiles under `sync,tcp,tls`.

**Verification:**
- [ ] `cargo check --features sync,tcp,tls` succeeds.

**Dependencies:** Task 1.6

**Files likely touched:**
- `src/tcp_client.rs`

**Estimated scope:** S

---

### Task 1.8: Unit-test sync TLS transport framing

**Description:** Write a test that sends and receives a Modbus TCP ADU through an in-memory or mocked sync TLS stream.

**Acceptance criteria:**
- [ ] Test verifies at least one request/response round-trip.

**Verification:**
- [ ] `cargo test --features sync,tcp,tls tls` passes.

**Dependencies:** Task 1.7

**Files likely touched:**
- `src/tls_transport.rs` or `tests/tls_transport.rs`

**Estimated scope:** S

---

### Task 1.9: Add async TLS server acceptor

**Description:** Extend `src/tcp_server.rs` to accept TLS connections and serve them with the existing async server.

**Acceptance criteria:**
- [ ] A new constructor/method exists for starting a TLS listener.
- [ ] The TLS stream is wrapped in `AsyncTlsTransport` and dispatched.

**Verification:**
- [ ] `cargo check --features async,tcp,tls` succeeds.

**Dependencies:** Task 1.3

**Files likely touched:**
- `src/tcp_server.rs`

**Estimated scope:** S

---

### Task 1.10: Add sync TLS server acceptor

**Description:** Extend `src/tcp_server.rs` to accept TLS connections for the sync server path.

**Acceptance criteria:**
- [ ] A new constructor/method exists for synchronous TLS accept.
- [ ] The TLS stream is wrapped in `TlsTransport` and dispatched.

**Verification:**
- [ ] `cargo check --features sync,tcp,tls` succeeds.

**Dependencies:** Task 1.7, Task 1.9

**Files likely touched:**
- `src/tcp_server.rs`

**Estimated scope:** S

---

### Task 1.11: TLS integration test fixtures

**Description:** Create self-signed certificate/key PEM fixtures for integration tests under `tests/fixtures/`.

**Acceptance criteria:**
- [ ] `tests/fixtures/tls_cert.pem` and `tests/fixtures/tls_key.pem` committed.
- [ ] Certificates are generated with a script or documented command so they can be regenerated.

**Verification:**
- [ ] Fixtures load successfully in a small sanity test.

**Dependencies:** None

**Files likely touched:**
- `tests/fixtures/tls_cert.pem`
- `tests/fixtures/tls_key.pem`
- `tests/fixtures/README.md` or comment

**Estimated scope:** XS

---

### Task 1.12: TLS end-to-end integration tests

**Description:** Add `tests/tls_integration.rs` that runs a full Modbus request/response cycle over TLS in both sync and async modes.

**Acceptance criteria:**
- [ ] Sync TLS integration test passes.
- [ ] Async TLS integration test passes.
- [ ] Tests use the fixtures from Task 1.11.

**Verification:**
- [ ] `cargo test --features sync,tcp,tls --test tls_integration` passes.
- [ ] `cargo test --features async,tcp,tls --test tls_integration` passes.

**Dependencies:** Task 1.4, Task 1.8, Task 1.9, Task 1.10, Task 1.11

**Files likely touched:**
- `tests/tls_integration.rs` (new)

**Estimated scope:** M

---

### Task 2.1: Add sync RTU-over-TCP client connect helper

**Description:** Add `TcpClient::connect_rtu_over_tcp(addr, config)` that opens a TCP stream and wraps it in `RtuTransport<TcpStream>`.

**Acceptance criteria:**
- [ ] Method exists under `sync,rtu,tcp` features.
- [ ] Returns a `Client` backed by the RTU-over-TCP adapter.

**Verification:**
- [ ] `cargo check --features sync,rtu,tcp` succeeds.

**Dependencies:** None

**Files likely touched:**
- `src/tcp_client.rs`

**Estimated scope:** XS

---

### Task 2.2: Add async RTU-over-TCP client connect helper

**Description:** Add `AsyncTcpClient::connect_rtu_over_tcp(addr, config)` that opens a TCP stream and wraps it in `RtuTransport<tokio::net::TcpStream>`.

**Acceptance criteria:**
- [ ] Method exists under `async,rtu,tcp` features.
- [ ] Returns an `AsyncClient` backed by the RTU-over-TCP adapter.

**Verification:**
- [ ] `cargo check --features async,rtu,tcp` succeeds.

**Dependencies:** None

**Files likely touched:**
- `src/tcp_client.rs`

**Estimated scope:** XS

---

### Task 2.3: RTU-over-TCP client framing tests

**Description:** Write tests that verify RTU ADUs (including CRC) are sent and received correctly over an in-memory TCP stream.

**Acceptance criteria:**
- [ ] Sync test passes.
- [ ] Async test passes.
- [ ] Test confirms CRC is present in the sent frame.

**Verification:**
- [ ] `cargo test --features sync,rtu,tcp rtu_over_tcp_client` passes.
- [ ] `cargo test --features async,rtu,tcp rtu_over_tcp_client` passes.

**Dependencies:** Task 2.1, Task 2.2

**Files likely touched:**
- `tests/rtu_over_tcp_client.rs` (new)

**Estimated scope:** S

---

### Task 2.4: Implement sync RTU-over-TCP server

**Description:** Create `src/rtu_over_tcp_server.rs` with a synchronous server that accepts TCP connections and dispatches RTU-framed PDUs to a `DataStore`.

**Acceptance criteria:**
- [ ] `RtuOverTcpServer` struct exists.
- [ ] `serve_one` / `serve_forever` style API consistent with sync `RtuServer`.
- [ ] Exported under `sync,rtu,tcp` features.

**Verification:**
- [ ] `cargo check --features sync,rtu,tcp` succeeds.

**Dependencies:** Task 2.1

**Files likely touched:**
- `src/rtu_over_tcp_server.rs` (new)
- `src/lib.rs`

**Estimated scope:** S

---

### Task 2.5: Implement async RTU-over-TCP server

**Description:** Create the async counterpart in `src/rtu_over_tcp_server.rs` that accepts TCP connections and dispatches RTU-framed PDUs.

**Acceptance criteria:**
- [ ] `AsyncRtuOverTcpServer` struct exists.
- [ ] `serve` / `serve_one` async API.
- [ ] Exported under `async,rtu,tcp` features.

**Verification:**
- [ ] `cargo check --features async,rtu,tcp` succeeds.

**Dependencies:** Task 2.2

**Files likely touched:**
- `src/rtu_over_tcp_server.rs`
- `src/lib.rs`

**Estimated scope:** S

---

### Task 2.6: RTU-over-TCP server integration tests

**Description:** Add `tests/rtu_over_tcp_server.rs` verifying read/write cycles against sync and async RTU-over-TCP servers.

**Acceptance criteria:**
- [ ] Sync server test passes.
- [ ] Async server test passes.

**Verification:**
- [ ] `cargo test --features sync,rtu,tcp --test rtu_over_tcp_server` passes.
- [ ] `cargo test --features async,rtu,tcp --test rtu_over_tcp_server` passes.

**Dependencies:** Task 2.4, Task 2.5

**Files likely touched:**
- `tests/rtu_over_tcp_server.rs` (new)

**Estimated scope:** S

---

### Task 3.1: Add `sync-serial` feature and `serialport` dependency

**Description:** Add an optional `sync-serial` feature and the `serialport` crate dependency. Ensure it composes with `rtu` and `ascii`.

**Acceptance criteria:**
- [ ] `sync-serial` feature exists.
- [ ] `cargo check --features sync,rtu,sync-serial` succeeds.
- [ ] `cargo check --features sync,ascii,sync-serial` succeeds.

**Verification:**
- [ ] `cargo tree --no-default-features --features sync,rtu,sync-serial` shows `serialport`.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 3.2: Implement `SerialTransport`

**Description:** Create `src/serial_transport.rs` with a `SerialTransport` wrapper around `serialport::SerialPort` implementing `Transport`.

**Acceptance criteria:**
- [ ] `SerialTransport` struct exists.
- [ ] Implements `Transport::send` / `recv` by reading/writing the serial port.
- [ ] Configurable read timeout.

**Verification:**
- [ ] `cargo check --features sync,rtu,sync-serial` succeeds.

**Dependencies:** Task 3.1

**Files likely touched:**
- `src/serial_transport.rs` (new)
- `src/lib.rs`

**Estimated scope:** S

---

### Task 3.3: Wire sync serial RTU client/server path

**Description:** Ensure `RtuTransport<SerialTransport>` can be used with the existing sync client and server constructors.

**Acceptance criteria:**
- [ ] A public helper opens a serial port and returns an RTU client.
- [ ] A public helper opens a serial port and returns an RTU server.
- [ ] Docs/examples mention the path.

**Verification:**
- [ ] `cargo check --features sync,rtu,sync-serial` succeeds.

**Dependencies:** Task 3.2

**Files likely touched:**
- `src/serial_transport.rs`
- `src/rtu_server.rs` or `src/client/sync.rs` (if convenience helper is added)

**Estimated scope:** S

---

### Task 3.4: Wire sync serial ASCII client/server path

**Description:** Ensure `AsciiTransport<SerialTransport>` works with the existing sync ASCII client and server.

**Acceptance criteria:**
- [ ] A public helper opens a serial port and returns an ASCII client.
- [ ] A public helper opens a serial port and returns an ASCII server.

**Verification:**
- [ ] `cargo check --features sync,ascii,sync-serial` succeeds.

**Dependencies:** Task 3.2

**Files likely touched:**
- `src/serial_transport.rs`
- `src/ascii_client.rs` / `src/ascii_server.rs` (if convenience helper is added)

**Estimated scope:** S

---

### Task 3.5: Sync serial mocked tests

**Description:** Write tests using a fake `Read + Write` implementation (or `serialport` mock if available) to verify RTU/ASCII framing over the serial transport path without real hardware.

**Acceptance criteria:**
- [ ] RTU framing test passes.
- [ ] ASCII framing test passes.

**Verification:**
- [ ] `cargo test --features sync,rtu,sync-serial serial` passes.
- [ ] `cargo test --features sync,ascii,sync-serial serial` passes.

**Dependencies:** Task 3.3, Task 3.4

**Files likely touched:**
- `src/serial_transport.rs`
- `tests/serial_sync.rs` (new)

**Estimated scope:** S

---

### Checkpoint: Transport parity

- [ ] Tasks 1.1–1.12, 2.1–2.6, 3.1–3.5 merged.
- [ ] `cargo test --all-features` passes.
- [ ] README updated with new transports and feature flags.
- [ ] Review with human before starting Phase 2.

---

## Phase 2 — Client ergonomics

### Task 4.1: Design typed helper signatures

**Description:** Decide method names, parameter order, and where `Endian`/`WordOrder` live (method args vs. `ClientConfig`). Document the decision.

**Acceptance criteria:**
- [ ] API sketch written in task comments or a short ADR.
- [ ] Consensus on naming (e.g., `read_holding_registers_u32`).

**Verification:**
- [ ] Human review of the API sketch.

**Dependencies:** None

**Files likely touched:**
- `docs/adr/typed-client-helpers.md` (new) or task comments

**Estimated scope:** XS

---

### Task 4.2: Add endian/word-order config to `ClientConfig`

**Description:** Extend `ClientConfig` with optional default `Endian` and `WordOrder` so typed helpers can use sensible defaults.

**Acceptance criteria:**
- [ ] `ClientConfig` gains `endian` and `word_order` fields.
- [ ] Defaults are big-endian / most-significant-first (Modbus convention).
- [ ] Existing constructors still compile.

**Verification:**
- [ ] `cargo check --features sync,helpers` succeeds.
- [ ] `cargo check --features async,helpers` succeeds.

**Dependencies:** Task 4.1

**Files likely touched:**
- `src/client/mod.rs`

**Estimated scope:** XS

---

### Task 4.3: Implement sync typed read helpers

**Description:** Add read methods to `ClientCore` for `u16`, `u32`, `u64`, `i16`, `i32`, `i64`, `f32`, `f64`, and `String`.

**Acceptance criteria:**
- [ ] Methods exist for holding registers and input registers.
- [ ] Methods accept explicit `Endian`/`WordOrder` or fall back to config.
- [ ] Delegate to `src/helpers.rs`.

**Verification:**
- [ ] `cargo check --features sync,helpers` succeeds.

**Dependencies:** Task 4.2

**Files likely touched:**
- `src/client/sync.rs`

**Estimated scope:** S

---

### Task 4.4: Implement async typed read helpers

**Description:** Mirror Task 4.3 on `AsyncClientCore`.

**Acceptance criteria:**
- [ ] All async typed read methods exist.
- [ ] Methods delegate to `src/helpers.rs`.

**Verification:**
- [ ] `cargo check --features async,helpers` succeeds.

**Dependencies:** Task 4.3

**Files likely touched:**
- `src/client/async.rs`

**Estimated scope:** S

---

### Task 4.5: Typed read helper unit tests

**Description:** Test all typed read helpers with big/little endian and both word orders.

**Acceptance criteria:**
- [ ] Sync tests pass.
- [ ] Async tests pass.
- [ ] Coverage for `u32`, `f32`, and `String` at minimum.

**Verification:**
- [ ] `cargo test --features sync,helpers typed_read` passes.
- [ ] `cargo test --features async,helpers typed_read` passes.

**Dependencies:** Task 4.3, Task 4.4

**Files likely touched:**
- `src/client/sync.rs`
- `src/client/async.rs`

**Estimated scope:** S

---

### Task 4.6: Implement sync typed write helpers

**Description:** Add write methods to `ClientCore` for scalar types and strings, encoding into registers before calling the existing function code.

**Acceptance criteria:**
- [ ] Methods exist: `write_multiple_registers_u32`, `_f32`, `_string`, etc.
- [ ] Methods accept explicit `Endian`/`WordOrder` or fall back to config.

**Verification:**
- [ ] `cargo check --features sync,helpers` succeeds.

**Dependencies:** Task 4.3

**Files likely touched:**
- `src/client/sync.rs`

**Estimated scope:** S

---

### Task 4.7: Implement async typed write helpers

**Description:** Mirror Task 4.6 on `AsyncClientCore`.

**Acceptance criteria:**
- [ ] All async typed write methods exist.

**Verification:**
- [ ] `cargo check --features async,helpers` succeeds.

**Dependencies:** Task 4.4

**Files likely touched:**
- `src/client/async.rs`

**Estimated scope:** S

---

### Task 4.8: Typed write helper unit tests

**Description:** Test round-trip encode/decode for typed write helpers through a mock adapter.

**Acceptance criteria:**
- [ ] Sync tests pass.
- [ ] Async tests pass.

**Verification:**
- [ ] `cargo test --features sync,helpers typed_write` passes.
- [ ] `cargo test --features async,helpers typed_write` passes.

**Dependencies:** Task 4.6, Task 4.7

**Files likely touched:**
- `src/client/sync.rs`
- `src/client/async.rs`

**Estimated scope:** S

---

### Task 5.1: Design `RetryPolicy` config

**Description:** Define the retry policy struct (max retries, initial backoff, max backoff, retryable predicate) and decide how it attaches to `ClientConfig`.

**Acceptance criteria:**
- [ ] `RetryPolicy` struct designed and reviewed.
- [ ] Default policy documented.

**Verification:**
- [ ] Human review of the design.

**Dependencies:** None

**Files likely touched:**
- `docs/adr/retry-policy.md` (new) or task comments

**Estimated scope:** XS

---

### Task 5.2: Implement `RetryPolicy` and config wiring

**Description:** Add `RetryPolicy` to `src/client/mod.rs` and expose it through `ClientConfig`.

**Acceptance criteria:**
- [ ] `RetryPolicy` struct exists with sensible defaults.
- [ ] `ClientConfig` optionally holds a `RetryPolicy`.

**Verification:**
- [ ] `cargo check --features sync` succeeds.
- [ ] `cargo check --features async` succeeds.

**Dependencies:** Task 5.1

**Files likely touched:**
- `src/client/mod.rs`

**Estimated scope:** XS

---

### Task 5.3: Implement sync `RetryAdapter`

**Description:** Create `src/client/retry_adapter.rs` with `RetryAdapter<A: AduAdapter>` that wraps an adapter, reconnects via a factory closure on transient errors, and retries with backoff.

**Acceptance criteria:**
- [ ] `RetryAdapter` implements `AduAdapter`.
- [ ] Reconnects on `TransportError::Disconnected` and configurable `Io` errors.
- [ ] Respects `RetryPolicy` limits.

**Verification:**
- [ ] `cargo check --features sync,rtu` succeeds.

**Dependencies:** Task 5.2

**Files likely touched:**
- `src/client/retry_adapter.rs` (new)
- `src/client/mod.rs`

**Estimated scope:** M

---

### Task 5.4: Implement async `AsyncRetryAdapter`

**Description:** Mirror Task 5.3 for async adapters.

**Acceptance criteria:**
- [ ] `AsyncRetryAdapter` implements `AsyncAduAdapter`.
- [ ] Backoff and reconnect logic are async-aware.

**Verification:**
- [ ] `cargo check --features async,rtu` succeeds.

**Dependencies:** Task 5.3

**Files likely touched:**
- `src/client/retry_adapter.rs`

**Estimated scope:** M

---

### Task 5.5: Integrate retry into client constructors

**Description:** Add client constructors that accept a `RetryPolicy` and wrap the adapter automatically.

**Acceptance criteria:**
- [ ] Sync and async clients can be constructed with retry enabled.
- [ ] Existing constructors remain unchanged (retry is opt-in).

**Verification:**
- [ ] `cargo check --features sync,rtu` succeeds.
- [ ] `cargo check --features async,rtu` succeeds.

**Dependencies:** Task 5.3, Task 5.4

**Files likely touched:**
- `src/client/sync.rs`
- `src/client/async.rs`
- `src/tcp_client.rs`, `src/udp_client.rs`, etc.

**Estimated scope:** S

---

### Task 5.6: Retry adapter unit tests

**Description:** Write tests simulating disconnects and transient failures to verify retry behavior.

**Acceptance criteria:**
- [ ] Test verifies successful transparent retry after disconnect.
- [ ] Test verifies policy limits are respected.
- [ ] Test verifies non-retryable errors propagate immediately.

**Verification:**
- [ ] `cargo test --features sync,rtu retry` passes.
- [ ] `cargo test --features async,rtu retry` passes.

**Dependencies:** Task 5.5

**Files likely touched:**
- `src/client/retry_adapter.rs`
- `tests/retry_adapter.rs` (new)

**Estimated scope:** M

---

### Task 6.1: Add broadcast detection to sync RTU adapter

**Description:** Update sync `RtuAduAdapter` to detect `unit_id == 0`, send the request, and return immediately without waiting for a response.

**Acceptance criteria:**
- [ ] `unit_id == 0` skips `recv`.
- [ ] Returns `Ok(Vec::new())` or equivalent empty PDU.

**Verification:**
- [ ] `cargo test --features sync,rtu broadcast` passes.

**Dependencies:** None

**Files likely touched:**
- `src/client/rtu_adapter.rs`

**Estimated scope:** XS

---

### Task 6.2: Add broadcast detection to async RTU adapter

**Description:** Mirror Task 6.1 for the async RTU adapter.

**Acceptance criteria:**
- [ ] `unit_id == 0` skips async `recv`.

**Verification:**
- [ ] `cargo test --features async,rtu broadcast` passes.

**Dependencies:** Task 6.1

**Files likely touched:**
- `src/client/rtu_adapter.rs`

**Estimated scope:** XS

---

### Task 6.3: Add broadcast detection to ASCII adapters

**Description:** Update sync and async ASCII adapters to support `unit_id == 0` broadcast.

**Acceptance criteria:**
- [ ] Sync ASCII broadcast works.
- [ ] Async ASCII broadcast works.

**Verification:**
- [ ] `cargo test --features sync,ascii broadcast` passes.
- [ ] `cargo test --features async,ascii broadcast` passes.

**Dependencies:** Task 6.1

**Files likely touched:**
- `src/ascii_client.rs`

**Estimated scope:** XS

---

### Task 6.4: Broadcast integration tests

**Description:** Add tests verifying broadcast write requests do not expect responses across RTU and ASCII.

**Acceptance criteria:**
- [ ] RTU sync/async broadcast tests pass.
- [ ] ASCII sync/async broadcast tests pass.

**Verification:**
- [ ] `cargo test --features sync,rtu,ascii broadcast` passes.

**Dependencies:** Task 6.2, Task 6.3

**Files likely touched:**
- `tests/broadcast.rs` (new)

**Estimated scope:** S

---

### Checkpoint: Client ergonomics

- [ ] Tasks 4.1–4.8, 5.1–5.6, 6.1–6.4 merged.
- [ ] `cargo test --all-features` passes.
- [ ] README updated with typed helper and retry examples.
- [ ] Review with human before starting Phase 3.

---

## Phase 3 — Server and runtime

### Task 7.1: Refactor async server dispatch for reuse

**Description:** Extract the RTU stream dispatch loop from `src/server/async.rs` into a reusable function so `AsyncRtuServer` can use it.

**Acceptance criteria:**
- [ ] Shared dispatch function exists.
- [ ] Existing async RTU server behavior unchanged.

**Verification:**
- [ ] `cargo test --features async,rtu` passes.

**Dependencies:** None

**Files likely touched:**
- `src/server/async.rs`

**Estimated scope:** S

---

### Task 7.2: Implement `AsyncRtuServer` stream server

**Description:** Create `src/async_rtu_server.rs` with an async RTU server that accepts any `AsyncRead + AsyncWrite` stream.

**Acceptance criteria:**
- [ ] `AsyncRtuServer` struct exists.
- [ ] `serve` / `serve_one` API.
- [ ] Exported under `async,rtu` features.

**Verification:**
- [ ] `cargo check --features async,rtu` succeeds.

**Dependencies:** Task 7.1

**Files likely touched:**
- `src/async_rtu_server.rs` (new)
- `src/lib.rs`

**Estimated scope:** S

---

### Task 7.3: Add tokio-serial integration to async RTU server

**Description:** Add a convenience constructor that opens a `tokio-serial` port and serves RTU frames.

**Acceptance criteria:**
- [ ] Constructor exists under `async,rtu,serial` features.
- [ ] Compiles with `tokio-serial`.

**Verification:**
- [ ] `cargo check --features async,rtu,serial` succeeds.

**Dependencies:** Task 7.2

**Files likely touched:**
- `src/async_rtu_server.rs`

**Estimated scope:** XS

---

### Task 7.4: Async RTU server tests

**Description:** Test the async RTU server using `tokio::io::DuplexStream` and (optionally) `tokio-serial` loopback.

**Acceptance criteria:**
- [ ] Stream-based test passes.
- [ ] Serial tests marked `#[ignore]` if hardware is required.

**Verification:**
- [ ] `cargo test --features async,rtu async_rtu_server` passes.
- [ ] `cargo test --features async,rtu,serial async_rtu_server` passes (non-ignored).

**Dependencies:** Task 7.2, Task 7.3

**Files likely touched:**
- `tests/async_rtu_server.rs` (new)

**Estimated scope:** S

---

### Task 8.1: Implement `FileStore`

**Description:** Create `examples/file_store.rs` (or `src/server/file_store.rs` behind a feature) with a JSON-backed `DataStore`.

**Acceptance criteria:**
- [ ] Implements `DataStore`.
- [ ] Persists on write, reloads on creation.
- [ ] Uses a simple JSON schema.

**Verification:**
- [ ] `cargo check --example file_store --features sync` succeeds.

**Dependencies:** None

**Files likely touched:**
- `examples/file_store.rs` (new)
- `Cargo.toml` (example)

**Estimated scope:** S

---

### Task 8.2: FileStore persistence test

**Description:** Add a test that writes through `FileStore`, drops it, recreates it, and reads back the values.

**Acceptance criteria:**
- [ ] Test passes.

**Verification:**
- [ ] `cargo test --example file_store` passes.

**Dependencies:** Task 8.1

**Files likely touched:**
- `examples/file_store.rs`

**Estimated scope:** XS

---

### Task 9.1: Design `RequestHook` trait

**Description:** Define the hook interface (before request, after response, can mutate/reject) without changing `DataStore`.

**Acceptance criteria:**
- [ ] Trait designed and reviewed.
- [ ] Decision documented (e.g., `docs/adr/request-hooks.md`).

**Verification:**
- [ ] Human review.

**Dependencies:** None

**Files likely touched:**
- `docs/adr/request-hooks.md` (new)

**Estimated scope:** XS

---

### Task 9.2: Implement `RequestHook` trait

**Description:** Create `src/server/hook.rs` with the `RequestHook` trait and a no-op default.

**Acceptance criteria:**
- [ ] Trait exists.
- [ ] No-op hook implements it.

**Verification:**
- [ ] `cargo check --features sync` succeeds.

**Dependencies:** Task 9.1

**Files likely touched:**
- `src/server/hook.rs` (new)
- `src/server/mod.rs`

**Estimated scope:** XS

---

### Task 9.3: Integrate hooks into sync server

**Description:** Update `src/server/sync.rs` to accept an optional hook and invoke it around dispatch.

**Acceptance criteria:**
- [ ] `Server::dispatch` accepts a hook.
- [ ] Hook is called before request and after response.

**Verification:**
- [ ] `cargo test --features sync` passes.

**Dependencies:** Task 9.2

**Files likely touched:**
- `src/server/sync.rs`

**Estimated scope:** S

---

### Task 9.4: Integrate hooks into async server

**Description:** Mirror Task 9.3 for `src/server/async.rs`.

**Acceptance criteria:**
- [ ] `AsyncServer` dispatch uses the hook.

**Verification:**
- [ ] `cargo test --features async` passes.

**Dependencies:** Task 9.3

**Files likely touched:**
- `src/server/async.rs`

**Estimated scope:** S

---

### Task 9.5: Add logging hook example

**Description:** Provide a simple `LoggingHook` example that prints/traces each request/response.

**Acceptance criteria:**
- [ ] Example compiles.
- [ ] Demonstrates hook usage.

**Verification:**
- [ ] `cargo build --example logging_hook --features sync` succeeds.

**Dependencies:** Task 9.4

**Files likely touched:**
- `examples/logging_hook.rs` (new)

**Estimated scope:** XS

---

### Task 9.6: Hook unit tests

**Description:** Test that hooks can inspect, mutate, and reject requests.

**Acceptance criteria:**
- [ ] Sync test passes.
- [ ] Async test passes.

**Verification:**
- [ ] `cargo test --features sync hooks` passes.
- [ ] `cargo test --features async hooks` passes.

**Dependencies:** Task 9.4, Task 9.5

**Files likely touched:**
- `src/server/hook.rs`
- `tests/server_hooks.rs` (new)

**Estimated scope:** S

---

### Checkpoint: Server and runtime

- [ ] Tasks 7.1–7.4, 8.1–8.2, 9.1–9.6 merged.
- [ ] `cargo test --all-features` passes.
- [ ] New server examples documented.
- [ ] Review with human before starting Phase 4.

---

## Phase 4 — Configuration and observability

### Task 10.1: Add `config` feature and serde dependencies

**Description:** Add a `config` feature and optional `serde` + format crates (`serde_json`, `toml`, `serde_yml`).

**Acceptance criteria:**
- [ ] `config` feature exists.
- [ ] `cargo check --features config` succeeds.

**Verification:**
- [ ] `cargo check --features config` passes.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 10.2: Define client config structs

**Description:** Create serde-deserializable structs for client configuration (transport, timeout, retry policy).

**Acceptance criteria:**
- [ ] `ClientConfigFile` struct exists.
- [ ] Fields map to `ClientConfig` and `RetryPolicy`.

**Verification:**
- [ ] `cargo check --features config` succeeds.

**Dependencies:** Task 10.1

**Files likely touched:**
- `src/config.rs` (new)
- `src/lib.rs`

**Estimated scope:** S

---

### Task 10.3: Define server config structs

**Description:** Create serde-deserializable structs for server configuration (bind address, transport, store path, hooks).

**Acceptance criteria:**
- [ ] `ServerConfigFile` struct exists.

**Verification:**
- [ ] `cargo check --features config` succeeds.

**Dependencies:** Task 10.2

**Files likely touched:**
- `src/config.rs`

**Estimated scope:** S

---

### Task 10.4: Add config loaders for JSON/TOML/YAML

**Description:** Implement `from_json`, `from_toml`, `from_yaml` functions (or a unified `load` function) in `src/config.rs`.

**Acceptance criteria:**
- [ ] Each format deserializes successfully.
- [ ] Errors are mapped to a clear error type.

**Verification:**
- [ ] `cargo check --features config` succeeds.

**Dependencies:** Task 10.3

**Files likely touched:**
- `src/config.rs`

**Estimated scope:** S

---

### Task 10.5: Config round-trip tests

**Description:** Write tests parsing sample config files in all three formats.

**Acceptance criteria:**
- [ ] JSON test passes.
- [ ] TOML test passes.
- [ ] YAML test passes.

**Verification:**
- [ ] `cargo test --features config` passes.

**Dependencies:** Task 10.4

**Files likely touched:**
- `tests/config.rs` (new)
- `tests/fixtures/client.json`, `client.toml`, `client.yaml`

**Estimated scope:** S

---

### Task 11.1: Add `tracing` feature flag

**Description:** Make `tracing` available as a standalone feature (not only via `cli`) so users can enable instrumentation without pulling in `clap`.

**Acceptance criteria:**
- [ ] `tracing` feature exists and enables `dep:tracing`.
- [ ] `tracing-subscriber` kept under `cli` only.

**Verification:**
- [ ] `cargo check --features tracing` succeeds.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 11.2: Instrument transport send/receive

**Description:** Add `tracing` events to `Transport::send` / `recv` and `AsyncTransport` equivalents.

**Acceptance criteria:**
- [ ] Events emitted at debug level.
- [ ] No events compiled in when `tracing` feature is disabled.

**Verification:**
- [ ] `cargo test --features sync,tcp,tracing` passes.

**Dependencies:** Task 11.1

**Files likely touched:**
- `src/tcp_transport.rs`
- `src/rtu_transport.rs`
- `src/ascii_transport.rs`
- `src/udp_transport.rs`

**Estimated scope:** S

---

### Task 11.3: Instrument client dispatch

**Description:** Add `tracing` spans/events around client request dispatch, timeouts, and exceptions.

**Acceptance criteria:**
- [ ] Client dispatch emits a span with function code and unit ID.
- [ ] Timeout and exception events emitted.

**Verification:**
- [ ] `cargo test --features sync,rtu,tracing` passes.
- [ ] `cargo test --features async,rtu,tracing` passes.

**Dependencies:** Task 11.2

**Files likely touched:**
- `src/client/sync.rs`
- `src/client/async.rs`

**Estimated scope:** S

---

### Task 11.4: Instrument server dispatch

**Description:** Add `tracing` spans/events around server request handling and exceptions.

**Acceptance criteria:**
- [ ] Server dispatch emits a span with function code and unit ID.
- [ ] Exception responses are logged.

**Verification:**
- [ ] `cargo test --features sync,tracing` passes.
- [ ] `cargo test --features async,tracing` passes.

**Dependencies:** Task 11.3

**Files likely touched:**
- `src/server/sync.rs`
- `src/server/async.rs`

**Estimated scope:** S

---

### Task 12.1: Add `metrics` feature and `Metrics` struct

**Description:** Add a `metrics` feature and define an atomic-counter-based `Metrics` struct.

**Acceptance criteria:**
- [ ] `metrics` feature exists.
- [ ] `Metrics` has counters for requests, exceptions, timeouts, retries.

**Verification:**
- [ ] `cargo check --features metrics` succeeds.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`
- `src/metrics.rs` (new)
- `src/lib.rs`

**Estimated scope:** XS

---

### Task 12.2: Integrate metrics into client

**Description:** Add optional `Metrics` reference to client cores and increment counters on requests, exceptions, timeouts, and retries.

**Acceptance criteria:**
- [ ] Sync client increments metrics.
- [ ] Async client increments metrics.
- [ ] No metrics code compiled without the feature.

**Verification:**
- [ ] `cargo check --features sync,rtu,metrics` succeeds.

**Dependencies:** Task 12.1, Task 5.4

**Files likely touched:**
- `src/client/sync.rs`
- `src/client/async.rs`
- `src/client/mod.rs`

**Estimated scope:** S

---

### Task 12.3: Integrate metrics into server

**Description:** Add optional `Metrics` reference to server dispatch and increment counters on requests and exceptions.

**Acceptance criteria:**
- [ ] Sync server increments metrics.
- [ ] Async server increments metrics.

**Verification:**
- [ ] `cargo check --features sync,metrics` succeeds.

**Dependencies:** Task 12.1

**Files likely touched:**
- `src/server/sync.rs`
- `src/server/async.rs`

**Estimated scope:** S

---

### Task 12.4: Metrics unit tests

**Description:** Verify counters increment correctly through synthetic client/server interactions.

**Acceptance criteria:**
- [ ] Client counter test passes.
- [ ] Server counter test passes.

**Verification:**
- [ ] `cargo test --features sync,rtu,metrics metrics` passes.

**Dependencies:** Task 12.2, Task 12.3

**Files likely touched:**
- `tests/metrics.rs` (new)

**Estimated scope:** S

---

### Checkpoint: Configuration and observability

- [ ] Tasks 10.1–10.5, 11.1–11.4, 12.1–12.4 merged.
- [ ] `cargo test --all-features` passes.
- [ ] README updated with config, tracing, and metrics examples.
- [ ] Review with human before starting Phase 5.

---

## Phase 5 — Ecosystem maturity

### Task 13.1: TCP loopback tests (sync + async)

**Description:** Ensure sync and async TCP clients and servers exercise a read/write loopback.

**Acceptance criteria:**
- [ ] Sync TCP loopback test passes.
- [ ] Async TCP loopback test passes.

**Verification:**
- [ ] `cargo test --features loopback-tests tcp_loopback` passes.

**Dependencies:** Phase 1–3

**Files likely touched:**
- `tests/loopback_tcp.rs` (new or extended)

**Estimated scope:** S

---

### Task 13.2: UDP loopback tests (sync + async)

**Description:** Add sync and async UDP loopback tests.

**Acceptance criteria:**
- [ ] Sync UDP loopback test passes.
- [ ] Async UDP loopback test passes.

**Verification:**
- [ ] `cargo test --features loopback-tests udp_loopback` passes.

**Dependencies:** Phase 1–3

**Files likely touched:**
- `tests/loopback_udp.rs` (new or extended)

**Estimated scope:** S

---

### Task 13.3: RTU and ASCII stream loopback tests (sync + async)

**Description:** Add loopback tests for RTU and ASCII over in-memory byte streams.

**Acceptance criteria:**
- [ ] RTU sync/async loopback tests pass.
- [ ] ASCII sync/async loopback tests pass.

**Verification:**
- [ ] `cargo test --features loopback-tests rtu_loopback` passes.
- [ ] `cargo test --features loopback-tests ascii_loopback` passes.

**Dependencies:** Phase 1–3

**Files likely touched:**
- `tests/loopback_rtu.rs` (new or extended)
- `tests/loopback_ascii.rs` (new or extended)

**Estimated scope:** S

---

### Task 13.4: TLS and RTU-over-TCP loopback tests

**Description:** Add loopback tests for TLS and RTU-over-TCP transports in both sync and async modes.

**Acceptance criteria:**
- [ ] TLS loopback test passes.
- [ ] RTU-over-TCP loopback test passes.

**Verification:**
- [ ] `cargo test --features loopback-tests tls_loopback` passes.
- [ ] `cargo test --features loopback-tests rtu_over_tcp_loopback` passes.

**Dependencies:** Phase 1

**Files likely touched:**
- `tests/loopback_tls.rs` (new)
- `tests/loopback_rtu_over_tcp.rs` (new)

**Estimated scope:** M

---

### Task 13.5: Update loopback-tests documentation

**Description:** Update `docs/loopback-tests.md` to describe all covered combinations and how to run them.

**Acceptance criteria:**
- [ ] Document lists all transport × sync/async combinations.
- [ ] Commands are copy-pasteable.

**Verification:**
- [ ] Manual review.

**Dependencies:** Task 13.1–13.4

**Files likely touched:**
- `docs/loopback-tests.md`

**Estimated scope:** XS

---

### Task 14.1: TCP client/server examples

**Description:** Add minimal `examples/tcp_client.rs` and `examples/tcp_server.rs`.

**Acceptance criteria:**
- [ ] Examples compile.
- [ ] Demonstrate basic read/write.

**Verification:**
- [ ] `cargo build --example tcp_client --features sync,tcp` succeeds.
- [ ] `cargo build --example tcp_server --features sync,tcp` succeeds.

**Dependencies:** Phase 1–3

**Files likely touched:**
- `examples/tcp_client.rs` (new)
- `examples/tcp_server.rs` (new)

**Estimated scope:** S

---

### Task 14.2: RTU and ASCII examples

**Description:** Add serial/stream examples for RTU and ASCII.

**Acceptance criteria:**
- [ ] RTU example compiles.
- [ ] ASCII example compiles.

**Verification:**
- [ ] `cargo build --example rtu_client --features sync,rtu` succeeds.
- [ ] `cargo build --example ascii_client --features sync,ascii` succeeds.

**Dependencies:** Phase 1–3

**Files likely touched:**
- `examples/rtu_client.rs` (new)
- `examples/ascii_client.rs` (new)

**Estimated scope:** S

---

### Task 14.3: TLS and RTU-over-TCP examples

**Description:** Add examples for TLS client and RTU-over-TCP client/server.

**Acceptance criteria:**
- [ ] TLS example compiles.
- [ ] RTU-over-TCP example compiles.

**Verification:**
- [ ] `cargo build --example tls_client --features sync,tcp,tls` succeeds.
- [ ] `cargo build --example rtu_over_tcp --features sync,rtu,tcp` succeeds.

**Dependencies:** Phase 1

**Files likely touched:**
- `examples/tls_client.rs` (new)
- `examples/rtu_over_tcp.rs` (new)

**Estimated scope:** S

---

### Task 14.4: Typed helper example

**Description:** Add an example showing `read_holding_registers_f32` and `write_multiple_registers_u32`.

**Acceptance criteria:**
- [ ] Example compiles.

**Verification:**
- [ ] `cargo build --example typed_helpers --features sync,tcp,helpers` succeeds.

**Dependencies:** Phase 2

**Files likely touched:**
- `examples/typed_helpers.rs` (new)

**Estimated scope:** XS

---

### Task 14.5: Update README with examples index

**Description:** Link all new examples from the README with short descriptions.

**Acceptance criteria:**
- [ ] README lists every example added in Phase 5.

**Verification:**
- [ ] Manual review.

**Dependencies:** Task 14.1–14.4

**Files likely touched:**
- `README.md`

**Estimated scope:** XS

---

### Task 15.1: Review and complete `Cargo.toml` metadata

**Description:** Ensure all crates.io metadata is present and correct.

**Acceptance criteria:**
- [ ] `description`, `license`, `repository`, `keywords`, `categories` populated.
- [ ] `authors` or `rust-version` if required.

**Verification:**
- [ ] `cargo publish --dry-run` warning-free for metadata.

**Dependencies:** None

**Files likely touched:**
- `Cargo.toml`

**Estimated scope:** XS

---

### Task 15.2: Make README examples testable

**Description:** Convert README code blocks into doc tests or use `doc-comment` so they are checked in CI.

**Acceptance criteria:**
- [ ] README examples compile.
- [ ] `cargo test --doc` passes.

**Verification:**
- [ ] `cargo test --doc --features sync,tcp` passes.

**Dependencies:** Phase 1–4

**Files likely touched:**
- `README.md`
- `src/lib.rs` (doc tests)
- `Cargo.toml` (if `doc-comment` added)

**Estimated scope:** S

---

### Task 15.3: Write CHANGELOG for release

**Description:** Create or update `CHANGELOG.md` covering all changes since the last release.

**Acceptance criteria:**
- [ ] CHANGELOG has an `Unreleased` or `0.2.0` section.
- [ ] Major features, fixes, and breaking changes listed.

**Verification:**
- [ ] Manual review.

**Dependencies:** Phase 1–4

**Files likely touched:**
- `CHANGELOG.md` (new or updated)

**Estimated scope:** XS

---

### Task 15.4: Final publish dry-run

**Description:** Run `cargo publish --dry-run` with all features and fix any warnings.

**Acceptance criteria:**
- [ ] `cargo publish --dry-run` succeeds.
- [ ] No new warnings.

**Verification:**
- [ ] `cargo publish --dry-run --features ...` succeeds.

**Dependencies:** Task 15.1–15.3

**Files likely touched:**
- Any file flagged by dry-run

**Estimated scope:** S

---

### Checkpoint: Complete

- [ ] Tasks 13.1–13.5, 14.1–14.5, 15.1–15.4 merged.
- [ ] `cargo test --all-features` passes.
- [ ] `cargo publish --dry-run` succeeds.
- [ ] Documentation and examples updated.
- [ ] Ready for review and release.

---

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| TLS dependency choice (rustls vs native-tls) blocks embedded users | Medium | Make TLS optional; prefer rustls for portability but keep the door open for native-tls behind a second feature. |
| Sync serial `serialport` crate is large and pulls in `libudev` on Linux | Medium | Gate behind `sync-serial` feature; document system dependencies. |
| Retry/reconnect adapter hides real errors or loops forever | High | Expose strict policy defaults and unit-test every failure mode. |
| Typed helpers bloat the client API | Low | Keep helpers behind `helpers` feature and delegate to existing conversion functions. |
| Async RTU server needs real serial hardware for meaningful testing | Medium | Test with `tokio::io::DuplexStream` and `tokio-serial` loopback; mark hardware tests `#[ignore]`. |
| Configuration format choice (JSON/TOML/YAML) adds dependencies | Low | Gate each format behind its own feature or pick one default (TOML) and document alternatives. |

## Open questions

1. **TLS library:** `rustls` or `native-tls`? → Decide before Task 1.1.
2. **Sync serial crate:** `serialport` acceptable? → Decide before Task 3.1.
3. **Config format default:** TOML default or explicit format features? → Decide before Task 10.1.
4. **Scope of Phase 5:** Publish now or wait for user feedback? → Decide before Task 15.1.

## Notes

- This plan intentionally does not add Redis, Prometheus, or other external store/metrics backends. Those can be added as examples or follow-up issues once the hook and metrics seams exist.
- Phases can be reordered if priorities change, but internal task dependencies must be respected.
