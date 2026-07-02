//! SCCP connectionless messages: [`UnitData`] (UDT) and [`UnitDataService`]
//! (UDTS), plus the [`SccpMessage`] dispatch enum.

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

/// Top-level SCCP message enum for the connectionless message types this codec
/// decodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SccpMessage {
    /// A Unitdata (UDT) message.
    Udt(UnitData),
    /// A Unitdata Service (UDTS) message.
    Udts(UnitDataService),
}

impl SccpMessage {
    /// Decode an SCCP message from raw bytes, dispatching on the message-type octet.
    ///
    /// Only the connectionless types (UDT, UDTS) are decoded; any other type
    /// yields [`SccpError::InvalidMessageType`].
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
            _ => Err(SccpError::InvalidMessageType(bytes[0])),
        }
    }

    /// Encode this message to bytes, including the leading message-type octet.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        match self {
            Self::Udt(udt) => udt.encode(),
            Self::Udts(udts) => udts.encode(),
        }
    }
}

impl fmt::Display for SccpMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Udt(udt) => write!(f, "{udt}"),
            Self::Udts(udts) => write!(f, "{udts}"),
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
}
