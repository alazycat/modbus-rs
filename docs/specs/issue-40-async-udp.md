# Spec: Async UDP client/server for coils & registers (issue #40)

## Objective
Add tokio-based async UDP transport, client, and server that mirror the existing synchronous UDP implementation (#24/#33/#34) and build on the async runtime core (#35/#36). The public API should offer `AsyncUdpTransport`, `AsyncUdpClient`, and `AsyncUdpServer` with the same coils/registers methods as the sync versions.

## Tech Stack
- Rust 2021 edition, existing crate feature flags
- `tokio` with `net` feature (already a dependency under the `async` feature)
- Native async traits / RPITIT

## Commands
- Build: `cargo build --features "udp async"`
- Test: `cargo test --features "udp async" --lib`
- Lint: `cargo clippy --features "udp async" --lib -- -D warnings`
- Full regression: `cargo test --all-features --lib`

## Project Structure
- `src/udp.rs` — existing UDP ADU/MBAP framing, unchanged
- `src/udp_transport.rs` — add `AsyncUdpTransport` under `#[cfg(all(feature = "udp", feature = "async"))]`
- `src/udp_client.rs` — add `AsyncUdpClient` under `#[cfg(all(feature = "udp", feature = "async"))]`
- `src/udp_server.rs` — add `AsyncUdpServer` under `#[cfg(all(feature = "udp", feature = "async"))]`
- `src/lib.rs` — relax module declarations for `udp_transport`, `udp_client`, `udp_server` from `all(udp, sync)` to `any(sync, async)`
- `src/client/mod.rs` — re-export `AsyncUdpClient` under `udp+async`

## Code Style
Follow the existing sync UDP modules and the async core modules:
- `AsyncUdpTransport` wraps a `tokio::net::UdpSocket` and a remote `SocketAddr`.
- `AsyncUdpClient<T: AsyncTransport>` wraps PDUs in `UdpAdu`, tracks transaction IDs, validates responses.
- `AsyncUdpServer<D: DataStore>` dispatches datagrams via sync `Server` and sends responses back to the originating peer.
- Reuse `UdpClientConfig`/`UdpClientError` and `UdpServerError` from the sync modules where possible.
- Use `tokio::time::timeout` for UDP recv timeout and `tokio::net::UdpSocket::send_to`/`recv_from`.

## Testing Strategy
- Unit tests in each new async module:
  - `AsyncUdpTransport`: round-trip send/receive over loopback UDP sockets, timeout when no response, zero-timeout behavior, reject response from wrong peer.
  - `AsyncUdpClient`: loopback test through an in-memory async server, transaction ID increment, mismatched transaction ID handling.
  - `AsyncUdpServer`: respond to matching unit ID, ignore non-matching unit ID, decode-error handling, loop behavior, read holding registers.
- All tests run with `cargo test --features "udp async" --lib`.

## Boundaries
- Always: run tests and clippy before committing; keep feature gates consistent.
- Ask first: adding new dependencies, changing the public API of existing sync types.
- Never: modify the existing UDP ADU encoder/decoder (`src/udp.rs`); delete or skip existing sync tests.

## Success Criteria
- [ ] `cargo test --features "udp async" --lib` passes
- [ ] `cargo clippy --features "udp async" --lib -- -D warnings` is clean
- [ ] `cargo test --all-features --lib` still passes
- [ ] Public API exposes `AsyncUdpTransport`, `AsyncUdpClient`, and `AsyncUdpServer` when `udp` + `async` features are enabled
- [ ] The implementation reuses the sync `Server` dispatch and async `AsyncTransport` trait already in the crate

## Open Questions
None — issue #40 acceptance criteria are explicit and all blockers (#24, #33, #34, #35, #36) are merged.
