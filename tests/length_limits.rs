//! Length indicators and pointers at their limits, ITU-T Q.713 (03/2001).
//!
//! * 2.2: the length indicator "is binary coded to indicate the number of
//!   octets in the parameter content field".
//! * 1.4: "Each pointer is encoded as a single octet or two octets in the case
//!   of LUDT and LUDTS."
//! * 3.16: the data parameter contains "less than or equal to 255 octets".
//! * 3.20: the long data parameter "has a two-octet length indicator field".
//!
//! So a one-octet length indicator or pointer that would have to carry more
//! than 255 is not encodable, and `encode` must say so. Writing the low eight
//! bits gives a message that decodes to something else.
//!
//! The vectors are derived by hand from Q.713 and are not produced by this
//! crate. The same octets are then given to Wireshark's SCCP dissector, an
//! independent decoder, which must read the pointers, the addresses and the
//! whole of the data back.
//!
//! `tshark` and `text2pcap` are optional: without them the Wireshark checks
//! print a `SKIP` line and the byte vectors still run. Set
//! `SCCP_REQUIRE_TSHARK=1` to make a missing tool a failure.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use sccp::{
    ExtendedUnitData, ExtendedUnitDataService, GlobalTitle, LongUnitData, LongUnitDataService,
    ReturnCause, SccpAddress, SccpError, SccpMessage, SubsystemNumber, UnitData, UnitDataService,
};

// ── Building blocks ─────────────────────────────────────────────────────────

/// Subsystem numbers Wireshark has no upper-layer dissector for, so that it
/// shows the data as plain octets.
const CALLED_SSN: u8 = 0xC8;
const CALLING_SSN: u8 = 0xC9;

/// A two-octet address: address indicator `0x42` (route on SSN, SSN present)
/// and the SSN.
fn ssn_address(ssn: u8) -> SccpAddress {
    SccpAddress::with_ssn(SubsystemNumber::from_u8(ssn), None)
}

/// An address of exactly `octets` octets (at least 2): address indicator
/// `0x08` (global title indicator 0010, route on GT), translation type 0, and
/// `2 * (octets - 2)` digits packed two to an octet.
fn address_of(octets: usize) -> SccpAddress {
    let gt = GlobalTitle::Gt0010 {
        translation_type: 0,
        digits: "1".repeat(2 * (octets - 2)),
    };
    SccpAddress::with_gt(gt, None)
}

/// The octets [`address_of`] stands for, written out by hand.
fn address_octets(octets: usize) -> Vec<u8> {
    let mut out = vec![0x08, 0x00];
    out.resize(octets, 0x11);
    out
}

/// `count` octets of data that differ from one another, so that a decoder
/// that drops, repeats or reorders any of them is caught.
fn data(count: usize) -> Vec<u8> {
    (0..count).map(|i| (i % 251) as u8).collect()
}

fn hex_of(bytes: &[u8]) -> String {
    hex::encode(bytes)
}

#[track_caller]
fn assert_too_long<T: std::fmt::Debug>(
    result: Result<T, SccpError>,
    field: &str,
    value: usize,
    maximum: usize,
) {
    match result {
        Err(SccpError::TooLong {
            field: f,
            value: v,
            maximum: m,
        }) => assert_eq!((f, v, m), (field, value, maximum)),
        other => panic!("expected TooLong for {field}, got {other:?}"),
    }
}

// ── UDT / UDTS: three one-octet pointers, one-octet lengths ─────────────────

/// UDT with two-octet addresses and 255 octets of data, by hand:
///
/// ```text
/// offset 0   09          message type UDT
///        1   00          protocol class 0
///        2   03          pointer to called:  2 + 3 = 5
///        3   05          pointer to calling: 3 + 5 = 8
///        4   07          pointer to data:    4 + 7 = 11
///        5   02 42 c8    called:  length 2, AI 0x42, SSN
///        8   02 42 c9    calling: length 2, AI 0x42, SSN
///       11   ff          data length 255
///       12   ...         255 octets
/// ```
fn udt_255_by_hand() -> Vec<u8> {
    let mut expected = vec![
        0x09, 0x00, 0x03, 0x05, 0x07, 0x02, 0x42, 0xC8, 0x02, 0x42, 0xC9, 0xFF,
    ];
    expected.extend_from_slice(&data(255));
    expected
}

#[test]
fn udt_with_255_octets_of_data_matches_the_hand_derived_vector() {
    let udt = UnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(255));
    let expected = udt_255_by_hand();
    assert_eq!(expected.len(), 12 + 255);
    assert_eq!(hex_of(&udt.encode().unwrap()), hex_of(&expected));
    assert_eq!(UnitData::decode(&expected).unwrap(), udt);
}

#[test]
fn udt_with_256_octets_of_data_is_an_error() {
    let udt = UnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(256));
    assert_too_long(udt.encode(), "data", 256, 255);
    assert_too_long(SccpMessage::Udt(udt).encode(), "data", 256, 255);
}

#[test]
fn udts_data_has_the_same_limit() {
    let build = |count| {
        UnitDataService::new(
            ReturnCause::SubsystemFailure,
            ssn_address(CALLED_SSN),
            ssn_address(CALLING_SSN),
            data(count),
        )
    };
    let fits = build(255);
    let encoded = fits.encode().unwrap();
    // Same layout as the UDT vector, with the return cause in octet 1.
    let mut expected = udt_255_by_hand();
    expected[0] = 0x0A;
    expected[1] = 0x03;
    assert_eq!(hex_of(&encoded), hex_of(&expected));
    assert_eq!(UnitDataService::decode(&expected).unwrap(), fits);

    assert_too_long(build(256).encode(), "data", 256, 255);
    // The length that used to wrap to 0x2C.
    assert_too_long(build(300).encode(), "data", 300, 255);
}

/// The third pointer of a UDT is `3 + called + calling` (Q.713 2.3: the octets
/// from the pointer itself up to the parameter). With two 126-octet addresses
/// it is 255, the largest a single octet holds:
///
/// ```text
/// offset 0   09 00
///        2   03          pointer to called:  2 + 3   = 5
///        3   81          pointer to calling: 3 + 129 = 132
///        4   ff          pointer to data:    4 + 255 = 259
///        5   7e ...      called:  length 126, AI 0x08, TT 0, 124 octets 0x11
///      132   7e ...      calling: the same
///      259   02 62 40    data
/// ```
#[test]
fn udt_pointer_to_data_of_255_matches_the_hand_derived_vector() {
    let udt = UnitData::new(address_of(126), address_of(126), vec![0x62, 0x40]);
    let mut expected = vec![0x09, 0x00, 0x03, 0x81, 0xFF, 0x7E];
    expected.extend_from_slice(&address_octets(126));
    expected.push(0x7E);
    expected.extend_from_slice(&address_octets(126));
    expected.extend_from_slice(&[0x02, 0x62, 0x40]);
    assert_eq!(expected.len(), 262);
    assert_eq!(expected[132], 0x7E);
    assert_eq!(expected[259], 0x02);

    assert_eq!(hex_of(&udt.encode().unwrap()), hex_of(&expected));
    assert_eq!(UnitData::decode(&expected).unwrap(), udt);
}

#[test]
fn udt_pointer_past_255_is_an_error() {
    // One octet more in either address: the pointer to data would be 256.
    let udt = UnitData::new(address_of(127), address_of(126), vec![0x62, 0x40]);
    assert_too_long(udt.encode(), "pointer to data", 256, 255);
    let udt = UnitData::new(address_of(126), address_of(127), vec![0x62, 0x40]);
    assert_too_long(udt.encode(), "pointer to data", 256, 255);

    // A called address of 253 octets moves the calling address out of reach
    // of the second pointer: 3 + 253 = 256.
    let udt = UnitData::new(address_of(253), ssn_address(CALLING_SSN), vec![0x62, 0x40]);
    assert_too_long(udt.encode(), "pointer to calling party address", 256, 255);
}

#[test]
fn an_address_longer_than_255_octets_is_an_error_in_every_message() {
    let long = || address_of(256);
    let short = || ssn_address(CALLING_SSN);
    let body = || vec![0x62, 0x40];
    let cause = ReturnCause::SubsystemFailure;

    assert_too_long(
        UnitData::new(long(), short(), body()).encode(),
        "called party address",
        256,
        255,
    );
    assert_too_long(
        UnitData::new(short(), long(), body()).encode(),
        "calling party address",
        256,
        255,
    );
    assert_too_long(
        UnitDataService::new(cause, long(), short(), body()).encode(),
        "called party address",
        256,
        255,
    );
    assert_too_long(
        ExtendedUnitData::new(long(), short(), body()).encode(),
        "called party address",
        256,
        255,
    );
    assert_too_long(
        ExtendedUnitDataService::new(cause, short(), long(), body()).encode(),
        "calling party address",
        256,
        255,
    );
    // LUDT and LUDTS have two-octet pointers, but the address length
    // indicator is still one octet.
    assert_too_long(
        LongUnitData::new(long(), short(), body()).encode(),
        "called party address",
        256,
        255,
    );
    assert_too_long(
        LongUnitDataService::new(cause, short(), long(), body()).encode(),
        "calling party address",
        256,
        255,
    );
}

// ── XUDT / XUDTS: four one-octet pointers ───────────────────────────────────

/// XUDT with two-octet addresses and 255 octets of data, by hand:
///
/// ```text
/// offset 0   11 00 0f    XUDT, class 0, hop counter 15
///        3   04          pointer to called:  3 + 4 = 7
///        4   06          pointer to calling: 4 + 6 = 10
///        5   08          pointer to data:    5 + 8 = 13
///        6   00          no optional part
///        7   02 42 c8    called
///       10   02 42 c9    calling
///       13   ff          data length 255
///       14   ...         255 octets
/// ```
fn xudt_255_by_hand() -> Vec<u8> {
    let mut expected = vec![
        0x11, 0x00, 0x0F, 0x04, 0x06, 0x08, 0x00, 0x02, 0x42, 0xC8, 0x02, 0x42, 0xC9, 0xFF,
    ];
    expected.extend_from_slice(&data(255));
    expected
}

#[test]
fn xudt_with_255_octets_of_data_matches_the_hand_derived_vector() {
    let xudt = ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(255));
    let expected = xudt_255_by_hand();
    assert_eq!(expected.len(), 14 + 255);
    assert_eq!(hex_of(&xudt.encode().unwrap()), hex_of(&expected));
    assert_eq!(ExtendedUnitData::decode(&expected).unwrap(), xudt);
}

#[test]
fn xudt_and_xudts_with_256_octets_of_data_are_errors() {
    let xudt = ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(256));
    assert_too_long(xudt.encode(), "data", 256, 255);

    let xudts = ExtendedUnitDataService::new(
        ReturnCause::HopCounterViolation,
        ssn_address(CALLED_SSN),
        ssn_address(CALLING_SSN),
        data(256),
    );
    assert_too_long(xudts.encode(), "data", 256, 255);
}

/// The fourth pointer of an XUDT is `4 + called + calling + data`. With
/// two-octet addresses and an optional part, 247 octets of data put it at 255:
///
/// ```text
/// offset 6   ff          pointer to optional: 6 + 255 = 261
///       13   f7          data length 247
///       14   ...         247 octets
///      261   12 01 03 00 importance 3, end of optional parameters
/// ```
#[test]
fn xudt_pointer_to_the_optional_part_of_255_matches_the_hand_derived_vector() {
    let importance = vec![0x12, 0x01, 0x03, 0x00];
    let mut xudt =
        ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(247));
    xudt.optional_part = importance.clone();

    let mut expected = vec![
        0x11, 0x00, 0x0F, 0x04, 0x06, 0x08, 0xFF, 0x02, 0x42, 0xC8, 0x02, 0x42, 0xC9, 0xF7,
    ];
    expected.extend_from_slice(&data(247));
    assert_eq!(expected.len(), 261);
    expected.extend_from_slice(&importance);

    assert_eq!(hex_of(&xudt.encode().unwrap()), hex_of(&expected));
    assert_eq!(ExtendedUnitData::decode(&expected).unwrap(), xudt);
}

#[test]
fn xudt_pointer_to_the_optional_part_past_255_is_an_error() {
    let mut xudt =
        ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(248));
    xudt.optional_part = vec![0x12, 0x01, 0x03, 0x00];
    assert_too_long(xudt.encode(), "pointer to optional part", 256, 255);

    // Without an optional part the pointer is 0 and the same data fits.
    xudt.optional_part.clear();
    let encoded = xudt.encode().unwrap();
    assert_eq!(encoded[6], 0x00);
    assert_eq!(ExtendedUnitData::decode(&encoded).unwrap(), xudt);

    let mut xudts = ExtendedUnitDataService::new(
        ReturnCause::HopCounterViolation,
        ssn_address(CALLED_SSN),
        ssn_address(CALLING_SSN),
        data(248),
    );
    xudts.optional_part = vec![0x12, 0x01, 0x03, 0x00];
    assert_too_long(xudts.encode(), "pointer to optional part", 256, 255);
}

// ── LUDT / LUDTS: two-octet pointers, two-octet data length ─────────────────

/// LUDT with a 255-octet called address and 256 octets of data, by hand
/// (two-octet fields are sent low octet first, Q.713 1.4):
///
/// ```text
/// offset 0   13 00 0f    LUDT, class 0, hop counter 15
///        3   07 00       pointer to called:  4 + 7    = 11
///        5   05 01       pointer to calling: 6 + 261  = 267
///        7   06 01       pointer to data:    8 + 262  = 270
///        9   00 00       no optional part
///       11   ff ...      called: length 255, AI 0x08, TT 0, 253 octets 0x11
///      267   02 42 c9    calling
///      270   00 01       long data length 256
///      272   ...         256 octets
/// ```
#[test]
fn ludt_carries_a_255_octet_address_and_256_octets_of_data() {
    let ludt = LongUnitData::new(address_of(255), ssn_address(CALLING_SSN), data(256));
    let mut expected = vec![
        0x13, 0x00, 0x0F, 0x07, 0x00, 0x05, 0x01, 0x06, 0x01, 0x00, 0x00, 0xFF,
    ];
    expected.extend_from_slice(&address_octets(255));
    assert_eq!(expected.len(), 267);
    expected.extend_from_slice(&[0x02, 0x42, 0xC9, 0x00, 0x01]);
    expected.extend_from_slice(&data(256));

    assert_eq!(hex_of(&ludt.encode().unwrap()), hex_of(&expected));
    assert_eq!(LongUnitData::decode(&expected).unwrap(), ludt);
}

#[test]
fn ludt_data_past_the_two_octet_length_is_an_error() {
    let build = |count| {
        LongUnitData::new(
            ssn_address(CALLED_SSN),
            ssn_address(CALLING_SSN),
            vec![0xAB; count],
        )
    };
    let fits = build(65_535);
    let encoded = fits.encode().unwrap();
    // Offset 17: the long data length indicator, after 11 + 3 + 3 octets.
    assert_eq!(encoded[17..19], [0xFF, 0xFF]);
    assert_eq!(LongUnitData::decode(&encoded).unwrap(), fits);

    assert_too_long(build(65_536).encode(), "long data", 65_536, 65_535);

    let ludts = LongUnitDataService::new(
        ReturnCause::SubsystemFailure,
        ssn_address(CALLED_SSN),
        ssn_address(CALLING_SSN),
        vec![0xAB; 65_536],
    );
    assert_too_long(ludts.encode(), "long data", 65_536, 65_535);
}

#[test]
fn ludt_pointer_to_the_optional_part_past_65535_is_an_error() {
    // The pointer is 5 + called + calling + data: 5 + 2 + 2 + 65 526 = 65 535.
    let mut ludt = LongUnitData::new(
        ssn_address(CALLED_SSN),
        ssn_address(CALLING_SSN),
        vec![0xAB; 65_526],
    );
    ludt.optional_part = vec![0x12, 0x01, 0x03, 0x00];
    let encoded = ludt.encode().unwrap();
    assert_eq!(encoded[9..11], [0xFF, 0xFF]);
    assert_eq!(LongUnitData::decode(&encoded).unwrap(), ludt);

    ludt.data.push(0xAB);
    assert_too_long(ludt.encode(), "pointer to optional part", 65_536, 65_535);
}

// ── Decoding: a length that runs past the end of the message ────────────────

#[test]
fn a_data_length_past_the_end_of_the_message_is_an_error() {
    // The 255-octet UDT, one octet short: the length indicator says 255, 254
    // octets follow.
    let mut udt = udt_255_by_hand();
    udt.pop();
    assert!(matches!(
        UnitData::decode(&udt),
        Err(SccpError::TooShort {
            expected: 267,
            actual: 266
        })
    ));
    assert!(SccpMessage::decode(&udt).is_err());

    let mut xudt = xudt_255_by_hand();
    xudt.pop();
    assert!(matches!(
        ExtendedUnitData::decode(&xudt),
        Err(SccpError::TooShort {
            expected: 269,
            actual: 268
        })
    ));

    // Nothing at all after the length indicator.
    let bare = [
        0x09, 0x00, 0x03, 0x05, 0x07, 0x02, 0x42, 0xC8, 0x02, 0x42, 0xC9, 0xFF,
    ];
    assert!(matches!(
        UnitData::decode(&bare),
        Err(SccpError::TooShort { .. })
    ));
}

#[test]
fn an_address_length_past_the_end_of_the_message_is_an_error() {
    // Called length says 0x7f, five octets follow.
    let bytes = [
        0x09, 0x00, 0x03, 0x05, 0x07, 0x7F, 0x42, 0xC8, 0x02, 0x42, 0xC9,
    ];
    assert!(matches!(
        UnitData::decode(&bytes),
        Err(SccpError::TooShort { .. })
    ));
}

#[test]
fn a_long_data_length_past_the_end_of_the_message_is_an_error() {
    // LUDT whose two-octet length says 0x0100, two octets follow.
    let bytes = [
        0x13, 0x00, 0x0F, 0x07, 0x00, 0x08, 0x00, 0x09, 0x00, 0x00, 0x00, 0x02, 0x42, 0xC8, 0x02,
        0x42, 0xC9, 0x00, 0x01, 0x62, 0x40,
    ];
    assert!(matches!(
        LongUnitData::decode(&bytes),
        Err(SccpError::TooShort {
            expected: 275,
            actual: 21
        })
    ));
}

/// What 1.1.1 put on the wire for 256 octets of data: length octet `00`. A
/// receiver reads an empty data parameter and no error, which is why the
/// encoder has to refuse.
#[test]
fn the_wrapped_length_of_earlier_releases_decodes_to_something_else() {
    let mut wrapped = vec![
        0x09, 0x00, 0x03, 0x05, 0x07, 0x02, 0x42, 0xC8, 0x02, 0x42, 0xC9, 0x00,
    ];
    wrapped.extend_from_slice(&data(256));
    let decoded = UnitData::decode(&wrapped).unwrap();
    assert!(decoded.data.is_empty());
}

// ── Wireshark as an independent decoder ─────────────────────────────────────

/// One field of the dissection, as `tshark -T pdml` reports it.
struct Field {
    name: String,
    show: String,
    value: String,
}

struct Dissection {
    fields: Vec<Field>,
}

impl Dissection {
    fn all(&self, name: &str) -> Vec<&Field> {
        self.fields.iter().filter(|f| f.name == name).collect()
    }

    #[track_caller]
    fn one(&self, name: &str) -> &Field {
        let found = self.all(name);
        assert_eq!(found.len(), 1, "{name} should be dissected exactly once");
        found[0]
    }

    #[track_caller]
    fn show(&self, name: &str, expected: &str) -> &Self {
        assert_eq!(self.one(name).show, expected, "display value of {name}");
        self
    }

    #[track_caller]
    fn hex(&self, name: &str, expected: &str) -> &Self {
        assert_eq!(self.one(name).value, expected, "raw octets of {name}");
        self
    }

    /// Nothing malformed, no expert information, anywhere in the frame.
    #[track_caller]
    fn clean(&self) -> &Self {
        let problems: Vec<&str> = self
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .filter(|n| n.starts_with("_ws.malformed") || n.starts_with("_ws.expert"))
            .collect();
        assert!(problems.is_empty(), "Wireshark reports {problems:?}");
        self
    }
}

fn tool(name: &str) -> bool {
    Command::new(name)
        .arg("-v")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Whether `tshark` and `text2pcap` can be run. Prints a `SKIP` line when not.
fn tshark_available() -> bool {
    if tool("tshark") && tool("text2pcap") {
        return true;
    }
    let message = "tshark / text2pcap not found: the independent Wireshark dissection \
                   is skipped, only the byte-level vectors run";
    assert!(
        std::env::var_os("SCCP_REQUIRE_TSHARK").is_none(),
        "SCCP_REQUIRE_TSHARK is set but {message}"
    );
    eprintln!("SKIP: {message}");
    false
}

/// An SCCP message inside an M3UA DATA message (RFC 4666): common header
/// `version 1, reserved, class 1 (Transfer), type 1 (DATA), length`, then one
/// Protocol Data parameter (tag 0x0210): `OPC 1, DPC 2, SI 3 (SCCP), NI 0,
/// MP 0, SLS 0, user data`, padded to four octets.
fn m3ua_frame(sccp: &[u8]) -> Vec<u8> {
    let mut parameter = Vec::new();
    parameter.extend_from_slice(&1u32.to_be_bytes());
    parameter.extend_from_slice(&2u32.to_be_bytes());
    parameter.extend_from_slice(&[3, 0, 0, 0]);
    parameter.extend_from_slice(sccp);
    let parameter_length = 4 + parameter.len();
    let padding = (4 - parameter_length % 4) % 4;

    let mut frame = vec![1, 0, 1, 1];
    frame.extend_from_slice(&((8 + parameter_length + padding) as u32).to_be_bytes());
    frame.extend_from_slice(&0x0210u16.to_be_bytes());
    frame.extend_from_slice(&(parameter_length as u16).to_be_bytes());
    frame.extend_from_slice(&parameter);
    frame.resize(frame.len() + padding, 0);
    frame
}

fn attribute(line: &str, key: &str) -> Option<String> {
    let needle = format!(" {key}=\"");
    let start = line.find(&needle)? + needle.len();
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

static SEQUENCE: AtomicU32 = AtomicU32::new(0);

/// Dissect one SCCP message with `tshark`. `None` when the tools are missing.
fn dissect(sccp: &[u8]) -> Option<Dissection> {
    if !tshark_available() {
        return None;
    }
    let directory: PathBuf = std::env::temp_dir().join(format!(
        "sccp-tshark-{}-{}",
        std::process::id(),
        SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).expect("temporary directory");
    let dump = directory.join("frame.txt");
    let capture = directory.join("frame.pcap");

    let mut text = String::from("000000");
    for byte in m3ua_frame(sccp) {
        text.push_str(&format!(" {byte:02x}"));
    }
    text.push('\n');
    std::fs::write(&dump, text).expect("write hex dump");

    // -S: dummy SCTP DATA chunk, ports 2905 (M3UA), payload protocol id 3.
    let converted = Command::new("text2pcap")
        .args(["-q", "-S", "2905,2905,3"])
        .arg(&dump)
        .arg(&capture)
        .output()
        .expect("run text2pcap");
    assert!(
        converted.status.success(),
        "text2pcap failed: {}",
        String::from_utf8_lossy(&converted.stderr)
    );

    // An empty configuration directory keeps the run independent of whatever
    // protocol preferences the local Wireshark profile carries.
    let dissected = Command::new("tshark")
        .env("WIRESHARK_CONFIG_DIR", &directory)
        .env("XDG_CONFIG_HOME", &directory)
        .arg("-r")
        .arg(&capture)
        .args(["-T", "pdml"])
        .output()
        .expect("run tshark");
    assert!(
        dissected.status.success(),
        "tshark failed: {}",
        String::from_utf8_lossy(&dissected.stderr)
    );
    let _ = std::fs::remove_dir_all(&directory);

    let pdml = String::from_utf8_lossy(&dissected.stdout).into_owned();
    let fields = pdml
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            if !(line.starts_with("<field ") || line.starts_with("<proto ")) {
                return None;
            }
            Some(Field {
                name: attribute(line, "name")?,
                show: attribute(line, "show").unwrap_or_default(),
                value: attribute(line, "value").unwrap_or_default(),
            })
        })
        .collect();
    Some(Dissection { fields })
}

#[test]
fn wireshark_reads_the_255_octet_udt_back_whole() {
    let udt = UnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(255));
    let Some(d) = dissect(&udt.encode().unwrap()) else {
        return;
    };
    d.clean()
        .show("sccp.message_type", "0x09")
        .show("sccp.variable_pointer1", "3")
        .show("sccp.variable_pointer2", "5")
        .show("sccp.variable_pointer3", "7")
        .show("sccp.called.ssn", "200")
        .show("sccp.calling.ssn", "201")
        .show("data.len", "255")
        .hex("data.data", &hex_of(&data(255)));
}

#[test]
fn wireshark_reads_the_255_octet_xudt_back_whole() {
    let xudt = ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(255));
    let Some(d) = dissect(&xudt.encode().unwrap()) else {
        return;
    };
    d.clean()
        .show("sccp.message_type", "0x11")
        .show("sccp.hops", "0x0f")
        .show("sccp.variable_pointer1", "4")
        .show("sccp.variable_pointer2", "6")
        .show("sccp.variable_pointer3", "8")
        .show("sccp.optional_pointer", "0")
        .show("sccp.called.ssn", "200")
        .show("sccp.calling.ssn", "201")
        .show("data.len", "255")
        .hex("data.data", &hex_of(&data(255)));
}

#[test]
fn wireshark_reads_the_xudt_with_the_largest_optional_pointer() {
    let mut xudt =
        ExtendedUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(247));
    xudt.optional_part = vec![0x12, 0x01, 0x03, 0x00];
    let Some(d) = dissect(&xudt.encode().unwrap()) else {
        return;
    };
    d.clean()
        .show("sccp.optional_pointer", "255")
        .show("sccp.importance", "0x03")
        .show("data.len", "247")
        .hex("data.data", &hex_of(&data(247)));
}

#[test]
fn wireshark_reads_the_udt_with_the_largest_data_pointer() {
    let udt = UnitData::new(address_of(126), address_of(126), vec![0x62, 0x40]);
    let Some(d) = dissect(&udt.encode().unwrap()) else {
        return;
    };
    d.clean()
        .show("sccp.variable_pointer2", "129")
        .show("sccp.variable_pointer3", "255")
        // Wireshark displays at most 224 global title digits, so the digits
        // are not compared: that it finds the data proves it followed both
        // pointers past the two 126-octet addresses.
        .show("data.len", "2")
        .hex("data.data", "6240");
}

#[test]
fn wireshark_reads_the_ludt_with_256_octets_of_data() {
    let ludt = LongUnitData::new(ssn_address(CALLED_SSN), ssn_address(CALLING_SSN), data(256));
    let Some(d) = dissect(&ludt.encode().unwrap()) else {
        return;
    };
    d.clean()
        .show("sccp.message_type", "0x13")
        .show("data.len", "256")
        .hex("data.data", &hex_of(&data(256)));
}
