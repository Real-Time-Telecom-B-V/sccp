# Changelog

All notable changes are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html). See
[VERSIONING.md](VERSIONING.md) for the policy.

## [1.1.1]

### Fixed
- **A global title says on the wire how many digits it has.** `GlobalTitle::encode`
  wrote the encoding scheme (GT 0011, 0100) and the odd/even bit (GT 0001) as the
  caller left them, so a title built with scheme 1 and an even number of digits
  went out marked "odd" and a receiver dropped its last digit. The indicator is
  now taken from the digit count: scheme 1 or 2 is written as whichever of the
  two is true, any other scheme is written as given. Q.713 3.4.2.3.
- **The filler after an odd number of digits is `0000`**, as Q.713 3.4.2.3.1 has
  it, for the formats that carry an odd/even indication. It was `1111`.
- **Decoding honours the indication.** With "odd" the high half of the last octet
  is the filler and is no longer read as a digit, so an odd title from a peer
  that writes filler `0000` no longer gains a trailing `0`.

GT 0010 has no odd/even indication and is unchanged: an odd count still goes out
with a `1111` filler. A decoder older than this release reads a new odd title one
`0` too long, since it took only `1111` for a filler.

## [1.1.0]

### Added
- **`ExtendedUnitData`** (XUDT, `0x11`), **`ExtendedUnitDataService`** (XUDTS,
  `0x12`), **`LongUnitData`** (LUDT, `0x13`) and **`LongUnitDataService`** (LUDTS,
  `0x14`) — the extended and long connectionless messages, each carrying a
  **hop counter** (the standard GTT loop breaker) and an opaque optional
  parameter part. XUDT/XUDTS use four one-octet pointers; LUDT/LUDTS use
  two-octet little-endian pointers and a two-octet data length, so they carry
  user data past the ~255-octet UDT/XUDT ceiling. Wired into `SccpMessage`
  decode/encode/`Display` and mirrored on the Python side. Encode vectors are
  known-answer-tested against the Wireshark Q.713 dissector.
- **`DEFAULT_HOP_COUNTER`** (15) and, on `SccpMessage`, the `called_party` /
  `calling_party` / `data` / `hop_counter` accessors.
- The remaining Q.713 §3.12 return causes: error in message transport, error in
  local processing, destination cannot perform reassembly, SCCP failure,
  segmentation not supported, segmentation failure.

### Fixed
- **`ReturnCause::HopCounterViolation` is `0x0C`, not `0x0D`.** The 1.0.0 value
  (13) is "segmentation not supported"; hop counter violation is 12 per
  Q.713 §3.12, confirmed against the Wireshark SCCP dissector. This is a
  wire-visible correction to any UDTS/XUDTS that carried this cause.

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

[1.1.0]: https://github.com/Real-Time-Telecom-B-V/sccp/releases/tag/v1.1.0
[1.0.0]: https://github.com/Real-Time-Telecom-B-V/sccp/releases/tag/v1.0.0
