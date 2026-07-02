"""Codec parity / round-trip tests for the sccp wheel.

These exercise the same Rust codec the crate ships, through the Python surface:
``encode`` must match the ITU-T Q.713 wire form, ``decode`` must recover the
fields, and Global Title / SSN addressing must round-trip. Digits are synthetic
(fictional +1-555 range).
"""

from __future__ import annotations

import pytest

import sccp

# Wire form of a UDT routed on SSN (HLR ← MSC) carrying a 2-byte body:
#   09          message type (UDT)
#   00          protocol class 0 / no special handling
#   03 05 07    pointers (called / calling / data)
#   02 42 06    called:  len 2, AI 0x42 (SSN + route-on-SSN), SSN 6 (HLR)
#   02 42 08    calling: len 2, AI 0x42, SSN 8 (MSC)
#   02 62 40    data:    len 2, [0x62, 0x40]
GOLDEN_UDT_SSN = bytes.fromhex("0900030507024206024208026240")

# Same shape but a UDTS (type 0x0A) with return cause 3 (Subsystem failure) and a
# 1-byte body.
GOLDEN_UDTS = bytes.fromhex("0a0303050702420602420801aa")


def test_message_type_constants() -> None:
    assert sccp.MESSAGE_TYPE_UDT == 0x09
    assert sccp.MESSAGE_TYPE_UDTS == 0x0A
    assert sccp.MESSAGE_TYPE_XUDT == 0x11
    assert sccp.MESSAGE_TYPE_XUDTS == 0x12
    assert sccp.MESSAGE_TYPE_LUDT == 0x13
    assert sccp.MESSAGE_TYPE_LUDTS == 0x14


def test_ssn_constants() -> None:
    assert sccp.SSN_SCCP_MGMT == 1
    assert sccp.SSN_MAP == 5
    assert sccp.SSN_HLR == 6
    assert sccp.SSN_VLR == 7
    assert sccp.SSN_MSC == 8
    assert sccp.SSN_CAP == 146
    assert sccp.SSN_PCAP == 249


def test_return_cause_constants() -> None:
    assert sccp.RETURN_CAUSE_NO_TRANSLATION_FOR_ADDRESS == 0
    assert sccp.RETURN_CAUSE_SUBSYSTEM_FAILURE == 3
    assert sccp.RETURN_CAUSE_UNEQUIPPED == 4
    assert sccp.RETURN_CAUSE_HOP_COUNTER_VIOLATION == 13


def test_udt_ssn_matches_golden_vector() -> None:
    called = sccp.Address.with_ssn(sccp.SSN_HLR)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    udt = sccp.UnitData(called, calling, bytes([0x62, 0x40]))
    assert udt.encode() == GOLDEN_UDT_SSN


def test_decode_golden_udt_ssn() -> None:
    msg = sccp.decode(GOLDEN_UDT_SSN)
    assert isinstance(msg, sccp.UnitData)
    assert msg.protocol_class == 0
    assert msg.called_party.ssn == sccp.SSN_HLR
    assert msg.calling_party.ssn == sccp.SSN_MSC
    assert msg.called_party.route_on_ssn is True
    assert msg.data == bytes([0x62, 0x40])
    assert msg.encode() == GOLDEN_UDT_SSN


def test_udt_ssn_round_trip() -> None:
    called = sccp.Address.with_ssn(sccp.SSN_HLR, point_code=1234)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    data = bytes([0x62, 0x40]) + bytes(range(32))
    udt = sccp.UnitData(called, calling, data, protocol_class=1)
    decoded = sccp.decode(udt.encode())
    assert isinstance(decoded, sccp.UnitData)
    assert decoded.protocol_class == 1
    assert decoded.called_party.point_code == 1234
    assert decoded.called_party.ssn == sccp.SSN_HLR
    assert decoded.data == data
    assert decoded.encode() == udt.encode()


def test_udt_gt0100_round_trip() -> None:
    gt = sccp.GlobalTitle.gt0100(
        "15551234567",
        translation_type=0,
        numbering_plan=1,
        encoding_scheme=1,
        nature_of_address=4,
    )
    assert gt.indicator == 4
    assert gt.digits == "15551234567"
    called = sccp.Address.with_gt(gt, ssn=sccp.SSN_HLR)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    udt = sccp.UnitData(called, calling, bytes([0x62, 0x40]))
    wire = udt.encode()
    decoded = sccp.decode(wire)
    assert isinstance(decoded, sccp.UnitData)
    assert decoded.called_party.route_on_ssn is False
    assert decoded.called_party.ssn == sccp.SSN_HLR
    assert decoded.called_party.global_title.digits == "15551234567"
    assert decoded.encode() == wire


@pytest.mark.parametrize(
    "gt",
    [
        sccp.GlobalTitle.gt0001("12345", nature_of_address=4, odd_even=True),
        sccp.GlobalTitle.gt0010("5550199", translation_type=9),
        sccp.GlobalTitle.gt0011(
            "5550142", translation_type=0, numbering_plan=1, encoding_scheme=2
        ),
    ],
)
def test_global_title_formats_round_trip(gt) -> None:
    called = sccp.Address.with_gt(gt)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    udt = sccp.UnitData(called, calling, b"\x01")
    decoded = sccp.decode(udt.encode())
    assert decoded.called_party.global_title == gt
    assert decoded.called_party.global_title.digits == gt.digits


def test_udts_round_trip_and_golden() -> None:
    called = sccp.Address.with_ssn(sccp.SSN_HLR)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    udts = sccp.UnitDataService(
        sccp.RETURN_CAUSE_SUBSYSTEM_FAILURE, called, calling, b"\xaa"
    )
    assert udts.encode() == GOLDEN_UDTS
    decoded = sccp.decode(GOLDEN_UDTS)
    assert isinstance(decoded, sccp.UnitDataService)
    assert decoded.return_cause == sccp.RETURN_CAUSE_SUBSYSTEM_FAILURE
    assert decoded.data == b"\xaa"
    assert decoded.encode() == GOLDEN_UDTS


def test_address_encode_decode_gt() -> None:
    gt = sccp.GlobalTitle.gt0010("5550100", translation_type=7)
    addr = sccp.Address.with_gt(gt, ssn=sccp.SSN_VLR)
    assert sccp.Address.decode(addr.encode()) == addr


def test_empty_data_round_trip() -> None:
    called = sccp.Address.with_ssn(sccp.SSN_HLR)
    calling = sccp.Address.with_ssn(sccp.SSN_MSC)
    udt = sccp.UnitData(called, calling, b"")
    assert sccp.decode(udt.encode()).data == b""


def test_decode_rejects_unknown_type() -> None:
    with pytest.raises(sccp.SccpError):
        sccp.decode(bytes([0xFF, 0, 0, 0, 0]))


def test_decode_rejects_unsupported_type() -> None:
    # CR (0x01) is a valid SCCP type but not decoded by this connectionless codec.
    with pytest.raises(sccp.SccpError):
        sccp.decode(bytes([0x01, 0, 0, 0, 0]))


def test_decode_rejects_truncated() -> None:
    with pytest.raises(sccp.SccpError):
        sccp.decode(b"\x09")


def test_decode_rejects_truncated_variable_part() -> None:
    # Valid UDT header claiming a called-party pointer past the buffer end.
    with pytest.raises(sccp.SccpError):
        sccp.decode(bytes([0x09, 0x00, 0x7F, 0x03, 0x04]))
