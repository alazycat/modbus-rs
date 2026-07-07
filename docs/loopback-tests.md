# Serial Loopback Tests

The `loopback-tests` Cargo feature enables optional integration tests that
exercise real serial ports. These tests are marked `#[ignore]` by default
because they require two cross-connected serial ports (a physical null-modem
cable or a virtual port pair).

## Requirements

- Two serial ports wired so that the TX of each is connected to the RX of the
  other.
- A Unix or Windows system with Rust and `cargo` installed.

## Quick Start

Create a virtual port pair, set the environment variable, and run the ignored
serial tests:

### Linux / macOS

```sh
socat -d -d pty,raw,echo=0,link=/tmp/ttyS0 pty,raw,echo=0,link=/tmp/ttyS1 &
export MODBUS_LOOPBACK_PORTS=/tmp/ttyS0,/tmp/ttyS1
cargo test --features loopback-tests -- --ignored
```

### Windows

Install [com0com](https://com0com.sourceforge.net/) to create a paired port
such as `COM3` ↔ `COM4`, then run:

```powershell
$env:MODBUS_LOOPBACK_PORTS = "COM3,COM4"
cargo test --features loopback-tests -- --ignored
```

## Environment Variable

`MODBUS_LOOPBACK_PORTS` must contain exactly two comma-separated port names.
The first port is opened by the server side and the second by the client side.

## CI Configuration

### GitHub Actions (Ubuntu)

```yaml
- name: Install socat
  run: sudo apt-get update && sudo apt-get install -y socat

- name: Create virtual serial loopback pair
  run: |
    socat -d -d pty,raw,echo=0,link=/tmp/ttyS0 pty,raw,echo=0,link=/tmp/ttyS1 &
    sleep 1
    echo "MODBUS_LOOPBACK_PORTS=/tmp/ttyS0,/tmp/ttyS1" >> "$GITHUB_ENV"

- name: Run loopback serial tests
  run: cargo test --features loopback-tests -- --ignored
```

### GitHub Actions (Windows)

Install `com0com` as part of the runner image or a setup step, then run:

```yaml
- name: Run loopback serial tests
  env:
    MODBUS_LOOPBACK_PORTS: COM3,COM4
  run: cargo test --features loopback-tests -- --ignored
```

## Notes

- Both RTU and ASCII tests open ports at 9600 baud with 8 data bits, no parity,
  and 1 stop bit.
- The tests are ignored unless `-- --ignored` is passed, so they do not run
  during the default `cargo test` invocation.
