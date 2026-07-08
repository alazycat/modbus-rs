# modbus-rs

A Rust implementation of the Modbus Application Protocol with optional
client/server runtimes for TCP, UDP, RTU serial, and ASCII.

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
| `helpers` | Byte/word conversion utilities |
| `cli` | `modbus-cli` binary |

## Building

```sh
cargo build
cargo build --features cli
cargo build --features sync,tcp,tls
cargo build --features sync,rtu,sync-serial
```

## CLI

`modbus-cli` is an optional binary built with the `cli` feature. It provides
`client` and `server` subcommands for both TCP and RTU serial transports.

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

TLS and sync serial transports are also available via the `tls` and
`sync-serial` feature flags (see the feature table above and the crate docs).

### RTU serial client/server example

RTU mode requires an actual serial port. Replace `/dev/ttyUSB0` with the
appropriate device path for your system.

Server:

```sh
cargo run --bin modbus-cli --features cli -- server rtu \
  --path /dev/ttyUSB0 --baud 9600 --slave-id 1 \
  --coils 16 --holding-registers 10
```

Client:

```sh
cargo run --bin modbus-cli --features cli -- client rtu \
  --path /dev/ttyUSB0 --baud 9600 --slave-id 1 \
  read-holding-registers --address 0 --quantity 4
```

## Typed register helpers

The `helpers` feature adds convenience methods for reading and writing
multi-register values. The byte order and word order are controlled by
[`ClientConfig`], defaulting to big-endian / most-significant-first:

```rust
use modbus::client::{ClientConfig, Client};
use modbus::helpers::{Endian, WordOrder};

let mut config = ClientConfig::default();
config.endian = Endian::Big;
config.word_order = WordOrder::MostSignificantFirst;

// client is any Client or ClientCore, e.g. a TCP client
let mut client = modbus::tcp_client::TcpClient::connect(
    "127.0.0.1:5502",
    config,
).unwrap();

let value = client.read_holding_registers_u32(1, 0).unwrap();
client.write_multiple_registers_u32(1, 0, value + 1).unwrap();
```

Supported types include `u16`, `i16`, `u32`, `i32`, `u64`, `i64`, `f32`,
`f64`, and NUL-terminated strings.

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
