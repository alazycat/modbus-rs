# Roadmap: Close remaining gaps with modbus-go

> Status as of 2026-07-09. The original gap analysis in
> [`docs/comparison-with-modbus-go.md`](comparison-with-modbus-go.md) is now
> largely out of date: most transport, client-ergonomics, server, config, and
> metrics items have already shipped. This roadmap focuses on the remaining
> high-value gaps and the polish needed for a stable release.

## Overview

This plan turns the *still-open* gaps from `comparison-with-modbus-go.md` into
small, verifiable tasks. Each task is sized for a single focused session, ends
with passing tests, and leaves the crate in a working state.

## Architecture decisions

- **Runtime seams first.** Connection-lifetime concerns (idle timeout, keep-alive)
  are added behind the existing `Transport` / `AsyncTransport` seams so every
  transport benefits without per-protocol duplication.
- **Hooks as a server seam.** Request/response hooks are introduced as a
  pluggable seam *around* `Server::dispatch`, not by mutating `DataStore`.
- **CLI reuses library features.** The CLI will delegate to existing transports
  and helpers rather than reimplementing them.
- **Tracing and metrics remain optional.** No runtime cost when the features are
  disabled.

## Current gap inventory

| # | Gap | Status | Notes |
|---|-----|--------|-------|
| 1 | TLS over TCP | **Done** | `tls` feature; sync + async client/server. |
| 2 | RTU over TCP | **Done** | Client helpers + `RtuOverTcpServer` / `AsyncRtuOverTcpServer`. |
| 3 | Sync serial RTU/ASCII | **Done** | `sync-serial` feature + `SerialTransport`. |
| 4 | Typed client helpers | **Done** | `read_holding_registers_u32/f32/string` etc. behind `helpers`. |
| 5 | Auto-reconnect / retry | **Done** | `RetryAdapter` / `AsyncRetryAdapter` with `RetryPolicy`. |
| 6 | Broadcast RTU/ASCII | **Done** | Client skips recv, server suppresses response for unit ID 0. |
| 7 | Idle timeout / keep-alive | **Missing** | Only per-operation timeouts exist. |
| 8 | Async RTU server | **Done** | `AsyncRtuServer` + serial integration. |
| 9 | Pluggable `DataStore` | **Extensible** | Public trait exists; no built-in file-backed example. |
| 10 | Request hooks / middleware | **Missing** | No auth, rate-limit, or interception seam. |
| 11 | Config file loading | **Done** | JSON/TOML/YAML loaders behind `config` feature. |
| 12 | Tracing instrumentation | **Partial** | Present but sparse; not uniform across all transports. |
| 13 | Optional metrics | **Done** | Atomic counters wired into client/server/retry. |
| 14 | CLI tool | **Partial** | Missing TLS, RTU-over-TCP, UDP, ASCII, typed helpers, config files. |

---

## Phase 1 — Runtime connectivity seams

### Task 1.1: Design idle-timeout / keep-alive seam

**Description:** Decide where connection-lifetime management lives. Options:
- Add optional `idle_timeout` / `keep_alive` fields to `ClientConfig` and let
  transports apply them to the underlying stream.
- Wrap streams in a small `KeepAlive` adapter at the `Transport` seam.

Document the decision in a short ADR or task notes.

**Acceptance criteria:**
- [x] API shape chosen (`ClientConfig` fields vs adapter).
- [x] Behavior defined for TCP, RTU-over-TCP, and serial streams.
- [x] No breaking change to existing constructors.

**Verification:**
- [x] Human review of the design sketch.

**Dependencies:** None

**Files likely touched:**
- `docs/adr/idle-timeout.md` (new) or task comments

**Estimated scope:** XS

---

### Task 1.2: Add idle-timeout support to sync transports

**Description:** If the design from Task 1.1 uses `ClientConfig`, apply
`TcpStream::set_read_timeout` / `set_write_timeout` and serial-port timeouts so
that an idle connection is closed after the configured duration. If an adapter
approach is chosen, implement the adapter for `std::io::Read + Write`.

**Acceptance criteria:**
- [x] `ClientConfig` gains an optional `idle_timeout` field, or a new adapter
      exists.
- [x] Sync TCP, RTU-over-TCP, and serial transports respect it.
- [x] Idle timeout maps to `TransportError::Timeout` or `Disconnected` as
      appropriate.

**Verification:**
- [x] `cargo test --features sync,tcp idle_timeout` passes.
- [x] `cargo test --features sync,rtu,sync-serial idle_timeout` passes.

**Dependencies:** Task 1.1

**Files likely touched:**
- `src/client/mod.rs`
- `src/tcp_transport.rs`
- `src/rtu_transport.rs`
- `src/serial_transport.rs`

**Estimated scope:** M

---

### Task 1.3: Add idle-timeout support to async transports

**Description:** Mirror Task 1.2 for tokio-based streams. Use
`tokio::net::TcpStream::set_linger` / `tokio::time::timeout` patterns, or a
wrapper adapter that resets an inactivity timer on every read/write.

**Acceptance criteria:**
- [x] Async TCP, RTU-over-TCP, and serial transports respect the idle timeout.
- [x] Timeout behavior is consistent with sync variants.

**Verification:**
- [x] `cargo test --features async,tcp idle_timeout` passes.
- [x] `cargo test --features async,rtu,serial idle_timeout` passes.

**Dependencies:** Task 1.2

**Files likely touched:**
- `src/tcp_transport.rs`
- `src/rtu_transport.rs`
- `src/serial_transport.rs` (async path)

**Estimated scope:** M

---

### Checkpoint: Runtime connectivity
- [x] Tasks 1.1–1.3 merged.
- [x] `cargo test --features "rtu sync async tcp sync-serial serial"` passes.
- [x] Design documented before Phase 2 starts.

---

## Phase 2 — Server hooks and middleware seam

### Task 2.1: Design `RequestHook` trait

**Description:** Define a hook interface invoked around `Server::dispatch`:
- `before_request(req_pdu, unit_id) -> Result<(), ExceptionResponse>`
- `after_response(req_pdu, resp_pdu, unit_id)`

Decide whether hooks are async-aware, whether they can mutate requests, and how
multiple hooks compose.

**Acceptance criteria:**
- [x] Trait signature chosen and reviewed.
- [x] Composition strategy documented (single hook generic vs Vec of hooks).

**Verification:**
- [ ] Human review of the API sketch.

**Dependencies:** None

**Files likely touched:**
- `docs/adr/request-hooks.md` (new)

**Estimated scope:** XS

---

### Task 2.2: Implement `RequestHook` trait and no-op default

**Description:** Add `src/server/hook.rs` with the trait and a `NoopHook`
implementation. Export it under the existing `server` module.

**Acceptance criteria:**
- [x] `RequestHook` trait compiles under `sync` and `async`.
- [x] `NoopHook` is the default and introduces no behavior change.

**Verification:**
- [ ] `cargo check --features sync` succeeds.
- [ ] `cargo check --features async` succeeds.

**Dependencies:** Task 2.1

**Files likely touched:**
- `src/server/hook.rs` (new)
- `src/server/mod.rs`

**Estimated scope:** S

---

### Task 2.3: Integrate hooks into sync server

**Description:** Update `src/server/sync.rs` so `Server` can hold an optional
`Box<dyn RequestHook>` (or generic `H: RequestHook`) and invoke it around
`dispatch`.

**Acceptance criteria:**
- [x] Sync server supports a hook without changing public `dispatch` signature
      by default.
- [x] Hook rejection produces an exception response.

**Verification:**
- [ ] `cargo test --features sync` passes.
- [ ] New hook unit test passes.

**Dependencies:** Task 2.2

**Files likely touched:**
- `src/server/sync.rs`

**Estimated scope:** S

---

### Task 2.4: Integrate hooks into async server

**Description:** Mirror Task 2.3 for `src/server/async.rs` and
`AsyncServer`.

**Acceptance criteria:**
- [x] Async server supports the same hook seam.

**Verification:**
- [ ] `cargo test --features async` passes.

**Dependencies:** Task 2.3

**Files likely touched:**
- `src/server/async.rs`

**Estimated scope:** S

---

### Task 2.5: Add logging hook example

**Description:** Provide `examples/logging_hook.rs` that prints each request and
response. This doubles as the first non-trivial hook implementation.

**Acceptance criteria:**
- [x] Example compiles.
- [x] README or example doc explains how to run it.

**Verification:**
- [ ] `cargo build --example logging_hook --features sync,tcp` succeeds.

**Dependencies:** Task 2.4

**Files likely touched:**
- `examples/logging_hook.rs` (new)

**Estimated scope:** XS

---

### Checkpoint: Server hooks
- [x] Tasks 2.1–2.5 merged.
- [x] `cargo test --features "sync async"` passes.
- [x] Hook API documented.

---

## Phase 3 — Observability and CLI polish

### Task 3.1: Unify tracing across all transports and servers

**Description:** Audit every transport `send`/`recv` and every server dispatch
path. Add `tracing::trace!`/`debug!` events where missing, and ensure spans
include transport type, function code, and unit ID where available. Keep all
instrumentation behind `cfg(feature = "tracing")`.

**Acceptance criteria:**
- [x] TCP, RTU, ASCII, UDP, serial, TLS, and RTU-over-TCP transports emit
      send/recv trace events.
- [x] Sync and async servers emit dispatch spans.
- [x] No tracing code compiled when the feature is disabled.

**Verification:**
- [x] `cargo test --all-features` passes.
- [x] `cargo test --features sync,tcp` passes.

**Dependencies:** None

**Files likely touched:**
- `src/tcp_transport.rs`
- `src/rtu_transport.rs`
- `src/ascii_transport.rs`
- `src/udp_transport.rs`
- `src/serial_transport.rs`
- `src/server/sync.rs`
- `src/server/async.rs`
- `src/tcp_server.rs`, `src/rtu_server.rs`, etc.

**Estimated scope:** M

---

### Task 3.2: Expose TLS in the CLI

**Description:** Add `--tls-cert` / `--tls-key` options to `modbus-cli` for both
TCP client and TCP server modes.

**Acceptance criteria:**
- [ ] `modbus-cli tcp-client --tls ...` connects via TLS.
- [ ] `modbus-cli tcp-server --tls-cert ... --tls-key ...` serves TLS.
- [ ] Compiles under `cli,tls` features.

**Verification:**
- [ ] `cargo build --bin modbus-cli --features cli,tls` succeeds.

**Dependencies:** None

**Files likely touched:**
- `src/bin/modbus_cli.rs`

**Estimated scope:** S

---

### Task 3.3: Expose RTU-over-TCP, UDP, and ASCII in the CLI

**Description:** Extend CLI subcommands to support the remaining transports and
framing modes.

**Acceptance criteria:**
- [ ] RTU-over-TCP client/server subcommands exist.
- [ ] UDP client/server subcommands exist.
- [ ] ASCII client/server subcommands exist (serial + stream where applicable).

**Verification:**
- [ ] `cargo build --bin modbus-cli --features cli,rtu,tcp` succeeds.
- [ ] `cargo build --bin modbus-cli --features cli,udp` succeeds.
- [ ] `cargo build --bin modbus-cli --features cli,ascii` succeeds.

**Dependencies:** None

**Files likely touched:**
- `src/bin/modbus_cli.rs`

**Estimated scope:** M

---

### Task 3.4: Support config-file loading in the CLI

**Description:** Add `--config <path>` to `modbus-cli` so client/server can be
started from JSON/TOML/YAML files when the `config` feature is enabled.

**Acceptance criteria:**
- [ ] `--config` flag deserializes `ClientConfigFile` / `ServerConfigFile`.
- [ ] Works for at least TCP and RTU clients.

**Verification:**
- [ ] `cargo build --bin modbus-cli --features cli,config` succeeds.
- [ ] `cargo test --features cli,config` passes.

**Dependencies:** Task 3.3

**Files likely touched:**
- `src/bin/modbus_cli.rs`
- `src/config.rs` (minor adjustments if needed)

**Estimated scope:** S

---

### Task 3.5: Add typed-helper commands to the CLI

**Description:** Add high-level commands like `read-holding-u32`,
`write-multiple-f32`, etc., that exercise the `helpers` feature.

**Acceptance criteria:**
- [ ] At least read/write u32, f32, and string commands exist.
- [ ] Endianness/word-order flags exposed.

**Verification:**
- [ ] `cargo build --bin modbus-cli --features cli,helpers` succeeds.

**Dependencies:** Task 3.4

**Files likely touched:**
- `src/bin/modbus_cli.rs`

**Estimated scope:** S

---

### Checkpoint: Observability and CLI
- [ ] Tasks 3.1–3.5 merged.
- [ ] `cargo test --all-features` passes.
- [ ] CLI help output reviewed.

---

## Phase 4 — Documentation and release readiness

### Task 4.1: Update `comparison-with-modbus-go.md`

**Description:** Refresh the comparison doc to reflect the current feature set
and clearly mark the remaining gaps (idle timeout, hooks, CLI polish).

**Acceptance criteria:**
- [ ] All "done" items are marked as implemented.
- [ ] Remaining gaps are accurate and limited to idle timeout, hooks, and CLI.

**Verification:**
- [ ] Manual review.

**Dependencies:** Phases 1–3

**Files likely touched:**
- `docs/comparison-with-modbus-go.md`

**Estimated scope:** S

---

### Task 4.2: Add missing examples

**Description:** Ensure every transport and major feature has a minimal
compiling example:
- `examples/tcp_client.rs`, `examples/tcp_server.rs`
- `examples/rtu_client.rs`, `examples/ascii_client.rs`
- `examples/rtu_over_tcp_client.rs`
- `examples/tls_client.rs`
- `examples/typed_helpers.rs`
- `examples/config_client.rs` / `examples/config_server.rs`

**Acceptance criteria:**
- [ ] Each example compiles with the correct feature set.
- [ ] Examples are listed in `Cargo.toml` where required.

**Verification:**
- [ ] `cargo build --example <name> --features ...` succeeds for each.

**Dependencies:** Phases 1–3

**Files likely touched:**
- `examples/*.rs` (new)
- `Cargo.toml`

**Estimated scope:** M

---

### Task 4.3: Update README and CHANGELOG

**Description:** Refresh README with current feature flags, transport matrix,
and example commands. Add a CHANGELOG entry for the next release.

**Acceptance criteria:**
- [ ] README transport table matches current code.
- [ ] CHANGELOG has an `Unreleased` or `0.2.0` section.

**Verification:**
- [ ] Manual review.

**Dependencies:** Task 4.2

**Files likely touched:**
- `README.md`
- `CHANGELOG.md`

**Estimated scope:** S

---

### Task 4.4: Final `cargo publish --dry-run`

**Description:** Run a publish dry-run with all features and fix any metadata,
documentation, or dependency warnings.

**Acceptance criteria:**
- [ ] `cargo publish --dry-run --all-features` succeeds.
- [ ] No new warnings.

**Verification:**
- [ ] Command output clean.

**Dependencies:** Tasks 4.1–4.3

**Files likely touched:**
- `Cargo.toml`
- Any file flagged by dry-run

**Estimated scope:** S

---

### Checkpoint: Release ready
- [ ] Tasks 4.1–4.4 merged.
- [ ] `cargo test --all-features` passes.
- [ ] `cargo publish --dry-run` succeeds.
- [ ] README, CHANGELOG, and comparison doc are current.

---

## Risks and mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Idle-timeout semantics differ across OS serial ports | Medium | Document expected behavior; test on Windows/Linux where possible. |
| Hook trait becomes too generic / async-sync split | Medium | Start with a sync-only trait that is also `Send`; add async variant only if needed. |
| CLI feature-combination explosion | Medium | Use `required-features` per example/bin; keep CLI code feature-gated. |
| Tracing bloats release builds | Low | All tracing stays behind `cfg(feature = "tracing")`. |

## Open questions

1. Should idle timeout be a `ClientConfig` field or a stream adapter? → Decide in Task 1.1.
2. Should `RequestHook` be generic on `Server` or stored as `Box<dyn ...>`? → Decide in Task 2.1.
3. Which CLI transports are highest priority if time is short? → Default to TCP/TLS first, then RTU-over-TCP.
