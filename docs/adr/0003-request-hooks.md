# ADR-0003: Server request hooks live on `Server` and are invoked through `dispatch_with_hook`

## Status

Accepted

## Context

The `modbus` crate has a clean [`DataStore`](crate::server::DataStore) seam for
business logic, but no place to inject cross-cutting concerns such as logging,
rate limiting, or authorization around request handling. The existing server
entry point is `Server::dispatch`, which operates on a raw PDU and has no
knowledge of the transport-level unit ID.

Adding a hook seam raises several design questions:

1. **Where is the hook invoked?** Inside `Server::dispatch` would change a
   stable public signature. Invoking it inside every transport wrapper would
   duplicate the before/after logic.
2. **What information does the hook receive?** The PDU alone is not enough for
   many concerns; the transport unit ID / server address is also useful.
3. **Sync vs. async.** The crate has both sync and async server paths. A
   separate async trait would add complexity and force every hook author to
   think about futures.
4. **Composition.** Should a server hold a `Vec` of hooks, a single generic
   hook, or a single boxed trait object?

## Decision

We will introduce a synchronous [`RequestHook`](crate::server::RequestHook)
trait and invoke it through a new `Server::dispatch_with_hook` method. The
existing `Server::dispatch` signature and behavior remain unchanged.

Specifically:

1. `src/server/hook.rs` defines the trait:
   - `before_request(unit_id, request_pdu) -> Result<(), ExceptionResponse>`
   - `after_response(unit_id, request_pdu, response_pdu)`
   - Supertraits: `Debug + Send` so the trait object is object-safe and
     thread-safe.
2. `NoopHook` is provided as the default implementation; it never rejects and
   does nothing in `after_response`.
3. `Server` stores at most one hook as `Option<Box<dyn RequestHook>>`.
   Multiple concerns are composed inside a single hook implementation rather
   than through a `Vec` of hooks. This keeps the server struct small and the
   invocation logic single-threaded and predictable.
4. `Server::with_hook` and `Server::set_hook` attach a hook at construction
   time or runtime, respectively.
5. `Server::dispatch_with_hook(unit_id, request, response)` is the new public
   seam. It calls `before_request`, falls back to an exception response on
   rejection, then calls `dispatch`, and finally calls `after_response` with
   the produced response PDU.
6. Every transport wrapper (`TcpServer`, `RtuServer`, `AsciiServer`,
   `UdpServer`, and their async equivalents) calls
   `dispatch_with_hook` with its transport-specific unit ID / server address.
   This is the only place where the unit ID is available, so it is the only
   place where the hook can be correctly contextualized.
7. Async wrappers call the synchronous `RequestHook` from async code. No
   `async` trait is introduced because hooks are expected to be fast,
   non-blocking predicates or observers.

## Consequences

- **Locality improves**: hook policy is declared in one trait, and the
  before/dispatch/after orchestration lives in a single `Server` method.
- **Leverage improves**: every sync and async transport gets hook support
  automatically because they all route through `dispatch_with_hook`.
- **Backward compatibility**: existing callers of `Server::dispatch` and the
  wrapper constructors see no behavior change when no hook is attached.
- **Composition is explicit**: because only one hook is stored, authors who
  need multiple concerns implement one hook that delegates to inner helpers.
  This avoids ordering surprises and implicit chaining.
- **Hook cannot mutate the request**: the request is passed as `&[u8]`. This
  is intentional; request mutation would make caching, metrics, and tracing
   harder to reason about. Rejection is the primary extension point.

## Rejected alternatives

- **Async `RequestHook` trait**: would have required `async-trait` or RPITIT,
  increasing compile-time complexity and making simple logging hooks awkward.
  Synchronous hooks are sufficient for the expected use cases.
- **Hook invoked inside `Server::dispatch`**: would have required changing the
  signature of a widely used public method or making unit ID optional, which
  would have leaked transport context into the PDU seam.
- **`Vec<Box<dyn RequestHook>>` on `Server`**: would have added implicit
  ordering and iteration overhead for a feature that is typically used for
  one concern at a time. Composition inside a single hook is clearer.
- **Generic `H: RequestHook` on `Server`**: would have changed the type of
  every server and wrapper, propagating the generic through transport code
  and tests. A boxed trait object keeps the public types unchanged.

## Related

- `CONTEXT.md` — domain vocabulary: Server, DataStore, ADU, PDU, Unit ID
- `docs/roadmap.md` — Phase 2: Server hooks and middleware seam
- `src/server/hook.rs` — `RequestHook` trait and `NoopHook`
- `src/server/sync.rs` — `Server::dispatch_with_hook`
- `src/tcp_server.rs`, `src/rtu_server.rs`, `src/ascii_server.rs`,
  `src/udp_server.rs` — sync and async wrapper integrations
