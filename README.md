# sccp

[![crates.io](https://img.shields.io/crates/v/sccp.svg)](https://crates.io/crates/sccp)
[![docs.rs](https://docs.rs/sccp/badge.svg)](https://docs.rs/sccp)
[![CI](https://github.com/Real-Time-Telecom-B-V/sccp/actions/workflows/ci.yaml/badge.svg)](https://github.com/Real-Time-Telecom-B-V/sccp/actions/workflows/ci.yaml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An **SCCP (Signaling Connection Control Part) codec** per **ITU-T Q.711-Q.716** —
the SS7 network-layer protocol that carries TCAP (and thus MAP / CAP) between
signalling nodes. Pure Rust: encoders and decoders for SCCP addresses, Global
Titles, and connectionless **Unitdata (UDT)** messages, with **no transport, no
async, and no I/O** — every consumer can unit-test against it.

```rust
use sccp::{SccpAddress, GlobalTitle, SubsystemNumber, UnitData};

// Called party: route on a Global Title (E.164 number), landing on the HLR.
let gt = GlobalTitle::Gt0100 {
    translation_type: 0,
    numbering_plan: 1,    // E.164
    encoding_scheme: 1,   // BCD, odd number of digits
    nature_of_address: 4, // international number
    digits: "15551234567".to_string(),
};
let called = SccpAddress::with_gt(gt, Some(SubsystemNumber::Hlr));

// Calling party: route on SSN, from the MSC.
let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);

// Wrap a TCAP payload in a UDT and round-trip it on the wire.
let udt = UnitData::new(called, calling, vec![0x62, 0x40]);
let bytes = udt.encode().unwrap();
let decoded = UnitData::decode(&bytes).unwrap();
assert_eq!(decoded.called_party.ssn, Some(SubsystemNumber::Hlr));
```

## Coverage

- **Messages** — `UnitData` (UDT, type `0x09`): encode + decode, including the
  variable-part pointer arithmetic. `SccpMessage` dispatches inbound bytes on the
  message-type octet. `UnitDataService` (UDTS) and its `ReturnCause` are modelled
  as types; the full `MessageType` table (CR/CC/DT/UDT/XUDT/LUDT/…) is decoded on
  read.
- **Addresses** — `SccpAddress` with the Address Indicator (point-code / SSN /
  GT-indicator / routing-indicator bits), optional 2-byte point code, and
  optional `SubsystemNumber`.
- **Global Titles** — the four GT indicator formats: `Gt0001` (nature of address),
  `Gt0010` (translation type), `Gt0011` (+ numbering plan / encoding scheme), and
  `Gt0100` (all four), plus `NoTitle`.
- **Subsystem Numbers** — named constants for SCCP-MGMT, ISUP, MAP, HLR, VLR, MSC,
  EIR, AuC, CAP, … with an `Other(u8)` catch-all.
- **TBCD digits** — Telephony-BCD pack/unpack (`bcd::encode_tbcd` /
  `bcd::decode_tbcd`) with `*`, `#`, and `a`–`c` nibbles and odd-length filler.

Connection-oriented SCCP (CR/CC/DT class 2/3) is **not** implemented — this crate
targets the connectionless path used by TCAP transactions. See
[`docs/OVERVIEW.md`](docs/OVERVIEW.md) for the module map and full public API.

## Where it fits

SCCP is the SS7 network layer beneath TCAP. In the wider stack this codec pairs
with an MTP3-User transport (native MTP3 over M2PA, or M3UA over SCTP) that
carries the encoded SCCP bytes; the codec itself stays pure and portable.

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo deny check
```

## License

MIT — see [LICENSE](LICENSE).
