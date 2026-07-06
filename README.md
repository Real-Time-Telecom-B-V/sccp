# sccp

[![crates.io](https://img.shields.io/crates/v/sccp.svg)](https://crates.io/crates/sccp)
[![docs.rs](https://docs.rs/sccp/badge.svg)](https://docs.rs/sccp)
[![CI](https://github.com/Real-Time-Telecom-B-V/sccp/actions/workflows/ci.yaml/badge.svg)](https://github.com/Real-Time-Telecom-B-V/sccp/actions/workflows/ci.yaml)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An **SCCP (Signaling Connection Control Part) codec** per **ITU-T Q.711-Q.716** —
the SS7 network-layer protocol that carries TCAP (and thus MAP / CAP) between
signalling nodes. Pure Rust: encoders and decoders for SCCP addresses, Global
Titles, and the connectionless **Unitdata** messages — **UDT / UDTS** and the
extended / long forms **XUDT / XUDTS / LUDT / LUDTS** (with a hop counter) — with
**no transport, no async, and no I/O** — every consumer can unit-test against it. It
ships as **both** a Rust crate (`cargo add sccp`) and a Rust-backed Python wheel
(`pip install sccp`), built from one source tree and one version.

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

```python
import sccp

gt = sccp.GlobalTitle.gt0100(
    "15551234567", translation_type=0, numbering_plan=1,
    encoding_scheme=1, nature_of_address=4,
)
called = sccp.Address.with_gt(gt, ssn=sccp.SSN_HLR)
calling = sccp.Address.with_ssn(sccp.SSN_MSC)

udt = sccp.UnitData(called, calling, bytes([0x62, 0x40]))
wire = udt.encode()                 # bytes
msg = sccp.decode(wire)             # -> UnitData | UnitDataService
```

## Coverage

- **Messages** — the connectionless types encode + decode, including the
  variable-part pointer arithmetic: `UnitData` (UDT, `0x09`), `UnitDataService`
  (UDTS, `0x0A`), and the extended / long forms `ExtendedUnitData` (XUDT, `0x11`),
  `ExtendedUnitDataService` (XUDTS, `0x12`), `LongUnitData` (LUDT, `0x13`) and
  `LongUnitDataService` (LUDTS, `0x14`). The extended / long messages carry a
  **hop counter** (the standard GTT loop breaker) and an opaque optional part;
  LUDT/LUDTS use two-octet pointers and a two-octet length to carry user data
  past the ~255-octet UDT/XUDT ceiling. `SccpMessage` dispatches inbound bytes on
  the message-type octet; the full `MessageType` table (CR/CC/DT/UDT/XUDT/LUDT/…)
  is recognised. `ReturnCause` covers the Q.713 §3.12 causes.
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

## Performance

Single-core, `cargo bench` ([`benches/codec.rs`](benches/codec.rs)); the codec is
allocation-light (a `Vec` per encoded message; addresses/GT digits on decode).
Encode + decode of a UDT routed on SSN and a UDT / UDTS carrying a full GT0100
(E.164) Global Title are all in the tens-of-nanoseconds range.

A counting-allocator [leak check](examples/leak_check.rs)
(`./scripts/mem_leak_test.sh`) hammers encode/decode of both the SSN and
Global-Title paths and asserts **live bytes stay flat** (Δ 0 over millions of
cycles). Both run in CI.

The Python wheel is the same Rust code behind PyO3; per-call overhead is the
Python↔Rust boundary, not the codec. The module is declared `gil_used = false`,
so it loads on free-threaded ("no-GIL") CPython 3.13t / 3.14t.

## Install

```bash
cargo add sccp          # Rust crate (zero pyo3 in the default build)
pip install sccp        # Rust-backed Python wheel
```

## Development

```bash
cargo test                              # unit + integration + doctests
cargo test --features python            # + the PyO3 binding face
cargo clippy --all-targets -- -D warnings
cargo bench --no-run
./scripts/mem_leak_test.sh              # live-bytes leak check (PASS/FAIL)
cargo deny check                        # advisories, licenses, sources

# Python wheel
maturin develop && pytest python/tests -q
```

## License

MIT — see [LICENSE](LICENSE).
