# Comparison with modbus-go

This document compares the current Rust `modbus` crate with the popular Go
implementation **[github.com/adibhanna/modbus-go](https://github.com/adibhanna/modbus-go)**.
The original goal was to identify functional gaps; as of version `0.1.3` the
gap inventory has flipped from "missing core features" to "missing examples
and release polish".

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
| Transports | TCP, UDP, RTU, ASCII, serial (async + sync), RTU-over-TCP, **TLS over TCP** | TCP, RTU, ASCII, UDP, serial, RTU-over-TCP, **TLS** |
| I/O model | Sync + `tokio` async | Sync (goroutine-based concurrency) |
| Roles | Client + Server | Client + Server |
| Function codes | 19 public function codes | 19 public function codes |
| High-level data helpers | Integrated via `ClientMethods` behind `helpers` feature: `u16`/`i16`, `u32`/`i32`, `u64`/`i64`, `f32`/`f64`, strings, raw bytes; configurable endianness and word order | Built into client: `uint32/64`, `int32/64`, `float32/64`, `string`, raw bytes, with configurable endianness/word order |
| Auto-reconnect / retries | Implemented: `RetryAdapter` / `AsyncRetryAdapter` with configurable `RetryPolicy` | Built-in |
| Broadcast (unit ID 0) | Implemented for RTU/ASCII client and server | Supported |
| Idle timeout / keep-alive | Implemented via `ClientConfig.idle_timeout` / `idle_timeout_ms`, applied to streams | Built-in |
| Request hooks / middleware | Implemented: `RequestHook` trait integrated into sync/async servers; `NoopHook` default; `logging_hook` example | Built-in logger |
| Pluggable `DataStore` | Public `DataStore` trait; `MemoryStore` and thread-safe `SharedStore` included; no built-in file/Redis adapters | Pluggable store |
| Configuration | Cargo feature flags + JSON/TOML/YAML config files behind `config` feature | JSON-based configuration |
| Observability | Optional `tracing` instrumentation across transports and servers; optional atomic `metrics` counters | Pluggable logger |
| Target platforms | Optional `no_std` + `alloc`; `std` default | Go standard library targets |
| CLI tool | `modbus-cli` (under `cli` feature): TCP, TLS, UDP, RTU-over-TCP, RTU/ASCII serial, typed helpers, config files | Not advertised |
| Maturity | Version `0.1.3`, active development | Advertised as production-ready |

## Transport coverage

This crate now matches `modbus-go` on every transport variant:

- **TCP**: sync + async client and server.
- **UDP**: async client and server.
- **RTU serial**: async via `tokio-serial` (`serial` feature) and sync via
  `serialport` (`sync-serial` feature), client and server.
- **ASCII serial**: async via `tokio-serial` and sync via `serialport`, client
  and server.
- **RTU over TCP**: sync + async client and server.
- **TLS over TCP**: async client and server via `tokio-rustls` / `rustls`.

## Client API

The `helpers` feature is no longer just a utility module: the `ClientMethods`
trait exposes typed methods such as `read_holding_registers_u32`,
`write_multiple_registers_f32`, and string helpers directly on TCP, RTU,
RTU-over-TCP, UDP, and ASCII clients. Callers no longer need to manually
convert `[u16]` registers.

Runtime resilience features:

- **Retry / reconnect**: `RetryAdapter` and `AsyncRetryAdapter` wrap any
  `AduAdapter` and rebuild the connection using a user-supplied factory.
- **Broadcast**: RTU/ASCII clients skip the response read for unit ID 0, and
  servers suppress the response.
- **Idle timeout**: all sync and async transports read `config.idle_timeout`
  and apply it to the underlying stream.

## Server capabilities

- **Sync server**: `Server<D: DataStore>` with request hooks and optional
  metrics.
- **Async server**: `AsyncServer<D: DataStore>` with the same hook and metrics
  seam.
- **RTU server**: sync `RtuServer` and async `AsyncRtuServer`, including serial
  helpers.
- **RTU-over-TCP server**: `RtuOverTcpServer` and `AsyncRtuOverTcpServer`.
- **TCP server**: `TcpServer` and `AsyncTcpServer`, including TLS.
- **DataStore**: public trait with `MemoryStore` and thread-safe `SharedStore`.
  Custom stores can be supplied without forking the crate.

## Configuration and observability

- **Config files**: `ClientConfigFile` and `ServerConfigFile` deserialize from
  JSON, TOML, or YAML when the `config` feature is enabled. They cover
  timeouts, retry policy, endianness/word order, store sizes, and unit ID.
- **Tracing**: optional `tracing::trace!`/`debug!` events on every transport
  send/recv and server dispatch path, all behind `cfg(feature = "tracing")`.
- **Metrics**: optional atomic counters for requests, exceptions, timeouts, and
  retries, behind `cfg(feature = "metrics")`.

## Remaining gaps

The core feature set is now on par with `modbus-go`. The remaining work is
ecosystem polish rather than functional parity:

1. **Pluggable `DataStore` examples**: the trait is public, but the crate ships
   only `MemoryStore` and `SharedStore`. A file-backed or Redis-backed example
   would demonstrate the seam.
2. **crates.io release**: package metadata and `--dry-run` are clean, and the
   repository is tagged. The final step is publishing to crates.io and
   announcing `0.2.0`.

Items recently resolved:

- **Example coverage**: every transport and major feature now has a minimal
  compiling example in `examples/`.
- **Loopback test matrix**: `tests/loopback.rs` covers all sync/async ×
  RTU/TCP/UDP/ASCII combinations using in-memory loopback transports.

## Sources

- [github.com/adibhanna/modbus-go](https://github.com/adibhanna/modbus-go)
- [pkg.go.dev/github.com/adibhanna/modbus-go](https://pkg.go.dev/github.com/adibhanna/modbus-go)
