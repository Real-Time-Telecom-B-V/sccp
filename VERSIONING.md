# Versioning

`sccp` follows [Semantic Versioning 2.0.0](https://semver.org/). The public API —
the `SccpMessage` / `UnitData` / `UnitDataService` types, `SccpAddress`,
`GlobalTitle` / `GtIndicator`, `SubsystemNumber` / `MessageType` / `ReturnCause`,
the `bcd` functions, and `SccpError` — is the contract.

## The git tag is the source of truth

`Cargo.toml`'s `version` matches the release tag; the release workflow's
`verify-version` job refuses to publish if they disagree. Bump `version`, commit,
tag `vX.Y.Z`, push the tag.

## Post-1.0 rule

- **MAJOR** — remove/rename/re-signature a `pub` item, or change documented
  wire-encoding or decode semantics.
- **MINOR** — backward-compatible additions (new message/address/GT variants, new
  `SubsystemNumber` / `ReturnCause` constants, new helper methods).
- **PATCH** — bug fixes, docs, behaviour-neutral dependency bumps.
