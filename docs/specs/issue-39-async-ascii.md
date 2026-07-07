# Spec: Async ASCII client/server for coils & registers (issue #39)

## Objective
Add tokio-based async ASCII transport, client, and server that mirror the existing synchronous ASCII implementation (#22/#31/#32) and build on the async runtime core (#35/#36). The public API should offer `AsyncAsciiTransport`, `AsyncAsciiClient`, and `AsyncAsciiServer` with the same coils/registers methods as the sync versions.

## Tech Stack
- Rust 2021 edition, existing crate feature flags
- `tokio` (already a dependency under the `async` feature)
- Native async traits / RPITIT (`async fn` in trait or `impl Future + Send`)

## Commands
- Build: `cargo build --features "ascii async"`
- Test: `cargo test --features "ascii async" --lib`
- Lint: `cargo clippy --features "ascii async" --lib -- -D warnings`
- Full regression: `cargo test --all-features --lib`

## Project Structure
- `src/ascii.rs` — existing ASCII ADU framing, unchanged
- `src/ascii_transport.rs` — add `AsyncAsciiTransport` under `#[cfg(all(feature = "ascii", feature = "async"))]`
- `src/ascii_client.rs` — add `AsyncAsciiClient` under `#[cfg(all(feature = "ascii", feature = "async"))]`
- `src/ascii_server.rs` — add `AsyncAsciiServer` under `#[cfg(all(feature = "ascii", feature = "async"))]`
- `src/lib.rs` — no new module declarations needed; existing `ascii_transport`, `ascii_client`, `ascii_server` are already declared under `all(ascii, sync)`. Relax those gates to `any(sync, async)`.

## Code Style
Follow the existing sync ASCII modules and the async core modules:
- `AsyncAsciiTransport<T>` wraps a `tokio::io::AsyncRead + AsyncWrite + Unpin` stream.
- `AsyncAsciiClient<T: AsyncTransport>` delegates to `AsyncClient` logic but wraps PDUs in `AsciiAdu` instead of `RtuAdu`.
- `AsyncAsciiServer<D: DataStore>` wraps `AsyncServer` but uses ASCII framing.
- Reuse `ClientConfig`/`ClientError` from `crate::client` and `DataStore`/`MemoryStore`/`Server` from `crate::server`.
- Use `tokio::io::AsyncReadExt` / `AsyncWriteExt` for byte-stream I/O.

## Testing Strategy
- Unit tests in each new async module:
  - `AsyncAsciiTransport`: round-trip frame send/receive over a tokio duplex stream, garbage skipping, partial-frame timeout.
  - `AsyncAsciiClient`: loopback test dispatching through an in-memory `AsyncServer`, plus exception/wrong-slave handling.
  - `AsyncAsciiServer`: respond to a matching ASCII read-coils request, ignore non-matching address, process broadcast, loop until EOF.
- All tests run with `cargo test --features "ascii async" --lib`.

## Boundaries
- Always: run tests and clippy before committing; keep feature gates consistent.
- Ask first: adding new dependencies, changing the public API of existing sync types.
- Never: modify the existing ASCII ADU encoder/decoder (`src/ascii.rs`) beyond feature-gate relaxation; delete or skip existing sync tests.

## Success Criteria
- [ ] `cargo test --features "ascii async" --lib` passes
- [ ] `cargo clippy --features "ascii async" --lib -- -D warnings` is clean
- [ ] `cargo test --all-features --lib` still passes
- [ ] Public API exposes `AsyncAsciiTransport`, `AsyncAsciiClient`, and `AsyncAsciiServer` when `ascii` + `async` features are enabled
- [ ] The implementation reuses the sync `Server` dispatch and async `AsyncTransport` trait already in the crate

## Open Questions
None — issue #39 acceptance criteria are explicit and all blockers (#22, #31, #32, #35, #36) are merged.
