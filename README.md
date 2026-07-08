# modbus-rs

A Rust implementation of the Modbus Application Protocol with optional
client/server runtimes for TCP and RTU serial.

## Building

```sh
cargo build
cargo build --features cli
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
