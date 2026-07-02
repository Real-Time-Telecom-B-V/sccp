# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). See
[VERSIONING.md](VERSIONING.md) for the policy.

## [1.0.0]

First release — the SCCP connectionless codec for the SS7 stack.

### Added
- **`SccpMessage`** — inbound dispatch on the message-type octet; **`UnitData`**
  (UDT, `0x09`) encode/decode including variable-part pointer arithmetic.
- **`UnitDataService`** (UDTS) and **`ReturnCause`** modelled as types.
- **`SccpAddress`** — Address Indicator handling (point-code / SSN / GT-indicator /
  routing-indicator), optional 2-byte point code, optional `SubsystemNumber`.
- **`GlobalTitle`** / **`GtIndicator`** — the four GT formats (`Gt0001`, `Gt0010`,
  `Gt0011`, `Gt0100`) plus `NoTitle`, with encode/decode and `Display`.
- **`SubsystemNumber`** (SCCP-MGMT/ISUP/MAP/HLR/VLR/MSC/EIR/AuC/CAP/…) and the
  full **`MessageType`** table (Q.713 §4).
- **`bcd`** — Telephony-BCD (TBCD) pack/unpack with `*`/`#`/`a`–`c` nibbles.
- **`SccpError`** — a `thiserror` enum covering the decode/encode failure modes.
- Unit + integration tests covering address, GT, TBCD, and UDT round-trips and
  error paths; a runnable doctest on the crate root.

[1.0.0]: https://github.com/Real-Time-Telecom-B-V/sccp/releases/tag/v1.0.0
