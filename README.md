# modbus-rs

A Rust implementation of the Modbus Application Protocol with optional
client/server runtimes for TCP, UDP, RTU serial, ASCII, RTU-over-TCP, and TLS.

## Features

| Feature | Enables |
|---|---|
| `tcp` | TCP transport and client/server |
| `udp` | UDP transport and client/server |
| `rtu` | RTU framing and client/server |
| `ascii` | ASCII framing and client/server |
| `sync` | Synchronous runtime |
| `async` | `tokio`-based asynchronous runtime |
| `serial` | Async serial support via `tokio-serial` |
| `sync-serial` | Sync serial support via `serialport` |
| `tls` | TLS over TCP via `rustls` / `tokio-rustls` |
| `helpers` | Byte/word conversion utilities and typed client methods |
| `config` | JSON/TOML/YAML configuration file loading |
| `tracing` | Optional `tracing` instrumentation |
| `metrics` | Optional atomic request/retry/exception counters |
| `cli` | `modbus-cli` binary |

## Building

```sh
cargo build
cargo build --features cli
cargo build --features sync,tcp,tls
cargo build --features sync,rtu,sync-serial
cargo build --features async,tcp,helpers
```

## Examples

See the [`examples/`](examples/) directory for minimal compiling examples:

```sh
cargo build --example tcp_client --features sync,tcp
cargo build --example tcp_server --features async,tcp
cargo build --example async_tcp_client --features async,tcp
cargo build --example udp_client --features sync,udp
cargo build --example udp_server --features async,udp
cargo build --example rtu_client --features sync,rtu,sync-serial
cargo build --example ascii_client --features sync,ascii,sync-serial
cargo build --example rtu_over_tcp_client --features sync,rtu,tcp
cargo build --example rtu_over_tcp_server --features async,rtu,tcp
cargo build --example tls_client --features async,tcp,tls
cargo build --example tls_server --features async,tcp,tls
cargo build --example typed_helpers --features sync,tcp,helpers
cargo build --example custom_data_store --features sync,tcp,config
cargo build --example logging_hook --features sync,tcp
cargo build --example config_client --features config,sync,tcp
cargo build --example config_server --features config,async,tcp
```

## CLI

`modbus-cli` is an optional binary built with the `cli` feature. It provides
`client` and `server` subcommands. Additional transports and helpers are
available when the corresponding features are enabled:

- `cli,tls` — TLS over TCP client/server.
- `cli,udp` — UDP client/server.
- `cli,ascii` — ASCII serial/stream client/server.
- `cli,helpers` — typed `read-holding-u32`, `write-multiple-f32`, etc.
- `cli,config` — load client/server settings from JSON/TOML/YAML.

### TCP client/server example

Start a server in one terminal:

```sh
cargo run --bin modbus-cli --features cli -- server tcp \
  --bind 127.0.0.1:5502 --unit-id 1 --coils 16 --holding-registers 10
```

From another terminal, write a holding register and read it back:

```sh
# Write 0x1234 to register 0
cargo run --bin modbus-cli --features cli -- client tcp \
  --host 127.0.0.1 --port 5502 --unit-id 1 \
  write-register --address 0 --value 0x1234

# Read it back
cargo run --bin modbus-cli --features cli -- client tcp \
  --host 127.0.0.1 --port 5502 --unit-id 1 \
  read-holding-registers --address 0 --quantity 1
```

Read coils:

```sh
cargo run --bin modbus-cli --features cli -- client tcp \
  --host 127.0.0.1 --port 5502 --unit-id 1 \
  read-coils --address 0 --quantity 8
```

### TLS TCP server example

```sh
# Generate or obtain PEM files first, then:
cargo run --bin modbus-cli --features cli,tls -- server tcp \
  --bind 127.0.0.1:5502 --unit-id 1 --holding-registers 10 \
  --tls --tls-cert server.crt --tls-key server.key
```

### Typed register helpers via CLI

```sh
cargo run --bin modbus-cli --features cli,helpers -- client tcp \
  --host 127.0.0.1 --port 5502 --unit-id 1 \
  read-holding-u32 --address 0
```

### Configuration file example

```sh
cargo run --bin modbus-cli --features cli,config -- server --config server.toml
```

## Typed register helpers

The `helpers` feature adds convenience methods for reading and writing
multi-register values. Byte order and word order are controlled by
[`ClientConfig`], defaulting to big-endian / most-significant-first:

```rust
use modbus::client::ClientConfig;
use modbus::helpers::{Endian, WordOrder};
use modbus::tcp_client::TcpClient;
use modbus::tcp_transport::TcpTransport;
use std::net::TcpStream;

let mut config = ClientConfig::default();
config.endian = Endian::Big;
config.word_order = WordOrder::MostSignificantFirst;

let stream = TcpStream::connect("127.0.0.1:5502").unwrap();
let mut client = TcpClient::with_config(TcpTransport::new(stream), config);

let value = client.read_holding_registers_u32(1, 0).unwrap();
client.write_multiple_registers_u32(1, 0, value + 1).unwrap();
```

Supported types include `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`,
`f64`, and NUL-terminated strings.

## Retry, reconnect, and idle timeout

`RetryAdapter` / `AsyncRetryAdapter` wrap any client adapter and rebuild the
connection using a factory when a retryable error occurs. Idle timeout is
configured through `ClientConfig.idle_timeout` and applied to the underlying
streams by all sync and async transports.

## Tests

```sh
cargo test --all-features
cargo clippy --all-features -- -D warnings
```

## Documentation

- [`docs/comparison-with-modbus-go.md`](docs/comparison-with-modbus-go.md) —
  comparison with the Go `modbus-go` implementation and the project roadmap.

## License

MIT
