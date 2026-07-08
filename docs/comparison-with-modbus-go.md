# Comparison with modbus-go

This document compares the current Rust `modbus` crate with the popular Go
implementation **[github.com/adibhanna/modbus-go](https://github.com/adibhanna/modbus-go)**.
The goal is to identify functional gaps and guide the project's roadmap.

## Scope note

The comparison targets `adibhanna/modbus-go` because it is the library whose
name matches "modbus-go" most directly and because it advertises itself as a
"production-ready", full-featured Modbus implementation. Other Go Modbus
libraries (e.g. `grid-x/modbus`, `simonvetter/modbus`, `aldas/go-modbus-client`)
are not covered here.

## High-level comparison

| Dimension | `modbus` (this crate) | `modbus-go` |
|---|---|---|
| Language / packaging | Rust, Cargo crate | Go, module |
| Transports | RTU, ASCII, TCP, UDP | TCP, RTU, ASCII, UDP, Serial, RTU-over-TCP, **TLS** |
| I/O model | Sync + `tokio` async | Sync (goroutine-based concurrency) |
| Roles | Client + Server | Client + Server |
| Function codes | 19 public function codes | 19 public function codes |
| High-level data helpers | Present under `helpers` feature, but not yet integrated into client API | Built into client: `uint32/64`, `int32/64`, `float32/64`, `string`, raw bytes, with configurable endianness/word order |
| Auto-reconnect / retries | Not currently implemented | Built-in |
| Broadcast (unit ID 0) | Not currently implemented | Supported |
| Configuration | Cargo feature flags | JSON-based configuration |
| Target platforms | Optional `no_std` + `alloc` | Go standard library targets |
| CLI tool | `modbus-cli` (under `cli` feature) | Not advertised |
| Maturity | Version `0.1.0`, active development | Advertised as production-ready |

## Transport coverage gap

`modbus-go` supports two transport variants that this crate does not yet
implement:

- **TLS over TCP**: encrypted Modbus TCP, increasingly required in industrial
  deployments.
- **RTU over TCP**: wrapping RTU frames inside a TCP stream, commonly used with
  serial-to-Ethernet converters.

In addition, this crate's serial support is currently async-only (`serial`
feature requires `rtu + async + tokio-serial`). A synchronous serial RTU/ASCII
path would widen the embedded/PLC use cases.

## Client API gap

The `helpers` module already provides byte/endianness conversions for `u16`,
`u32`, `u64`, signed variants, `f32`, `f64`, and strings. However, callers must
still:

1. Issue `read_holding_registers(...)` (or similar).
2. Manually convert the returned `[u16]` via helpers such as
   `u32_from_registers`.

`modbus-go` exposes high-level methods like `ReadHoldingRegistersAsUint32` that
combine the two steps. Bridging this gap would significantly improve ergonomics.

Other client runtime features present in `modbus-go` but missing here:

- Automatic reconnect and retry policy.
- Broadcast requests (no response expected).
- Idle connection timeout / keep-alive.

## Server capabilities gap

- **Async RTU server**: this crate currently exposes `rtu_server` only under the
  `sync` feature. An async equivalent is needed for parity with the async TCP/UDP
  servers.
- **Pluggable data store**: `MemoryStore` exists, but there is no built-in path
  for Redis, file-backed, or user-provided stores.
- **Request hooks / middleware**: logging, authentication, and rate limiting
  require extension points.

## Configuration and observability gap

`modbus-go` supports JSON configuration and a pluggable logger. This crate
already depends on `tracing` for the CLI feature, but runtime logging and
metrics are not unified across client/server code. A configuration loader
(JSON, TOML, or YAML) would also help server deployments.

## Suggested roadmap

The following ordering is intentionally lazy: fill the highest-value gaps first
before adding polish.

### Phase 1 — Transport parity (high priority)

1. Add **TLS over TCP** transport.
2. Add **RTU over TCP** framing/transport.
3. Add **synchronous serial RTU/ASCII** support (currently async-only).

### Phase 2 — Client ergonomics (high priority)

1. Integrate `helpers` into the client API, e.g.
   `client.read_holding_registers_u32(...)`,
   `client.read_holding_registers_f32(...)`, etc.
2. Add automatic reconnect and configurable retry policy.
3. Add broadcast support for RTU/ASCII.

### Phase 3 — Server and runtime (medium priority)

1. Implement async RTU server.
2. Add pluggable `DataStore` examples (file-backed, custom trait usage).
3. Add request hooks for logging, metrics, and access control.

### Phase 4 — Configuration and observability (medium priority)

1. Add configuration file loading (JSON/TOML/YAML) for client and server.
2. Unify `tracing` instrumentation across transports and runtimes.
3. Add optional metrics (requests, exceptions, timeouts, retries).

### Phase 5 — Ecosystem maturity (lower priority)

1. Expand `loopback-tests` to cover all `transport × sync/async` combinations.
2. Add a cookbook of minimal examples per transport.
3. Publish to crates.io and move toward a stable `1.0` API.

## Sources

- [github.com/adibhanna/modbus-go](https://github.com/adibhanna/modbus-go)
- [pkg.go.dev/github.com/adibhanna/modbus-go](https://pkg.go.dev/github.com/adibhanna/modbus-go)
