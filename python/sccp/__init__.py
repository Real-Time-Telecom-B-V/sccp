"""sccp — Rust-backed SCCP (ITU-T Q.711-Q.716) connectionless codec for Python.

SCCP (Signaling Connection Control Part) is the SS7 network layer that carries
TCAP (and thus MAP / CAP) between signalling nodes. This package exposes the same
connectionless codec the Rust crate (``cargo add sccp``) ships — Global Title
addressing, Subsystem Numbers, and Unitdata (UDT) / Unitdata Service (UDTS)
messages — from one source tree / one version.

The wire work (Address Indicator pack/unpack, the variable-part pointer
arithmetic, TBCD digit packing, body copy) runs in Rust; Python just builds and
inspects messages.
"""

from __future__ import annotations

from importlib.metadata import PackageNotFoundError, version

from ._sccp import (
    MESSAGE_TYPE_LUDT,
    MESSAGE_TYPE_LUDTS,
    MESSAGE_TYPE_UDT,
    MESSAGE_TYPE_UDTS,
    MESSAGE_TYPE_XUDT,
    MESSAGE_TYPE_XUDTS,
    RETURN_CAUSE_HOP_COUNTER_VIOLATION,
    RETURN_CAUSE_MTP_FAILURE,
    RETURN_CAUSE_NETWORK_CONGESTION,
    RETURN_CAUSE_NO_TRANSLATION_FOR_ADDRESS,
    RETURN_CAUSE_NO_TRANSLATION_FOR_SPECIFIC_ADDRESS,
    RETURN_CAUSE_SUBSYSTEM_CONGESTION,
    RETURN_CAUSE_SUBSYSTEM_FAILURE,
    RETURN_CAUSE_UNEQUIPPED,
    RETURN_CAUSE_UNQUALIFIED,
    SSN_AUC,
    SSN_CAP,
    SSN_EIR,
    SSN_HLR,
    SSN_ISUP,
    SSN_MAP,
    SSN_MSC,
    SSN_OMAP,
    SSN_PCAP,
    SSN_SCCP_MGMT,
    SSN_UNKNOWN,
    SSN_VLR,
    Address,
    GlobalTitle,
    SccpError,
    UnitData,
    UnitDataService,
    decode,
)

try:
    __version__ = version("sccp")
except PackageNotFoundError:  # running from a source checkout without an installed dist
    __version__ = "0.0.0+unknown"

__all__ = [
    # addresses + global titles
    "Address",
    "GlobalTitle",
    # messages + codec
    "UnitData",
    "UnitDataService",
    "decode",
    "SccpError",
    # message-type constants
    "MESSAGE_TYPE_UDT",
    "MESSAGE_TYPE_UDTS",
    "MESSAGE_TYPE_XUDT",
    "MESSAGE_TYPE_XUDTS",
    "MESSAGE_TYPE_LUDT",
    "MESSAGE_TYPE_LUDTS",
    # subsystem numbers
    "SSN_UNKNOWN",
    "SSN_SCCP_MGMT",
    "SSN_ISUP",
    "SSN_OMAP",
    "SSN_MAP",
    "SSN_HLR",
    "SSN_VLR",
    "SSN_MSC",
    "SSN_EIR",
    "SSN_AUC",
    "SSN_CAP",
    "SSN_PCAP",
    # return causes
    "RETURN_CAUSE_NO_TRANSLATION_FOR_ADDRESS",
    "RETURN_CAUSE_NO_TRANSLATION_FOR_SPECIFIC_ADDRESS",
    "RETURN_CAUSE_SUBSYSTEM_CONGESTION",
    "RETURN_CAUSE_SUBSYSTEM_FAILURE",
    "RETURN_CAUSE_UNEQUIPPED",
    "RETURN_CAUSE_MTP_FAILURE",
    "RETURN_CAUSE_NETWORK_CONGESTION",
    "RETURN_CAUSE_UNQUALIFIED",
    "RETURN_CAUSE_HOP_COUNTER_VIOLATION",
    "__version__",
]
