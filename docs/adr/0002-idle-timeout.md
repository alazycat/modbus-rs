# ADR-0002: Idle timeout lives on `ClientConfig` and is applied at the transport seam

## Status

Accepted

## Context

The `modbus` crate supports many transport and runtime combinations: sync/async TCP,
RTU-over-TCP, TLS, serial RTU/ASCII, and UDP. Today each operation carries a per-call
`timeout`, but there is no connection-level idle timeout. A peer that stops sending
bytes can leave a connection open indefinitely on some platforms.

Adding idle-timeout / keep-alive behavior raises two design questions:

1. Where is the timeout configured?
2. Where is it enforced?

The configuration must be usable from both sync and async clients without forcing
either runtime into the other. The enforcement must work across stream types whose
idle-timeout capabilities differ:

- `std::net::TcpStream` has `set_read_timeout` / `set_write_timeout`.
- `serialport` ports have `set_timeout`.
- `tokio::net::TcpStream` has no built-in read timeout, so each read must be wrapped
  with `tokio::time::timeout`.

## Decision

We will add an optional `idle_timeout: Option<Duration>` field to `ClientConfig`.
Transports apply it at construction time (sync) or per read (async), falling back to
the existing response `timeout` when `idle_timeout` is `None`. This preserves all
existing constructors and default behavior.

Specifically:

1. `ClientConfig` gains `pub idle_timeout: Option<Duration>`, defaulting to `None`.
2. `ClientConfigFile` gains `pub idle_timeout_ms: Option<u64>` and maps it to the
   runtime field.
3. Sync connection helpers set the underlying stream / serial port timeout to
   `config.idle_timeout.unwrap_or(config.timeout)` before wrapping the transport.
   Affected helpers:
   - `Client::connect_rtu_over_tcp`
   - `Client::connect_serial_rtu`
   - `AsciiClient::connect_serial_ascii`
   - `TcpClient::connect_tls`
4. Async transports store the idle timeout internally and wrap each `read` call with
   `tokio::time::timeout(idle_timeout, ...)`, while still bounding the whole receive
   with the per-call `timeout`.
   Affected transports:
   - `AsyncTcpTransport`
   - `AsyncRtuTransport`
5. When an idle timeout fires, the transport returns `TransportError::Timeout`, which
   the client maps to `ClientError::Timeout`.

## Consequences

- **Locality improves**: idle-timeout policy lives in one config type; enforcement is
  co-located with each transport's existing read loop.
- **Leverage improves**: every client helper that uses these transports gets idle
  timeout behavior automatically.
- **Backward compatibility**: existing callers using `ClientConfig::default()` or
  constructors like `Client::new(transport)` see no behavior change.
- The per-call `timeout` remains the dominant bound when `idle_timeout` is `None`.
- Async transports grow a small extra field and a `with_idle_timeout` builder method.

## Rejected alternatives

- **Stream adapter wrapping `Read` / `AsyncRead`**: would have been runtime-agnostic,
  but would require a new public type and could not easily apply to `rustls::StreamOwned`
  or serial ports without extra trait machinery. Applying the timeout where the stream
  is already concrete is simpler.
- **Enforce idle timeout only in `ClientConfig::timeout`**: would conflate response
  timeout with inactivity timeout. A slow-but-active response could be cut off even
  though bytes are still arriving.
- **Add idle timeout to the `Transport` / `AsyncTransport` trait methods**: would have
  changed the public seam and forced every transport implementation to change. Keeping
  it as a transport-internal detail avoids that.

## Related

- `CONTEXT.md` — domain vocabulary: Client, Transport, ADU, Timeout
- `docs/roadmap.md` — Phase 1: Runtime connectivity seams
- `src/client/mod.rs` — `ClientConfig`
- `src/tcp_transport.rs`, `src/rtu_transport.rs` — transport implementations
