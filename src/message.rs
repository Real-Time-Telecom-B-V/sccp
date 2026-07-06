//! SCCP connectionless messages: [`UnitData`] (UDT), [`UnitDataService`] (UDTS)
//! and their extended / long forms [`ExtendedUnitData`] (XUDT),
//! [`ExtendedUnitDataService`] (XUDTS), [`LongUnitData`] (LUDT) and
//! [`LongUnitDataService`] (LUDTS) — the latter four carrying a hop counter —
//! plus the [`SccpMessage`] dispatch enum.

use std::fmt;

use crate::address::SccpAddress;
use crate::error::SccpError;
use crate::types::{MessageType, ReturnCause};

/// Decode the three-pointer variable part shared by UDT and UDTS:
/// `[ptr_called, ptr_calling, ptr_data]` starting at `base`, where each pointer
/// is relative to its own position. Returns the two addresses and the data.
fn decode_variable_part(
    bytes: &[u8],
    base: usize,
) -> Result<(SccpAddress, SccpAddress, Vec<u8>), SccpError> {
    // Three pointers occupy bytes[base..base+3].
    if bytes.len() < base + 3 {
        return Err(SccpError::TooShort {
            expected: base + 3,
            actual: bytes.len(),
        });
    }

    let ptr_called = bytes[base] as usize;
    let ptr_calling = bytes[base + 1] as usize;
    let ptr_data = bytes[base + 2] as usize;

    // Each pointer is relative to its own position.
    let called_offset = base + ptr_called;
    let calling_offset = base + 1 + ptr_calling;
    let data_offset = base + 2 + ptr_data;

    let called_party = decode_length_prefixed_address(bytes, called_offset)?;
    let calling_party = decode_length_prefixed_address(bytes, calling_offset)?;
    let data = decode_length_prefixed_data(bytes, data_offset)?;

    Ok((called_party, calling_party, data))
}

fn decode_length_prefixed_address(bytes: &[u8], offset: usize) -> Result<SccpAddress, SccpError> {
    let (start, len) = length_prefixed_bounds(bytes, offset)?;
    SccpAddress::decode(&bytes[start..start + len])
}

fn decode_length_prefixed_data(bytes: &[u8], offset: usize) -> Result<Vec<u8>, SccpError> {
    let (start, len) = length_prefixed_bounds(bytes, offset)?;
    Ok(bytes[start..start + len].to_vec())
}

/// Validate a length-prefixed field at `offset` and return `(start, len)` of its
/// payload, erroring if either the length byte or the payload runs past the end.
fn length_prefixed_bounds(bytes: &[u8], offset: usize) -> Result<(usize, usize), SccpError> {
    if offset >= bytes.len() {
        return Err(SccpError::TooShort {
            expected: offset + 1,
            actual: bytes.len(),
        });
    }
    let len = bytes[offset] as usize;
    let start = offset + 1;
    if start + len > bytes.len() {
        return Err(SccpError::TooShort {
            expected: start + len,
            actual: bytes.len(),
        });
    }
    Ok((start, len))
}

/// Encode the three-pointer variable part (called / calling / data), returning
/// the pointer bytes followed by the length-prefixed fields.
fn encode_variable_part(
    called: &SccpAddress,
    calling: &SccpAddress,
    data: &[u8],
) -> Result<Vec<u8>, SccpError> {
    let called_bytes = called.encode()?;
    let calling_bytes = calling.encode()?;

    // Pointers are relative to their own position; the three pointer bytes are
    // followed immediately by the called-address length byte.
    let ptr_called: u8 = 3;
    let ptr_calling: u8 = (2 + 1 + called_bytes.len()) as u8;
    let ptr_data: u8 = (1 + 1 + called_bytes.len() + 1 + calling_bytes.len()) as u8;

    let mut buf = vec![ptr_called, ptr_calling, ptr_data, called_bytes.len() as u8];
    buf.extend_from_slice(&called_bytes);
    buf.push(calling_bytes.len() as u8);
    buf.extend_from_slice(&calling_bytes);
    buf.push(data.len() as u8);
    buf.extend_from_slice(data);
    Ok(buf)
}

/// Decode the XUDT/XUDTS variable part at `base` (the first pointer octet):
/// called, calling and data via the shared three-pointer decode, plus the raw
/// optional part addressed by a fourth pointer at `base + 3` (empty when that
/// pointer is 0). The optional part runs to the end of the message; it is kept
/// opaque so a transiting node preserves segmentation/importance verbatim.
fn decode_extended_variable_part(
    bytes: &[u8],
    base: usize,
) -> Result<(SccpAddress, SccpAddress, Vec<u8>, Vec<u8>), SccpError> {
    if bytes.len() < base + 4 {
        return Err(SccpError::TooShort {
            expected: base + 4,
            actual: bytes.len(),
        });
    }
    let (called_party, calling_party, data) = decode_variable_part(bytes, base)?;

    // Fourth pointer (optional part), relative to its own octet at base + 3.
    let ptr_optional = bytes[base + 3] as usize;
    let optional_part = if ptr_optional == 0 {
        Vec::new()
    } else {
        let offset = base + 3 + ptr_optional;
        if offset > bytes.len() {
            return Err(SccpError::TooShort {
                expected: offset,
                actual: bytes.len(),
            });
        }
        bytes[offset..].to_vec()
    };
    Ok((called_party, calling_party, data, optional_part))
}

/// Encode the XUDT/XUDTS variable part: four one-octet pointers (called,
/// calling, data, optional) followed by the length-prefixed mandatory parts and
/// the raw optional part. The optional pointer is 0 when there is no optional
/// part. Each pointer is relative to its own octet.
fn encode_extended_variable_part(
    called: &SccpAddress,
    calling: &SccpAddress,
    data: &[u8],
    optional: &[u8],
) -> Result<Vec<u8>, SccpError> {
    let called_bytes = called.encode()?;
    let calling_bytes = calling.encode()?;

    // Four pointer octets precede the called-address length byte, so the called
    // pointer is 4; each subsequent pointer steps past the prior length-prefixed
    // field. The optional pointer is relative to the fourth pointer octet.
    let ptr_called = 4usize;
    let ptr_calling = 4 + called_bytes.len();
    let ptr_data = 4 + called_bytes.len() + calling_bytes.len();
    let ptr_optional = if optional.is_empty() {
        0
    } else {
        4 + called_bytes.len() + calling_bytes.len() + data.len()
    };

    let mut buf = vec![
        ptr_called as u8,
        ptr_calling as u8,
        ptr_data as u8,
        ptr_optional as u8,
        called_bytes.len() as u8,
    ];
    buf.extend_from_slice(&called_bytes);
    buf.push(calling_bytes.len() as u8);
    buf.extend_from_slice(&calling_bytes);
    buf.push(data.len() as u8);
    buf.extend_from_slice(data);
    buf.extend_from_slice(optional);
    Ok(buf)
}

/// Read a two-octet little-endian field at `offset`, bounds-checked.
fn read_le16(bytes: &[u8], offset: usize) -> Result<usize, SccpError> {
    if offset + 2 > bytes.len() {
        return Err(SccpError::TooShort {
            expected: offset + 2,
            actual: bytes.len(),
        });
    }
    Ok(u16::from_le_bytes([bytes[offset], bytes[offset + 1]]) as usize)
}

/// Validate a two-octet-length-prefixed field (the LUDT long data) at `offset`,
/// returning `(start, len)` of its payload.
fn long_length_prefixed_bounds(bytes: &[u8], offset: usize) -> Result<(usize, usize), SccpError> {
    let len = read_le16(bytes, offset)?;
    let start = offset + 2;
    if start + len > bytes.len() {
        return Err(SccpError::TooShort {
            expected: start + len,
            actual: bytes.len(),
        });
    }
    Ok((start, len))
}

/// Decode the LUDT/LUDTS variable part of a full message (the mandatory part
/// begins at octet 11, after the 3-octet header and four two-octet pointers).
/// Addresses carry a one-octet length indicator; the long data carries a
/// two-octet little-endian one. Each pointer is relative to the octet following
/// its own first octet, per ITU-T Q.713 §4.3.
fn decode_long_variable_part(
    bytes: &[u8],
) -> Result<(SccpAddress, SccpAddress, Vec<u8>, Vec<u8>), SccpError> {
    if bytes.len() < 11 {
        return Err(SccpError::TooShort {
            expected: 11,
            actual: bytes.len(),
        });
    }
    let ptr_called = read_le16(bytes, 3)?;
    let ptr_calling = read_le16(bytes, 5)?;
    let ptr_data = read_le16(bytes, 7)?;
    let ptr_optional = read_le16(bytes, 9)?;

    let called_party = decode_length_prefixed_address(bytes, 4 + ptr_called)?;
    let calling_party = decode_length_prefixed_address(bytes, 6 + ptr_calling)?;
    let (data_start, data_len) = long_length_prefixed_bounds(bytes, 8 + ptr_data)?;
    let data = bytes[data_start..data_start + data_len].to_vec();

    let optional_part = if ptr_optional == 0 {
        Vec::new()
    } else {
        let offset = 10 + ptr_optional;
        if offset > bytes.len() {
            return Err(SccpError::TooShort {
                expected: offset,
                actual: bytes.len(),
            });
        }
        bytes[offset..].to_vec()
    };
    Ok((called_party, calling_party, data, optional_part))
}

/// Encode the LUDT/LUDTS variable part: four two-octet little-endian pointers
/// (called, calling, long data, optional), a one-octet length indicator on each
/// address, a two-octet length indicator on the long data, then the raw optional
/// part. The optional pointer is 0 when there is no optional part.
fn encode_long_variable_part(
    called: &SccpAddress,
    calling: &SccpAddress,
    data: &[u8],
    optional: &[u8],
) -> Result<Vec<u8>, SccpError> {
    let called_bytes = called.encode()?;
    let calling_bytes = calling.encode()?;

    // Absolute message offsets of the mandatory length indicators. The pointer
    // block is octets 3..11; the mandatory part starts at 11.
    let called_li = 11usize;
    let calling_li = called_li + 1 + called_bytes.len();
    let data_li = calling_li + 1 + calling_bytes.len();
    let optional_off = data_li + 2 + data.len();

    // Pointer value = target offset - (pointer-field offset + 1).
    let ptr_called = (called_li - 4) as u16;
    let ptr_calling = (calling_li - 6) as u16;
    let ptr_data = (data_li - 8) as u16;
    let ptr_optional = if optional.is_empty() {
        0
    } else {
        (optional_off - 10) as u16
    };

    let mut buf = Vec::new();
    buf.extend_from_slice(&ptr_called.to_le_bytes());
    buf.extend_from_slice(&ptr_calling.to_le_bytes());
    buf.extend_from_slice(&ptr_data.to_le_bytes());
    buf.extend_from_slice(&ptr_optional.to_le_bytes());
    buf.push(called_bytes.len() as u8);
    buf.extend_from_slice(&called_bytes);
    buf.push(calling_bytes.len() as u8);
    buf.extend_from_slice(&calling_bytes);
    buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
    buf.extend_from_slice(data);
    buf.extend_from_slice(optional);
    Ok(buf)
}

/// SCCP Unitdata (UDT) message — connectionless data transfer.
///
/// ```ignore
/// 0: Message type (0x09)
/// 1: Protocol class + message handling
/// 2: Pointer to called party address
/// 3: Pointer to calling party address
/// 4: Pointer to data
/// Variable: Called party address (length-prefixed)
/// Variable: Calling party address (length-prefixed)
/// Variable: Data (length-prefixed)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitData {
    /// Protocol class (0 or 1).
    pub protocol_class: u8,
    /// Message handling (0 = no special options, 1 = return on error).
    pub message_handling: u8,
    /// Called (destination) party address.
    pub called_party: SccpAddress,
    /// Calling (source) party address.
    pub calling_party: SccpAddress,
    /// User data (TCAP payload typically).
    pub data: Vec<u8>,
}

impl UnitData {
    /// Build a UDT with protocol class 0 and no special message handling.
    pub fn new(called_party: SccpAddress, calling_party: SccpAddress, data: Vec<u8>) -> Self {
        Self {
            protocol_class: 0,
            message_handling: 0,
            called_party,
            calling_party,
            data,
        }
    }

    /// Decode a UDT message from raw bytes (including the leading message-type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.len() < 5 {
            return Err(SccpError::TooShort {
                expected: 5,
                actual: bytes.len(),
            });
        }

        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Udt {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }

        let protocol_class = bytes[1] & 0x0F;
        let message_handling = (bytes[1] >> 4) & 0x0F;

        // Variable part: the three pointers begin at byte 2 (after type + class).
        let (called_party, calling_party, data) = decode_variable_part(bytes, 2)?;

        Ok(Self {
            protocol_class,
            message_handling,
            called_party,
            calling_party,
            data,
        })
    }

    /// Encode this UDT to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![
            MessageType::Udt as u8,
            (self.message_handling << 4) | (self.protocol_class & 0x0F),
        ];
        buf.extend_from_slice(&encode_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for UnitData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UDT [class={}, called={}, calling={}, data_len={}]",
            self.protocol_class,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// SCCP Unitdata Service (UDTS, type `0x0A`) — the error response returned when a
/// UDT cannot be delivered.
///
/// The wire layout mirrors UDT, with a [`ReturnCause`] octet in place of the
/// protocol-class/message-handling octet:
///
/// ```ignore
/// 0: Message type (0x0A)
/// 1: Return cause
/// 2: Pointer to called party address
/// 3: Pointer to calling party address
/// 4: Pointer to data
/// Variable: Called party address (length-prefixed)
/// Variable: Calling party address (length-prefixed)
/// Variable: Data (length-prefixed)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDataService {
    /// Why the original UDT could not be delivered.
    pub return_cause: ReturnCause,
    /// Called (destination) party address, copied from the returned UDT.
    pub called_party: SccpAddress,
    /// Calling (source) party address, copied from the returned UDT.
    pub calling_party: SccpAddress,
    /// The returned user data.
    pub data: Vec<u8>,
}

impl UnitDataService {
    /// Build a UDTS with the given return cause and returned addresses/data.
    pub fn new(
        return_cause: ReturnCause,
        called_party: SccpAddress,
        calling_party: SccpAddress,
        data: Vec<u8>,
    ) -> Self {
        Self {
            return_cause,
            called_party,
            calling_party,
            data,
        }
    }

    /// Decode a UDTS message from raw bytes (including the leading message-type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.len() < 5 {
            return Err(SccpError::TooShort {
                expected: 5,
                actual: bytes.len(),
            });
        }

        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Udts {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }

        let return_cause = ReturnCause::from_u8(bytes[1]);

        // Variable part: the three pointers begin at byte 2 (after type + cause).
        let (called_party, calling_party, data) = decode_variable_part(bytes, 2)?;

        Ok(Self {
            return_cause,
            called_party,
            calling_party,
            data,
        })
    }

    /// Encode this UDTS to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![MessageType::Udts as u8, self.return_cause.value()];
        buf.extend_from_slice(&encode_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for UnitDataService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "UDTS [cause={}, called={}, calling={}, data_len={}]",
            self.return_cause,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// SCCP Extended Unitdata (XUDT, type `0x11`) — connectionless data transfer
/// carrying a **hop counter** (ITU-T Q.713 §4) and an optional parameter part.
///
/// ```ignore
/// 0: Message type (0x11)
/// 1: Protocol class + message handling
/// 2: Hop counter
/// 3: Pointer to called party address
/// 4: Pointer to calling party address
/// 5: Pointer to data
/// 6: Pointer to optional part (0 = none)
/// Variable: called / calling / data (each length-prefixed), then optional part
/// ```
///
/// A translating node decrements `hop_counter` at each global-title translation
/// and discards the message when it reaches zero, breaking routing loops.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedUnitData {
    /// Protocol class (0 or 1).
    pub protocol_class: u8,
    /// Message handling (0 = no special options, 8 = return on error).
    pub message_handling: u8,
    /// Hop counter, decremented at each GT translation.
    pub hop_counter: u8,
    /// Called (destination) party address.
    pub called_party: SccpAddress,
    /// Calling (source) party address.
    pub calling_party: SccpAddress,
    /// User data (a TCAP payload, typically).
    pub data: Vec<u8>,
    /// Raw optional parameter part (parameters plus the end-of-optional marker)
    /// exactly as it appears on the wire; empty when absent. Kept opaque so a
    /// transiting node preserves segmentation / importance verbatim.
    pub optional_part: Vec<u8>,
}

/// The hop-counter value a new extended/long message starts with. ITU-T Q.714
/// leaves the initial value to the originating node; 15 is the usual maximum.
pub const DEFAULT_HOP_COUNTER: u8 = 15;

impl ExtendedUnitData {
    /// Build an XUDT with protocol class 0, no special message handling, the
    /// default hop counter and no optional part.
    pub fn new(called_party: SccpAddress, calling_party: SccpAddress, data: Vec<u8>) -> Self {
        Self {
            protocol_class: 0,
            message_handling: 0,
            hop_counter: DEFAULT_HOP_COUNTER,
            called_party,
            calling_party,
            data,
            optional_part: Vec::new(),
        }
    }

    /// Decode an XUDT message from raw bytes (including the leading type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.len() < 7 {
            return Err(SccpError::TooShort {
                expected: 7,
                actual: bytes.len(),
            });
        }
        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Xudt {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }
        let protocol_class = bytes[1] & 0x0F;
        let message_handling = (bytes[1] >> 4) & 0x0F;
        let hop_counter = bytes[2];
        let (called_party, calling_party, data, optional_part) =
            decode_extended_variable_part(bytes, 3)?;
        Ok(Self {
            protocol_class,
            message_handling,
            hop_counter,
            called_party,
            calling_party,
            data,
            optional_part,
        })
    }

    /// Encode this XUDT to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![
            MessageType::Xudt as u8,
            (self.message_handling << 4) | (self.protocol_class & 0x0F),
            self.hop_counter,
        ];
        buf.extend_from_slice(&encode_extended_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
            &self.optional_part,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for ExtendedUnitData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "XUDT [class={}, hop={}, called={}, calling={}, data_len={}]",
            self.protocol_class,
            self.hop_counter,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// SCCP Extended Unitdata Service (XUDTS, type `0x12`) — the error response for
/// an XUDT, with a [`ReturnCause`] in place of the protocol class and its own
/// hop counter. Returned with cause [`ReturnCause::HopCounterViolation`] when a
/// hop counter reaches zero.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtendedUnitDataService {
    /// Why the original XUDT could not be delivered.
    pub return_cause: ReturnCause,
    /// Hop counter.
    pub hop_counter: u8,
    /// Called (destination) party address, copied from the returned XUDT.
    pub called_party: SccpAddress,
    /// Calling (source) party address, copied from the returned XUDT.
    pub calling_party: SccpAddress,
    /// The returned user data.
    pub data: Vec<u8>,
    /// Raw optional parameter part; empty when absent.
    pub optional_part: Vec<u8>,
}

impl ExtendedUnitDataService {
    /// Build an XUDTS with the given return cause, the default hop counter and
    /// no optional part.
    pub fn new(
        return_cause: ReturnCause,
        called_party: SccpAddress,
        calling_party: SccpAddress,
        data: Vec<u8>,
    ) -> Self {
        Self {
            return_cause,
            hop_counter: DEFAULT_HOP_COUNTER,
            called_party,
            calling_party,
            data,
            optional_part: Vec::new(),
        }
    }

    /// Decode an XUDTS message from raw bytes (including the leading type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.len() < 7 {
            return Err(SccpError::TooShort {
                expected: 7,
                actual: bytes.len(),
            });
        }
        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Xudts {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }
        let return_cause = ReturnCause::from_u8(bytes[1]);
        let hop_counter = bytes[2];
        let (called_party, calling_party, data, optional_part) =
            decode_extended_variable_part(bytes, 3)?;
        Ok(Self {
            return_cause,
            hop_counter,
            called_party,
            calling_party,
            data,
            optional_part,
        })
    }

    /// Encode this XUDTS to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![
            MessageType::Xudts as u8,
            self.return_cause.value(),
            self.hop_counter,
        ];
        buf.extend_from_slice(&encode_extended_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
            &self.optional_part,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for ExtendedUnitDataService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "XUDTS [cause={}, hop={}, called={}, calling={}, data_len={}]",
            self.return_cause,
            self.hop_counter,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// SCCP Long Unitdata (LUDT, type `0x13`) — like XUDT but with two-octet
/// pointers and a two-octet data length, so it can carry user data past the
/// ~255-octet UDT/XUDT ceiling (ITU-T Q.713 §4.3). Also carries a hop counter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongUnitData {
    /// Protocol class (0 or 1).
    pub protocol_class: u8,
    /// Message handling (0 = no special options, 8 = return on error).
    pub message_handling: u8,
    /// Hop counter, decremented at each GT translation.
    pub hop_counter: u8,
    /// Called (destination) party address.
    pub called_party: SccpAddress,
    /// Calling (source) party address.
    pub calling_party: SccpAddress,
    /// User data (may exceed 255 octets).
    pub data: Vec<u8>,
    /// Raw optional parameter part; empty when absent.
    pub optional_part: Vec<u8>,
}

impl LongUnitData {
    /// Build an LUDT with protocol class 0, no special message handling, the
    /// default hop counter and no optional part.
    pub fn new(called_party: SccpAddress, calling_party: SccpAddress, data: Vec<u8>) -> Self {
        Self {
            protocol_class: 0,
            message_handling: 0,
            hop_counter: DEFAULT_HOP_COUNTER,
            called_party,
            calling_party,
            data,
            optional_part: Vec::new(),
        }
    }

    /// Decode an LUDT message from raw bytes (including the leading type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.is_empty() {
            return Err(SccpError::TooShort {
                expected: 1,
                actual: 0,
            });
        }
        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Ludt {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }
        // decode_long_variable_part enforces the 11-octet minimum, so the fixed
        // class/hop octets at 1 and 2 are in range once it returns.
        let (called_party, calling_party, data, optional_part) = decode_long_variable_part(bytes)?;
        let protocol_class = bytes[1] & 0x0F;
        let message_handling = (bytes[1] >> 4) & 0x0F;
        let hop_counter = bytes[2];
        Ok(Self {
            protocol_class,
            message_handling,
            hop_counter,
            called_party,
            calling_party,
            data,
            optional_part,
        })
    }

    /// Encode this LUDT to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![
            MessageType::Ludt as u8,
            (self.message_handling << 4) | (self.protocol_class & 0x0F),
            self.hop_counter,
        ];
        buf.extend_from_slice(&encode_long_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
            &self.optional_part,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for LongUnitData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LUDT [class={}, hop={}, called={}, calling={}, data_len={}]",
            self.protocol_class,
            self.hop_counter,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// SCCP Long Unitdata Service (LUDTS, type `0x14`) — the error response for an
/// LUDT, with a [`ReturnCause`] in place of the protocol class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LongUnitDataService {
    /// Why the original LUDT could not be delivered.
    pub return_cause: ReturnCause,
    /// Hop counter.
    pub hop_counter: u8,
    /// Called (destination) party address, copied from the returned LUDT.
    pub called_party: SccpAddress,
    /// Calling (source) party address, copied from the returned LUDT.
    pub calling_party: SccpAddress,
    /// The returned user data.
    pub data: Vec<u8>,
    /// Raw optional parameter part; empty when absent.
    pub optional_part: Vec<u8>,
}

impl LongUnitDataService {
    /// Build an LUDTS with the given return cause, the default hop counter and
    /// no optional part.
    pub fn new(
        return_cause: ReturnCause,
        called_party: SccpAddress,
        calling_party: SccpAddress,
        data: Vec<u8>,
    ) -> Self {
        Self {
            return_cause,
            hop_counter: DEFAULT_HOP_COUNTER,
            called_party,
            calling_party,
            data,
            optional_part: Vec::new(),
        }
    }

    /// Decode an LUDTS message from raw bytes (including the leading type octet).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.is_empty() {
            return Err(SccpError::TooShort {
                expected: 1,
                actual: 0,
            });
        }
        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Ludts {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }
        let (called_party, calling_party, data, optional_part) = decode_long_variable_part(bytes)?;
        let return_cause = ReturnCause::from_u8(bytes[1]);
        let hop_counter = bytes[2];
        Ok(Self {
            return_cause,
            hop_counter,
            called_party,
            calling_party,
            data,
            optional_part,
        })
    }

    /// Encode this LUDTS to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let mut buf = vec![
            MessageType::Ludts as u8,
            self.return_cause.value(),
            self.hop_counter,
        ];
        buf.extend_from_slice(&encode_long_variable_part(
            &self.called_party,
            &self.calling_party,
            &self.data,
            &self.optional_part,
        )?);
        Ok(buf)
    }
}

impl fmt::Display for LongUnitDataService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "LUDTS [cause={}, hop={}, called={}, calling={}, data_len={}]",
            self.return_cause,
            self.hop_counter,
            self.called_party,
            self.calling_party,
            self.data.len()
        )
    }
}

/// Top-level SCCP message enum for the connectionless message types this codec
/// decodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SccpMessage {
    /// A Unitdata (UDT) message.
    Udt(UnitData),
    /// A Unitdata Service (UDTS) message.
    Udts(UnitDataService),
    /// An Extended Unitdata (XUDT) message.
    Xudt(ExtendedUnitData),
    /// An Extended Unitdata Service (XUDTS) message.
    Xudts(ExtendedUnitDataService),
    /// A Long Unitdata (LUDT) message.
    Ludt(LongUnitData),
    /// A Long Unitdata Service (LUDTS) message.
    Ludts(LongUnitDataService),
}

impl SccpMessage {
    /// Decode an SCCP message from raw bytes, dispatching on the message-type octet.
    ///
    /// The connectionless types (UDT, UDTS, XUDT, XUDTS, LUDT, LUDTS) are
    /// decoded; any other type yields [`SccpError::InvalidMessageType`].
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.is_empty() {
            return Err(SccpError::TooShort {
                expected: 1,
                actual: 0,
            });
        }

        let msg_type =
            MessageType::from_u8(bytes[0]).ok_or(SccpError::InvalidMessageType(bytes[0]))?;

        match msg_type {
            MessageType::Udt => Ok(Self::Udt(UnitData::decode(bytes)?)),
            MessageType::Udts => Ok(Self::Udts(UnitDataService::decode(bytes)?)),
            MessageType::Xudt => Ok(Self::Xudt(ExtendedUnitData::decode(bytes)?)),
            MessageType::Xudts => Ok(Self::Xudts(ExtendedUnitDataService::decode(bytes)?)),
            MessageType::Ludt => Ok(Self::Ludt(LongUnitData::decode(bytes)?)),
            MessageType::Ludts => Ok(Self::Ludts(LongUnitDataService::decode(bytes)?)),
            _ => Err(SccpError::InvalidMessageType(bytes[0])),
        }
    }

    /// Encode this message to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        match self {
            Self::Udt(udt) => udt.encode(),
            Self::Udts(udts) => udts.encode(),
            Self::Xudt(xudt) => xudt.encode(),
            Self::Xudts(xudts) => xudts.encode(),
            Self::Ludt(ludt) => ludt.encode(),
            Self::Ludts(ludts) => ludts.encode(),
        }
    }

    /// The called (destination) party address, whichever connectionless type
    /// this is.
    pub fn called_party(&self) -> &SccpAddress {
        match self {
            Self::Udt(m) => &m.called_party,
            Self::Udts(m) => &m.called_party,
            Self::Xudt(m) => &m.called_party,
            Self::Xudts(m) => &m.called_party,
            Self::Ludt(m) => &m.called_party,
            Self::Ludts(m) => &m.called_party,
        }
    }

    /// The calling (source) party address.
    pub fn calling_party(&self) -> &SccpAddress {
        match self {
            Self::Udt(m) => &m.calling_party,
            Self::Udts(m) => &m.calling_party,
            Self::Xudt(m) => &m.calling_party,
            Self::Xudts(m) => &m.calling_party,
            Self::Ludt(m) => &m.calling_party,
            Self::Ludts(m) => &m.calling_party,
        }
    }

    /// The user data.
    pub fn data(&self) -> &[u8] {
        match self {
            Self::Udt(m) => &m.data,
            Self::Udts(m) => &m.data,
            Self::Xudt(m) => &m.data,
            Self::Xudts(m) => &m.data,
            Self::Ludt(m) => &m.data,
            Self::Ludts(m) => &m.data,
        }
    }

    /// The hop counter, for the types that carry one (XUDT/XUDTS/LUDT/LUDTS);
    /// `None` for UDT/UDTS, which have no hop counter.
    pub fn hop_counter(&self) -> Option<u8> {
        match self {
            Self::Udt(_) | Self::Udts(_) => None,
            Self::Xudt(m) => Some(m.hop_counter),
            Self::Xudts(m) => Some(m.hop_counter),
            Self::Ludt(m) => Some(m.hop_counter),
            Self::Ludts(m) => Some(m.hop_counter),
        }
    }
}

impl fmt::Display for SccpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Udt(udt) => write!(f, "{udt}"),
            Self::Udts(udts) => write!(f, "{udts}"),
            Self::Xudt(xudt) => write!(f, "{xudt}"),
            Self::Xudts(xudts) => write!(f, "{xudts}"),
            Self::Ludt(ludt) => write!(f, "{ludt}"),
            Self::Ludts(ludts) => write!(f, "{ludts}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::global_title::GlobalTitle;
    use crate::types::SubsystemNumber;

    #[test]
    fn udt_round_trip_ssn() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let data = vec![0x62, 0x40, 0x01, 0x02, 0x03]; // fake TCAP

        let udt = UnitData::new(called.clone(), calling.clone(), data.clone());
        let encoded = udt.encode().unwrap();
        let decoded = UnitData::decode(&encoded).unwrap();

        assert_eq!(decoded.called_party, called);
        assert_eq!(decoded.calling_party, calling);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn udt_round_trip_gt() {
        let called_gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "15551234567".to_string(),
        };
        let calling_gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "15559876543".to_string(),
        };

        let called = SccpAddress::with_gt(called_gt, Some(SubsystemNumber::Hlr));
        let calling = SccpAddress::with_gt(calling_gt, Some(SubsystemNumber::Msc));
        let data = vec![0x62, 0x40];

        let udt = UnitData::new(called.clone(), calling.clone(), data.clone());
        let encoded = udt.encode().unwrap();
        let decoded = UnitData::decode(&encoded).unwrap();

        assert_eq!(decoded.called_party, called);
        assert_eq!(decoded.calling_party, calling);
        assert_eq!(decoded.data, data);
    }

    #[test]
    fn sccp_message_decode() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let udt = UnitData::new(called, calling, vec![0x01, 0x02]);
        let encoded = udt.encode().unwrap();

        let msg = SccpMessage::decode(&encoded).unwrap();
        match msg {
            SccpMessage::Udt(decoded_udt) => {
                assert_eq!(decoded_udt.data, vec![0x01, 0x02]);
            }
            _ => panic!("Expected UDT"),
        }
    }

    #[test]
    fn udt_display() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let udt = UnitData::new(called, calling, vec![0x01]);
        let s = format!("{udt}");
        assert!(s.contains("UDT"));
        assert!(s.contains("HLR"));
    }

    #[test]
    fn udts_round_trip() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, Some(100));
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, Some(200));
        let udts = UnitDataService::new(
            ReturnCause::SubsystemFailure,
            called.clone(),
            calling.clone(),
            vec![0x62, 0x40],
        );

        let encoded = udts.encode().unwrap();
        assert_eq!(encoded[0], MessageType::Udts as u8);
        let decoded = UnitDataService::decode(&encoded).unwrap();
        assert_eq!(decoded, udts);
        assert_eq!(decoded.return_cause, ReturnCause::SubsystemFailure);
    }

    #[test]
    fn sccp_message_dispatches_udts() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let udts = UnitDataService::new(ReturnCause::Unequipped, called, calling, vec![0xAA]);
        let encoded = udts.encode().unwrap();

        match SccpMessage::decode(&encoded).unwrap() {
            SccpMessage::Udts(d) => assert_eq!(d.return_cause, ReturnCause::Unequipped),
            other => panic!("expected UDTS, got {other:?}"),
        }
    }

    #[test]
    fn sccp_message_round_trip_via_enum() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let msg = SccpMessage::Udt(UnitData::new(called, calling, vec![0x01, 0x02]));
        let encoded = msg.encode().unwrap();
        assert_eq!(SccpMessage::decode(&encoded).unwrap(), msg);
    }

    #[test]
    fn decode_empty_is_too_short() {
        assert!(matches!(
            SccpMessage::decode(&[]),
            Err(SccpError::TooShort { .. })
        ));
    }

    #[test]
    fn decode_unknown_type_is_invalid() {
        // 0xFF is not a valid SCCP message type.
        assert!(matches!(
            SccpMessage::decode(&[0xFF, 0, 0, 0, 0]),
            Err(SccpError::InvalidMessageType(0xFF))
        ));
    }

    #[test]
    fn decode_known_but_unsupported_type_is_invalid() {
        // CR (0x01) is a valid type but not decoded by this connectionless codec.
        assert!(matches!(
            SccpMessage::decode(&[MessageType::Cr as u8, 0, 0, 0, 0]),
            Err(SccpError::InvalidMessageType(_))
        ));
    }

    #[test]
    fn udt_decode_truncated_variable_part() {
        // Valid UDT header claiming a called-party pointer past the buffer end.
        let bytes = [MessageType::Udt as u8, 0x00, 0x7F, 0x03, 0x04];
        assert!(matches!(
            UnitData::decode(&bytes),
            Err(SccpError::TooShort { .. })
        ));
    }

    #[test]
    fn udts_decode_wrong_type() {
        // A well-formed UDT is not a UDTS.
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let udt = UnitData::new(called, calling, vec![0x01]);
        let encoded = udt.encode().unwrap();
        assert!(matches!(
            UnitDataService::decode(&encoded),
            Err(SccpError::InvalidMessageType(_))
        ));
    }

    // ── XUDT / XUDTS / LUDT / LUDTS ──────────────────────────────────────────
    // The encode vectors below are known-answer vectors: each was dissected
    // clean by the Wireshark (tshark) ITU-T Q.713 SCCP dissector — message type,
    // hop counter, addresses and (for XUDTS) return cause all as asserted — so
    // they check the wire layout against an independent oracle, not a round-trip.

    #[test]
    fn xudt_encode_matches_q713_vector() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let xudt = ExtendedUnitData::new(called, calling, vec![0x62, 0x40]);
        // type 0x11, class 0, hop 0x0f, ptrs 04 06 08 00, called SSN=HLR,
        // calling SSN=MSC, data 62 40.
        let expected = [
            0x11, 0x00, 0x0F, 0x04, 0x06, 0x08, 0x00, 0x02, 0x42, 0x06, 0x02, 0x42, 0x08, 0x02,
            0x62, 0x40,
        ];
        assert_eq!(xudt.encode().unwrap(), expected);
        assert_eq!(ExtendedUnitData::decode(&expected).unwrap(), xudt);
        assert_eq!(xudt.hop_counter, DEFAULT_HOP_COUNTER);
    }

    #[test]
    fn xudt_optional_part_preserved() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let mut xudt = ExtendedUnitData::new(called, calling, vec![0x62, 0x40]);
        // Importance parameter (0x12 len 1 value 3) + end-of-optional (0x00).
        xudt.optional_part = vec![0x12, 0x01, 0x03, 0x00];
        let expected = [
            0x11, 0x00, 0x0F, 0x04, 0x06, 0x08, 0x0A, 0x02, 0x42, 0x06, 0x02, 0x42, 0x08, 0x02,
            0x62, 0x40, 0x12, 0x01, 0x03, 0x00,
        ];
        assert_eq!(xudt.encode().unwrap(), expected);
        let decoded = ExtendedUnitData::decode(&expected).unwrap();
        assert_eq!(decoded.optional_part, vec![0x12, 0x01, 0x03, 0x00]);
        assert_eq!(decoded, xudt);
    }

    #[test]
    fn xudts_hop_counter_violation_uses_q713_cause_0x0c() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let xudts = ExtendedUnitDataService::new(
            ReturnCause::HopCounterViolation,
            called,
            calling,
            vec![0x62, 0x40],
        );
        let expected = [
            0x12, 0x0C, 0x0F, 0x04, 0x06, 0x08, 0x00, 0x02, 0x42, 0x06, 0x02, 0x42, 0x08, 0x02,
            0x62, 0x40,
        ];
        assert_eq!(xudts.encode().unwrap(), expected);
        assert_eq!(ExtendedUnitDataService::decode(&expected).unwrap(), xudts);
        // Q.713 §3.12: hop counter violation is 0x0C, not 0x0D (which is
        // "segmentation not supported").
        assert_eq!(expected[1], 0x0C);
        assert_eq!(ReturnCause::HopCounterViolation.value(), 0x0C);
    }

    #[test]
    fn ludt_encode_matches_q713_vector() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let ludt = LongUnitData::new(called, calling, vec![0x62, 0x40]);
        // type 0x13, class 0, hop 0x0f, two-octet LE pointers 7/8/9/0, SSN
        // addresses, two-octet LE data length 02 00, data 62 40.
        let expected = [
            0x13, 0x00, 0x0F, 0x07, 0x00, 0x08, 0x00, 0x09, 0x00, 0x00, 0x00, 0x02, 0x42, 0x06,
            0x02, 0x42, 0x08, 0x02, 0x00, 0x62, 0x40,
        ];
        assert_eq!(ludt.encode().unwrap(), expected);
        assert_eq!(LongUnitData::decode(&expected).unwrap(), ludt);
        assert_eq!(ludt.hop_counter, DEFAULT_HOP_COUNTER);
    }

    #[test]
    fn ludts_round_trips_through_enum() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let ludts = LongUnitDataService::new(
            ReturnCause::HopCounterViolation,
            called,
            calling,
            vec![0x62, 0x40],
        );
        let expected = [
            0x14, 0x0C, 0x0F, 0x07, 0x00, 0x08, 0x00, 0x09, 0x00, 0x00, 0x00, 0x02, 0x42, 0x06,
            0x02, 0x42, 0x08, 0x02, 0x00, 0x62, 0x40,
        ];
        assert_eq!(ludts.encode().unwrap(), expected);
        match SccpMessage::decode(&expected).unwrap() {
            SccpMessage::Ludts(d) => assert_eq!(d, ludts),
            other => panic!("expected LUDTS, got {other:?}"),
        }
    }

    #[test]
    fn ludt_carries_data_beyond_the_udt_ceiling() {
        // 600 octets of data cannot fit a one-octet UDT/XUDT length; the LUDT
        // two-octet length carries it.
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let data = vec![0xAB; 600];
        let ludt = LongUnitData::new(called, calling, data.clone());
        let decoded = LongUnitData::decode(&ludt.encode().unwrap()).unwrap();
        assert_eq!(decoded.data, data);
        assert_eq!(decoded, ludt);
    }

    #[test]
    fn xudt_and_ludt_gt_round_trip() {
        let gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "15551234567".to_string(),
        };
        let called = SccpAddress::with_gt(gt.clone(), Some(SubsystemNumber::Hlr));
        let calling = SccpAddress::with_gt(gt, Some(SubsystemNumber::Msc));

        let xudt = ExtendedUnitData::new(called.clone(), calling.clone(), vec![0x62, 0x40]);
        assert_eq!(
            ExtendedUnitData::decode(&xudt.encode().unwrap()).unwrap(),
            xudt
        );

        let ludt = LongUnitData::new(called, calling, vec![0x62, 0x40]);
        assert_eq!(LongUnitData::decode(&ludt.encode().unwrap()).unwrap(), ludt);
    }

    #[test]
    fn sccp_message_accessors_and_dispatch() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let encoded = ExtendedUnitData::new(called, calling, vec![0x01, 0x02])
            .encode()
            .unwrap();
        let msg = SccpMessage::decode(&encoded).unwrap();
        assert!(matches!(msg, SccpMessage::Xudt(_)));
        assert_eq!(msg.data(), &[0x01, 0x02]);
        assert_eq!(msg.hop_counter(), Some(DEFAULT_HOP_COUNTER));
        assert_eq!(msg.called_party().ssn, Some(SubsystemNumber::Hlr));
        assert_eq!(msg.calling_party().ssn, Some(SubsystemNumber::Msc));
    }

    #[test]
    fn udt_has_no_hop_counter() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let msg = SccpMessage::Udt(UnitData::new(called, calling, vec![]));
        assert_eq!(msg.hop_counter(), None);
    }

    #[test]
    fn extended_and_long_decode_wrong_type() {
        let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
        let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let udt = UnitData::new(called, calling, vec![0x01]).encode().unwrap();
        assert!(matches!(
            ExtendedUnitData::decode(&udt),
            Err(SccpError::InvalidMessageType(_))
        ));
        assert!(matches!(
            LongUnitData::decode(&udt),
            Err(SccpError::InvalidMessageType(_))
        ));
    }

    #[test]
    fn extended_and_long_decode_truncated() {
        assert!(matches!(
            ExtendedUnitData::decode(&[0x11, 0x00, 0x0F]),
            Err(SccpError::TooShort { .. })
        ));
        assert!(matches!(
            LongUnitData::decode(&[0x13, 0x00, 0x0F, 0x07]),
            Err(SccpError::TooShort { .. })
        ));
    }
}
