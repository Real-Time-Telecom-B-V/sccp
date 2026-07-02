# sccp — overview

An **SCCP (Signaling Connection Control Part) codec** per **ITU-T Q.711-Q.716**.
SCCP is the SS7 network layer that sits above MTP3 and below TCAP; it adds
Global-Title addressing and subsystem multiplexing on top of raw point codes, and
carries the connectionless traffic (TCAP transactions, and thus MAP / CAP) that
makes up most SS7 signalling. This crate is a pure codec — no transport, no async,
no I/O — so it encodes/decodes bytes and nothing else.

## Module map

| Module | Public surface |
|---|---|
| `message` | `SccpMessage` (inbound dispatch), `UnitData` (UDT encode/decode), `UnitDataService` (UDTS) |
| `address` | `SccpAddress` — Address Indicator, point code, SSN, Global Title |
| `global_title` | `GlobalTitle` (5 variants), `GtIndicator` |
| `types` | `MessageType` (Q.713 §4 table), `SubsystemNumber`, `ReturnCause` |
| `bcd` | `encode_tbcd` / `decode_tbcd` — Telephony-BCD digit packing |
| `error` | `SccpError` (`thiserror`) |

The crate root re-exports the headline types, so `use sccp::{SccpAddress,
GlobalTitle, SubsystemNumber, UnitData};` is enough for typical use.

## The pieces

### Messages (`message`)

`UnitData` models the connectionless **UDT** message (type `0x09`): protocol
class + message handling, a called and a calling `SccpAddress`, and a user-data
payload (a TCAP APDU in practice). `encode` lays out the three-pointer variable
part; `decode` walks the pointers back to the length-prefixed fields and validates
every offset against the buffer length, returning `SccpError::TooShort` rather than
panicking on a truncated frame.

`SccpMessage::decode` reads the leading message-type octet and dispatches — today
that means UDT; other types decode their `MessageType` and are reported as
`InvalidMessageType` until modelled. `UnitDataService` (UDTS) and `ReturnCause`
are present as types for the error-return path.

### Addresses (`address`)

`SccpAddress` carries the Address Indicator octet (point-code-present,
SSN-present, GT indicator, routing indicator), an optional 2-byte ITU point code
(little-endian), an optional `SubsystemNumber`, and a `GlobalTitle`. Constructors
`with_gt` (route on GT) and `with_ssn` (route on SSN) cover the common cases; the
byte layout round-trips through `encode` / `decode`.

### Global Titles (`global_title`)

`GtIndicator` is the 4-bit field selecting one of five `GlobalTitle` shapes:
`NoTitle`, `Gt0001` (nature of address + odd/even), `Gt0010` (translation type),
`Gt0011` (+ numbering plan + encoding scheme), and `Gt0100` (all four fields).
Digits are TBCD-coded; `GlobalTitle::digits()` returns the decoded string.

### Types (`types`)

`MessageType` is the full Q.713 §4 message-type table (CR/CC/CREF/… through
LUDT/LUDTS). `SubsystemNumber` names the well-known SSNs (SCCP-MGMT, ISUP, OMAP,
MAP, HLR, VLR, MSC, EIR, AuC, CAP, PCAP) with an `Other(u8)` fall-through.
`ReturnCause` enumerates the UDTS return causes.

### TBCD (`bcd`)

`encode_tbcd` / `decode_tbcd` implement Telephony-BCD: two digits per byte,
low nibble first, `0xF` filler for an odd count, and the `*` `#` `a` `b` `c`
extension nibbles.

## Scope

Connectionless (class 0/1) only. Connection-oriented SCCP (CR/CC/DT class 2/3 and
the associated state machine) and the XUDT/LUDT segmentation variants are out of
scope for this codec.

## Where it fits

In the SS7 stack, SCCP bytes ride an MTP3-User transport — native MTP3 over M2PA
links, or M3UA over SCTP. This crate produces and consumes those bytes; the
transport and routing live in separate, Linux-only providers so the codec stays
pure and portable.
