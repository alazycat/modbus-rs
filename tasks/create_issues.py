#!/usr/bin/env python3
"""Create 48 GitHub issues for modbus-rs in dependency order."""
import subprocess
import sys
import time

REPO = "alazycat/modbus-rs"


def run_gh(title, body):
    cmd = [
        "gh", "issue", "create",
        "--repo", REPO,
        "--title", title,
        "--body", body,
    ]
    result = subprocess.run(cmd, capture_output=True, text=True)
    if result.returncode != 0:
        print(f"FAILED to create issue: {title}")
        print(result.stderr)
        sys.exit(1)
    url = result.stdout.strip()
    number = int(url.split("/")[-1])
    print(f"Created #{number}: {title}")
    return number


def issue_body(what, acceptance, blocked_by=None):
    body = f"""## What to build
{what}

## Acceptance criteria
"""
    for item in acceptance:
        body += f"- [ ] {item}\n"
    body += "\n## Blocked by\n"
    if blocked_by:
        for num in blocked_by:
            body += f"- #{num}\n"
    else:
        body += "None - can start immediately\n"
    return body


# ---------------------------------------------------------------------------
# Phase 1: PDU foundation (1-20)
# ---------------------------------------------------------------------------
pdu_issues = {}

pdu_issues["fc01"] = run_gh(
    "PDU: FC 01 Read Coils",
    issue_body(
        "Implement request/response PDU types for FC 0x01 Read Coils, including encode/decode, quantity validation (1-2000), and LSB-first coil packing.",
        ["ReadCoilsRequest / ReadCoilsResponse structs", "Round-trip encode/decode tests", "Boundary tests for quantity 1 and 2000"],
    ),
)
pdu_issues["fc02"] = run_gh(
    "PDU: FC 02 Read Discrete Inputs",
    issue_body(
        "Implement request/response PDU types for FC 0x02 Read Discrete Inputs, mirroring Read Coils semantics.",
        ["ReadDiscreteInputsRequest / ReadDiscreteInputsResponse structs", "Round-trip encode/decode tests", "Boundary tests"],
    ),
)
pdu_issues["fc05"] = run_gh(
    "PDU: FC 05 Write Single Coil",
    issue_body(
        "Implement request/response PDU types for FC 0x05 Write Single Coil, including ON/OFF value encoding (0x0000 / 0xFF00).",
        ["WriteSingleCoilRequest / WriteSingleCoilResponse structs", "Round-trip tests", "Invalid value rejection"],
    ),
)
pdu_issues["fc0f"] = run_gh(
    "PDU: FC 0F Write Multiple Coils",
    issue_body(
        "Implement request/response PDU types for FC 0x0F Write Multiple Coils, including coil packing and byte count.",
        ["WriteMultipleCoilsRequest / WriteMultipleCoilsResponse structs", "Round-trip tests", "Zero-padded final byte test"],
    ),
)
pdu_issues["fc03"] = run_gh(
    "PDU: FC 03 Read Holding Registers",
    issue_body(
        "Implement request/response PDU types for FC 0x03 Read Holding Registers, big-endian register encoding.",
        ["ReadHoldingRegistersRequest / ReadHoldingRegistersResponse structs", "Round-trip tests", "Quantity boundary 1-125"],
    ),
)
pdu_issues["fc04"] = run_gh(
    "PDU: FC 04 Read Input Registers",
    issue_body(
        "Implement request/response PDU types for FC 0x04 Read Input Registers.",
        ["ReadInputRegistersRequest / ReadInputRegistersResponse structs", "Round-trip tests", "Quantity boundary"],
    ),
)
pdu_issues["fc06"] = run_gh(
    "PDU: FC 06 Write Single Register",
    issue_body(
        "Implement request/response PDU types for FC 0x06 Write Single Register.",
        ["WriteSingleRegisterRequest / WriteSingleRegisterResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc10"] = run_gh(
    "PDU: FC 10 Write Multiple Registers",
    issue_body(
        "Implement request/response PDU types for FC 0x10 Write Multiple Registers.",
        ["WriteMultipleRegistersRequest / WriteMultipleRegistersResponse structs", "Round-trip tests", "Quantity boundary 1-123"],
    ),
)
pdu_issues["fc16"] = run_gh(
    "PDU: FC 16 Mask Write Register",
    issue_body(
        "Implement request/response PDU types for FC 0x16 Mask Write Register (AND mask, OR mask, reference address).",
        ["MaskWriteRegisterRequest / MaskWriteRegisterResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc17"] = run_gh(
    "PDU: FC 17 Read/Write Multiple Registers",
    issue_body(
        "Implement request/response PDU types for FC 0x17 Read/Write Multiple Registers, combining read and write quantities.",
        ["ReadWriteMultipleRegistersRequest / ReadWriteMultipleRegistersResponse structs", "Round-trip tests", "Read/write quantity boundaries"],
    ),
)
pdu_issues["fc18"] = run_gh(
    "PDU: FC 18 Read FIFO Queue",
    issue_body(
        "Implement request/response PDU types for FC 0x18 Read FIFO Queue, including FIFO count and register values.",
        ["ReadFifoQueueRequest / ReadFifoQueueResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc07"] = run_gh(
    "PDU: FC 07 Read Exception Status",
    issue_body(
        "Implement request/response PDU types for FC 0x07 Read Exception Status.",
        ["ReadExceptionStatusRequest / ReadExceptionStatusResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc08"] = run_gh(
    "PDU: FC 08 Diagnostics",
    issue_body(
        "Implement request/response PDU types for FC 0x08 Diagnostics, including common sub-function codes.",
        ["DiagnosticsRequest / DiagnosticsResponse structs", "Sub-function code support", "Round-trip tests"],
    ),
)
pdu_issues["fc0b"] = run_gh(
    "PDU: FC 0B Get Comm Event Counter",
    issue_body(
        "Implement request/response PDU types for FC 0x0B Get Comm Event Counter.",
        ["GetCommEventCounterRequest / GetCommEventCounterResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc0c"] = run_gh(
    "PDU: FC 0C Get Comm Event Log",
    issue_body(
        "Implement request/response PDU types for FC 0x0C Get Comm Event Log, including status, event count, message count, and event bytes.",
        ["GetCommEventLogRequest / GetCommEventLogResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc11"] = run_gh(
    "PDU: FC 11 Report Server ID",
    issue_body(
        "Implement request/response PDU types for FC 0x11 Report Server ID, including server ID byte count and data.",
        ["ReportServerIdRequest / ReportServerIdResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc14"] = run_gh(
    "PDU: FC 14 Read File Record",
    issue_body(
        "Implement request/response PDU types for FC 0x14 Read File Record, including sub-request groups.",
        ["ReadFileRecordRequest / ReadFileRecordResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc15"] = run_gh(
    "PDU: FC 15 Write File Record",
    issue_body(
        "Implement request/response PDU types for FC 0x15 Write File Record, including sub-request groups.",
        ["WriteFileRecordRequest / WriteFileRecordResponse structs", "Round-trip tests"],
    ),
)
pdu_issues["fc2b"] = run_gh(
    "PDU: FC 2B Encapsulated Interface Transport (MEI)",
    issue_body(
        "Implement request/response PDU types for FC 0x2B MEI, including Read Device Identification (0x0E) and CANopen General Reference (0x0D).",
        ["EncapsulatedInterfaceTransportRequest / Response structs", "MEI type 0x0D and 0x0E support", "Round-trip tests"],
    ),
)
pdu_issues["exception"] = run_gh(
    "PDU: Exception responses",
    issue_body(
        "Implement ExceptionCode enum and ExceptionResponse encoding/decoding for standard exception codes 0x01-0x0B, including function code + 0x80.",
        ["ExceptionCode enum", "ExceptionResponse struct", "Encode/decode tests for all codes"],
    ),
)

# Collect PDU issue numbers for blocked-by references
pdu_range = list(pdu_issues.values())

# ---------------------------------------------------------------------------
# Phase 2: Transport framers (21-24)
# ---------------------------------------------------------------------------
framer_issues = {}
framer_issues["rtu"] = run_gh(
    "Framer: RTU frame + CRC-16",
    issue_body(
        "Implement RTU ADU builder/parser with CRC-16 (Modbus polynomial). Include broadcast address handling and known test vectors.",
        ["RtuAdu struct", "CRC-16 computation and verification", "Round-trip and malformed-frame tests"],
    ),
)
framer_issues["ascii"] = run_gh(
    "Framer: ASCII frame + LRC-8",
    issue_body(
        "Implement ASCII ADU builder/parser with colon start, CR/LF delimiter, and LRC-8 checksum. Include hex ASCII encoding/decoding.",
        ["AsciiAdu struct", "LRC-8 computation and verification", "Round-trip tests"],
    ),
)
framer_issues["tcp"] = run_gh(
    "Framer: TCP MBAP header",
    issue_body(
        "Implement MBAP header encode/decode for TCP (transaction ID, protocol ID, length, unit ID).",
        ["TcpAdu struct", "MBAP header encode/decode", "Length field validation"],
    ),
)
framer_issues["udp"] = run_gh(
    "Framer: UDP MBAP header",
    issue_body(
        "Implement MBAP-like header encode/decode for UDP datagrams.",
        ["UdpAdu struct", "UDP MBAP header encode/decode", "Datagram boundary handling"],
    ),
)

# ---------------------------------------------------------------------------
# Phase 3: Sync runtime core (25-26)
# ---------------------------------------------------------------------------
sync_client_core = run_gh(
    "Sync runtime: Transport trait + sync client core",
    issue_body(
        "Define the Transport trait and implement the synchronous Client core (request dispatch, transaction matching, timeout). Demonstrate with an in-memory mock transport.",
        ["Transport trait", "Sync Client struct and request dispatch", "Timeout configuration", "Unit tests with mock transport"],
        blocked_by=[pdu_issues["fc01"], pdu_issues["fc03"], framer_issues["rtu"]],
    ),
)
sync_server_core = run_gh(
    "Sync runtime: DataStore trait + sync server core",
    issue_body(
        "Define the DataStore trait and implement the synchronous Server core (request parsing, dispatch, exception handling). Provide an in-memory MemoryStore.",
        ["DataStore trait", "MemoryStore implementation", "Sync Server dispatcher", "Exception response generation"],
        blocked_by=[pdu_issues["fc01"], pdu_issues["fc03"], pdu_issues["exception"], framer_issues["rtu"]],
    ),
)

# ---------------------------------------------------------------------------
# Phase 4: Sync transport client/server (27-34)
# ---------------------------------------------------------------------------
sync_issues = {}
sync_issues["rtu_client"] = run_gh(
    "Sync RTU client for coils & registers",
    issue_body(
        "Wire the sync Client to the RTU framer so it can read/write coils and holding registers over RTU.",
        ["RTU transport implementation", "Read/write coils and registers over RTU", "Integration tests"],
        blocked_by=[sync_client_core, framer_issues["rtu"]],
    ),
)
sync_issues["rtu_server"] = run_gh(
    "Sync RTU server for coils & registers",
    issue_body(
        "Wire the sync Server to the RTU framer to serve coils and holding registers over RTU.",
        ["RTU server listener", "Address filtering", "Integration tests"],
        blocked_by=[sync_server_core, framer_issues["rtu"]],
    ),
)
sync_issues["tcp_client"] = run_gh(
    "Sync TCP client for coils & registers",
    issue_body(
        "Wire the sync Client to the TCP MBAP framer for coils and holding registers.",
        ["TCP transport implementation", "Transaction ID tracking", "Integration tests"],
        blocked_by=[sync_client_core, framer_issues["tcp"]],
    ),
)
sync_issues["tcp_server"] = run_gh(
    "Sync TCP server for coils & registers",
    issue_body(
        "Wire the sync Server to the TCP MBAP framer to serve coils and holding registers.",
        ["TCP server listener", "Unit ID filtering", "Integration tests"],
        blocked_by=[sync_server_core, framer_issues["tcp"]],
    ),
)
sync_issues["ascii_client"] = run_gh(
    "Sync ASCII client for coils & registers",
    issue_body(
        "Wire the sync Client to the ASCII framer for coils and holding registers.",
        ["ASCII transport implementation", "Read/write coils and registers over ASCII", "Integration tests"],
        blocked_by=[sync_client_core, framer_issues["ascii"]],
    ),
)
sync_issues["ascii_server"] = run_gh(
    "Sync ASCII server for coils & registers",
    issue_body(
        "Wire the sync Server to the ASCII framer to serve coils and holding registers.",
        ["ASCII server listener", "Address filtering", "Integration tests"],
        blocked_by=[sync_server_core, framer_issues["ascii"]],
    ),
)
sync_issues["udp_client"] = run_gh(
    "Sync UDP client for coils & registers",
    issue_body(
        "Wire the sync Client to the UDP framer for coils and holding registers.",
        ["UDP transport implementation", "Datagram-based requests", "Integration tests"],
        blocked_by=[sync_client_core, framer_issues["udp"]],
    ),
)
sync_issues["udp_server"] = run_gh(
    "Sync UDP server for coils & registers",
    issue_body(
        "Wire the sync Server to the UDP framer to serve coils and holding registers.",
        ["UDP server listener", "Unit ID filtering", "Integration tests"],
        blocked_by=[sync_server_core, framer_issues["udp"]],
    ),
)

# ---------------------------------------------------------------------------
# Phase 5: Async runtime core (35-36)
# ---------------------------------------------------------------------------
async_client_core = run_gh(
    "Async runtime: AsyncTransport trait + async client core",
    issue_body(
        "Define the AsyncTransport trait and implement the asynchronous Client core using tokio. Demonstrate with an in-memory mock transport.",
        ["AsyncTransport trait", "Async Client struct and request dispatch", "Timeout and cancellation", "Unit tests with mock transport"],
        blocked_by=[sync_client_core],
    ),
)
async_server_core = run_gh(
    "Async runtime: Async server core",
    issue_body(
        "Implement the asynchronous Server core using tokio, reusing the DataStore trait.",
        ["Async Server dispatcher", "tokio task per connection", "Exception response generation", "Unit tests"],
        blocked_by=[sync_server_core],
    ),
)

# ---------------------------------------------------------------------------
# Phase 6: Async transport client/server (37-40)
# ---------------------------------------------------------------------------
async_issues = {}
async_issues["tcp"] = run_gh(
    "Async TCP client/server for coils & registers",
    issue_body(
        "Implement async TCP client and server for coils and holding registers using tokio::net.",
        ["Async TCP transport", "Async TCP client/server", "Integration tests"],
        blocked_by=[async_client_core, async_server_core, framer_issues["tcp"], sync_issues["tcp_client"], sync_issues["tcp_server"]],
    ),
)
async_issues["rtu"] = run_gh(
    "Async RTU client/server for coils & registers",
    issue_body(
        "Implement async RTU client and server, with optional tokio-serial support and a byte-stream trait fallback.",
        ["Async RTU transport", "tokio-serial wrapper", "Async RTU client/server", "Integration tests"],
        blocked_by=[async_client_core, async_server_core, framer_issues["rtu"], sync_issues["rtu_client"], sync_issues["rtu_server"]],
    ),
)
async_issues["ascii"] = run_gh(
    "Async ASCII client/server for coils & registers",
    issue_body(
        "Implement async ASCII client and server with LRC framing.",
        ["Async ASCII transport", "Async ASCII client/server", "Integration tests"],
        blocked_by=[async_client_core, async_server_core, framer_issues["ascii"], sync_issues["ascii_client"], sync_issues["ascii_server"]],
    ),
)
async_issues["udp"] = run_gh(
    "Async UDP client/server for coils & registers",
    issue_body(
        "Implement async UDP client and server using tokio::net::UdpSocket.",
        ["Async UDP transport", "Async UDP client/server", "Integration tests"],
        blocked_by=[async_client_core, async_server_core, framer_issues["udp"], sync_issues["udp_client"], sync_issues["udp_server"]],
    ),
)

# ---------------------------------------------------------------------------
# Phase 7: Advanced function code exposure (41-44)
# ---------------------------------------------------------------------------
all_sync_async = list(sync_issues.values()) + list(async_issues.values())
run_gh(
    "Expose bit-access function codes in client/server",
    issue_body(
        "Ensure FC 01/02/05/0F work end-to-end through sync and async clients/servers across all transports.",
        ["Client methods for bit-access FCs", "Server dispatch for bit-access FCs", "Cross-transport integration tests"],
        blocked_by=all_sync_async + [pdu_issues["fc01"], pdu_issues["fc02"], pdu_issues["fc05"], pdu_issues["fc0f"]],
    ),
)
run_gh(
    "Expose register-access function codes in client/server",
    issue_body(
        "Ensure FC 03/04/06/10 work end-to-end through sync and async clients/servers across all transports.",
        ["Client methods for register-access FCs", "Server dispatch for register-access FCs", "Cross-transport integration tests"],
        blocked_by=all_sync_async + [pdu_issues["fc03"], pdu_issues["fc04"], pdu_issues["fc06"], pdu_issues["fc10"]],
    ),
)
run_gh(
    "Expose advanced/diagnostic function codes in client/server",
    issue_body(
        "Ensure FC 07/08/0B/0C/11/16/17/18 work end-to-end through sync and async clients/servers.",
        ["Client methods for advanced/diagnostic FCs", "Server dispatch", "Integration tests"],
        blocked_by=all_sync_async + [pdu_issues["fc07"], pdu_issues["fc08"], pdu_issues["fc0b"], pdu_issues["fc0c"], pdu_issues["fc11"], pdu_issues["fc16"], pdu_issues["fc17"], pdu_issues["fc18"]],
    ),
)
run_gh(
    "Expose file record and MEI function codes in client/server",
    issue_body(
        "Ensure FC 14/15/2B work end-to-end through sync and async clients/servers.",
        ["Client methods for file record and MEI FCs", "Server dispatch", "Integration tests"],
        blocked_by=all_sync_async + [pdu_issues["fc14"], pdu_issues["fc15"], pdu_issues["fc2b"]],
    ),
)

# ---------------------------------------------------------------------------
# Phase 8: Helpers, CLI, tests (45-48)
# ---------------------------------------------------------------------------
run_gh(
    "Data-type helpers",
    issue_body(
        "Implement register-to-primitive conversions for u16, u32, u64, i16, i32, i64, f32, f64, and strings, with configurable endianness and word order.",
        ["Endian and WordOrder enums", "Conversion helpers for all types", "Unit tests"],
        blocked_by=[pdu_issues["fc03"]],
    ),
)
run_gh(
    "CLI tool",
    issue_body(
        "Build a minimal command-line binary (modbus-cli) with client/server subcommands for TCP and RTU, using clap and tracing.",
        ["CLI argument parsing", "Client subcommand", "Server subcommand", "README usage example"],
        blocked_by=[async_issues["tcp"], async_issues["rtu"]],
    ),
)
run_gh(
    "Integration test harness",
    issue_body(
        "Create end-to-end integration tests using in-memory mock transports and real TCP sockets, covering client/server pairs for all transports.",
        ["Mock transport utilities", "TCP integration tests", "RTU/ASCII/UDP integration tests"],
        blocked_by=all_sync_async,
    ),
)
run_gh(
    "Loopback serial integration tests",
    issue_body(
        "Add optional loopback serial integration tests for RTU and ASCII, gated by the loopback-tests feature and marked #[ignore] by default.",
        ["loopback-tests Cargo feature", "RTU loopback test", "ASCII loopback test", "CI documentation"],
        blocked_by=[async_issues["rtu"], async_issues["ascii"]],
    ),
)

print("\nAll issues created successfully.")
