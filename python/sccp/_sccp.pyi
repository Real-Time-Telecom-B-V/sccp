"""Type stubs for the Rust-backed ``sccp._sccp`` extension module."""

from __future__ import annotations

# ── Message-type constants (Q.713 §4) ────────────────────────────────────────
MESSAGE_TYPE_UDT: int
MESSAGE_TYPE_UDTS: int
MESSAGE_TYPE_XUDT: int
MESSAGE_TYPE_XUDTS: int
MESSAGE_TYPE_LUDT: int
MESSAGE_TYPE_LUDTS: int

# ── Subsystem Numbers (Q.713 §3.4.2.2) ───────────────────────────────────────
SSN_UNKNOWN: int
SSN_SCCP_MGMT: int
SSN_ISUP: int
SSN_OMAP: int
SSN_MAP: int
SSN_HLR: int
SSN_VLR: int
SSN_MSC: int
SSN_EIR: int
SSN_AUC: int
SSN_CAP: int
SSN_PCAP: int

# ── Return causes for UDTS (Q.713 §3.12) ─────────────────────────────────────
RETURN_CAUSE_NO_TRANSLATION_FOR_ADDRESS: int
RETURN_CAUSE_NO_TRANSLATION_FOR_SPECIFIC_ADDRESS: int
RETURN_CAUSE_SUBSYSTEM_CONGESTION: int
RETURN_CAUSE_SUBSYSTEM_FAILURE: int
RETURN_CAUSE_UNEQUIPPED: int
RETURN_CAUSE_MTP_FAILURE: int
RETURN_CAUSE_NETWORK_CONGESTION: int
RETURN_CAUSE_UNQUALIFIED: int
RETURN_CAUSE_HOP_COUNTER_VIOLATION: int

class SccpError(Exception):
    """SCCP protocol / codec error (ITU-T Q.711-Q.716)."""

class GlobalTitle:
    """An SCCP Global Title. Built via one of the classmethods below."""

    indicator: int
    digits: str | None
    @staticmethod
    def no_title() -> GlobalTitle:
        """No Global Title (used with SSN routing)."""
    @staticmethod
    def gt0001(
        digits: str, *, nature_of_address: int, odd_even: bool = False
    ) -> GlobalTitle:
        """GT format 0001: Nature of Address Indicator + digits."""
    @staticmethod
    def gt0010(digits: str, *, translation_type: int) -> GlobalTitle:
        """GT format 0010: Translation Type + digits."""
    @staticmethod
    def gt0011(
        digits: str, *, translation_type: int, numbering_plan: int, encoding_scheme: int
    ) -> GlobalTitle:
        """GT format 0011: Translation Type + Numbering Plan + Encoding Scheme + digits."""
    @staticmethod
    def gt0100(
        digits: str,
        *,
        translation_type: int,
        numbering_plan: int,
        encoding_scheme: int,
        nature_of_address: int,
    ) -> GlobalTitle:
        """GT format 0100: Translation Type + Numbering Plan + Encoding Scheme +
        Nature of Address + digits (E.164)."""
    def __eq__(self, other: object) -> bool: ...

class Address:
    """An SCCP Called / Calling Party Address."""

    route_on_ssn: bool
    point_code: int | None
    ssn: int | None
    global_title: GlobalTitle
    @staticmethod
    def with_gt(global_title: GlobalTitle, ssn: int | None = None) -> Address:
        """Build an address that routes on a Global Title, optionally with an SSN."""
    @staticmethod
    def with_ssn(ssn: int, point_code: int | None = None) -> Address:
        """Build an address that routes on the SSN, optionally with a point code."""
    def encode(self) -> bytes:
        """Encode just this address (no length prefix)."""
    @staticmethod
    def decode(data: bytes) -> Address:
        """Decode an address from its own bytes (length-prefix stripped)."""
    def __eq__(self, other: object) -> bool: ...

class UnitData:
    """An SCCP Unitdata (UDT) message — connectionless data transfer."""

    protocol_class: int
    message_handling: int
    called_party: Address
    calling_party: Address
    data: bytes
    def __init__(
        self,
        called_party: Address,
        calling_party: Address,
        data: bytes,
        *,
        protocol_class: int = 0,
        message_handling: int = 0,
    ) -> None: ...
    def encode(self) -> bytes:
        """Encode the complete UDT message (including the leading type octet)."""
    def __eq__(self, other: object) -> bool: ...

class UnitDataService:
    """An SCCP Unitdata Service (UDTS) message — the UDT error return."""

    return_cause: int
    called_party: Address
    calling_party: Address
    data: bytes
    def __init__(
        self,
        return_cause: int,
        called_party: Address,
        calling_party: Address,
        data: bytes,
    ) -> None: ...
    def encode(self) -> bytes:
        """Encode the complete UDTS message (including the leading type octet)."""
    def __eq__(self, other: object) -> bool: ...

def decode(data: bytes) -> UnitData | UnitDataService:
    """Decode a connectionless SCCP message into a :class:`UnitData` or
    :class:`UnitDataService`."""
