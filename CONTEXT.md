# Modbus Protocol Context

This context covers the Modbus application protocol and transport encodings implemented by the `modbus` crate.

## Language

**ADU (Application Data Unit)**:
A complete Modbus frame on the wire: transport-specific header/framing plus a PDU. Examples: RTU ADU, TCP ADU (MBAP), UDP ADU, ASCII ADU.
_Avoid_: frame, packet, message

**PDU (Protocol Data Unit)**:
The function code and its payload, independent of transport framing.
_Avoid_: message, payload

**Function Code**:
The single-byte Modbus operation identifier (e.g., `0x03` Read Holding Registers).
_Avoid_: opcode, command

**Client**:
The initiator of a Modbus request.
_Avoid_: master, host

**Server**:
The responder to Modbus requests.
_Avoid_: slave, device

**Unit ID**:
The address identifier that a client uses to reach a server within a single transport link. In RTU/ASCII this is the server address; in TCP/UDP it is the MBAP unit identifier.
_Avoid_: slave, slave_id, unit_identifier

**Transport**:
The byte-oriented I/O primitive that moves ADU bytes between client and server.
_Avoid_: driver, link, connection
