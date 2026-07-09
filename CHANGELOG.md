# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **TLS over TCP** (`tls` feature): async and sync client/server support via
  `tokio-rustls` / `rustls`.
- **RTU-over-TCP**: sync and async client (`Client::connect_rtu_over_tcp`,
  `AsyncClient::connect_rtu_over_tcp`) plus `RtuOverTcpServer` and
  `AsyncRtuOverTcpServer`.
- **Sync serial RTU/ASCII** (`sync-serial` feature): `Client::connect_serial_rtu`
  and `AsciiClient::connect_serial_ascii` using `serialport`.
- **Async RTU server**: `AsyncRtuServer` with serial helper.
- **Typed client helpers** (`helpers` feature): `read_holding_registers_u32`,
  `write_multiple_registers_f32`, string helpers, etc., directly on all
  sync/async clients with configurable endianness and word order.
- **Retry / reconnect**: `RetryAdapter` and `AsyncRetryAdapter` with
  configurable `RetryPolicy`.
- **Broadcast support**: RTU/ASCII clients skip response reads for unit ID 0;
  servers suppress broadcast responses.
- **Idle timeout**: `ClientConfig.idle_timeout` applied across all transports.
- **Request hooks** (`server::hook::RequestHook`): pluggable seam around server
  dispatch with `NoopHook` default; `examples/logging_hook.rs` demonstrates
  usage.
- **Configuration file loading** (`config` feature): JSON/TOML/YAML loaders via
  `ClientConfigFile` and `ServerConfigFile`.
- **Optional metrics** (`metrics` feature): atomic counters for requests,
  exceptions, timeouts, and retries.
- **Optional tracing** (`tracing` feature): `trace!`/`debug!` events across
  transports and server dispatch paths.
- **CLI polish**: `modbus-cli` now supports TCP, TLS, UDP, RTU-over-TCP, RTU
  serial, ASCII serial, typed helpers, and `--config` file loading.
- **Shared store**: thread-safe `SharedStore` wrapper around `MemoryStore`.

### Changed

- `ServerConfigFile` now carries `unit_id` and store sizes so a server can be
  started entirely from a config file.
- README refreshed with feature flags, transport matrix, CLI examples, and a
  link to the comparison document.

## [0.1.2] - 2024-??-??

### Added

- Initial public crate with TCP, UDP, RTU, ASCII, sync/async clients and
  servers, and `modbus-cli`.

[Unreleased]: https://github.com/netmare/modbus-rs/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/netmare/modbus-rs/releases/tag/v0.1.2
