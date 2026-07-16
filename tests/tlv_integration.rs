//! Validate tlv-rs against hand-rolled Modbus PDU codec.
//!
//! Proves tlv-rs can produce and consume the same wire format.

use tlv_rs::{Encoder, Decoder, Tag1};

/// Verify that tlv-rs can encode a flat binary PDU
/// matching the Modbus spec wire format.
#[test]
fn tlv_encode_read_coils_request_flat() {
    // Modbus ReadCoilsRequest wire format (flat, no TLV):
    // [function_code(1)] [starting_address(2 BE)] [quantity(2 BE)]
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.write_raw(&[0x01]).unwrap();                     // function code
    enc.write_raw(&100u16.to_be_bytes()).unwrap();       // starting_address
    enc.write_raw(&10u16.to_be_bytes()).unwrap();        // quantity
    assert_eq!(enc.len(), 5);
    assert_eq!(&buf[..5], &[0x01, 0x00, 0x64, 0x00, 0x0A]);
}

/// Round-trip a Modbus PDU using tlv-rs raw I/O.
#[test]
fn tlv_raw_roundtrip_modbus_pdu() {
    // Encode with tlv-rs raw writes (flat format, no tag/length wrapper)
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);
    enc.write_raw(&[0x01]).unwrap();                     // function_code
    enc.write_raw(&100u16.to_be_bytes()).unwrap();       // starting_address = 100
    enc.write_raw(&10u16.to_be_bytes()).unwrap();        // quantity = 10

    let wire = enc.written();
    assert_eq!(wire.len(), 5);
    assert_eq!(wire, &[0x01, 0x00, 0x64, 0x00, 0x0A]);

    // Decode by reading raw bytes
    let mut dec = Decoder::new(wire);
    assert_eq!(dec.remaining(), 5);

    // Read function code
    let fc = dec.read_value_u8(1).unwrap();
    assert_eq!(fc, 0x01);

    // Read address
    let addr = dec.read_value_u16(2).unwrap();
    assert_eq!(addr, 100);

    // Read quantity
    let qty = dec.read_value_u16(2).unwrap();
    assert_eq!(qty, 10);

    assert!(dec.is_empty());
}

/// Validate tlv-rs TLV encoding against Modbus-like framing.
#[test]
fn tlv_encode_decode_modbus_style_tlv() {
    // Use TLV encoding where function code = tag, value = the params
    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);

    // Write tag (function code 0x01) + value (4 bytes: addr + qty)
    let tw = enc.write_tag(Tag1(0x01)).unwrap();
    tw.write_value(&[0x00, 0x64, 0x00, 0x0A]).unwrap();

    // Wire format: [tag=0x01] [len=0x04] [addr_hi addr_lo qty_hi qty_lo]
    let wire = enc.written();
    assert_eq!(wire[0], 0x01); // tag
    assert_eq!(wire[1], 0x04); // length (4 bytes of value)
    assert_eq!(&wire[2..6], &[0x00, 0x64, 0x00, 0x0A]);

    // Decode back
    let mut dec = Decoder::new(wire);
    let (tag, len) = dec.read_tag::<Tag1>().unwrap();
    assert_eq!(tag, Tag1(0x01));
    assert_eq!(len, 4);
    let value = dec.read_value_bytes(len).unwrap();
    assert_eq!(value, &[0x00, 0x64, 0x00, 0x0A]);

    // Parse value fields
    let addr = u16::from_be_bytes([value[0], value[1]]);
    let qty = u16::from_be_bytes([value[2], value[3]]);
    assert_eq!(addr, 100);
    assert_eq!(qty, 10);
}

/// Validate nested TLV for a Modbus response (function code + byte count + data).
#[test]
fn tlv_encode_decode_modbus_response() {
    // Modbus ReadCoilsResponse: [fc=0x01] [byte_count=N] [N bytes of coil data]
    let coil_data = &[0xCD, 0x6B, 0x05]; // from Modbus spec example

    let mut buf = [0u8; 64];
    let mut enc = Encoder::new(&mut buf);

    // Use TLV: tag = function code, value = [byte_count + data]
    let tw = enc.write_tag(Tag1(0x01)).unwrap();
    let mut val = vec![coil_data.len() as u8];
    val.extend_from_slice(coil_data);
    tw.write_value(&val).unwrap();

    let wire = enc.written();
    // [0x01] [0x04] [0x03, 0xCD, 0x6B, 0x05]
    assert_eq!(wire[0], 0x01); // tag (function code)
    assert_eq!(wire[1], 0x04); // length (1 + 3 = 4)
    assert_eq!(wire[2], 0x03); // byte count
    assert_eq!(&wire[3..6], coil_data);

    // Decode
    let mut dec = Decoder::new(wire);
    let bytes = dec.read_tlv_bytes(Tag1(0x01)).unwrap();
    assert_eq!(bytes.len(), 4);
    let byte_count = bytes[0] as usize;
    assert_eq!(byte_count, 3);
    let data = &bytes[1..];
    assert_eq!(data, coil_data);
}
