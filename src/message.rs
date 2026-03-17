use std::fmt;

use crate::address::SccpAddress;
use crate::error::SccpError;
use crate::types::{MessageType, ReturnCause};

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
    pub fn new(
        called_party: SccpAddress,
        calling_party: SccpAddress,
        data: Vec<u8>,
    ) -> Self {
        Self {
            protocol_class: 0,
            message_handling: 0,
            called_party,
            calling_party,
            data,
        }
    }

    /// Decode a UDT message from bytes (after message type byte).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        // bytes[0] = message type (already consumed by caller or included)
        if bytes.len() < 5 {
            return Err(SccpError::TooShort {
                expected: 5,
                actual: bytes.len(),
            });
        }

        let msg_type = MessageType::from_u8(bytes[0])
            .ok_or(SccpError::InvalidMessageType(bytes[0]))?;
        if msg_type != MessageType::Udt {
            return Err(SccpError::InvalidMessageType(bytes[0]));
        }

        let protocol_class = bytes[1] & 0x0F;
        let message_handling = (bytes[1] >> 4) & 0x0F;

        // Pointers are relative to their own position
        let ptr_called = bytes[2] as usize;
        let ptr_calling = bytes[3] as usize;
        let ptr_data = bytes[4] as usize;

        // Absolute offsets (pointer base is the pointer's position)
        let called_offset = 2 + ptr_called;
        let calling_offset = 3 + ptr_calling;
        let data_offset = 4 + ptr_data;

        // Decode called party (length-prefixed)
        if called_offset >= bytes.len() {
            return Err(SccpError::TooShort {
                expected: called_offset + 1,
                actual: bytes.len(),
            });
        }
        let called_len = bytes[called_offset] as usize;
        let called_start = called_offset + 1;
        if called_start + called_len > bytes.len() {
            return Err(SccpError::TooShort {
                expected: called_start + called_len,
                actual: bytes.len(),
            });
        }
        let called_party = SccpAddress::decode(&bytes[called_start..called_start + called_len])?;

        // Decode calling party (length-prefixed)
        if calling_offset >= bytes.len() {
            return Err(SccpError::TooShort {
                expected: calling_offset + 1,
                actual: bytes.len(),
            });
        }
        let calling_len = bytes[calling_offset] as usize;
        let calling_start = calling_offset + 1;
        if calling_start + calling_len > bytes.len() {
            return Err(SccpError::TooShort {
                expected: calling_start + calling_len,
                actual: bytes.len(),
            });
        }
        let calling_party =
            SccpAddress::decode(&bytes[calling_start..calling_start + calling_len])?;

        // Decode data (length-prefixed)
        if data_offset >= bytes.len() {
            return Err(SccpError::TooShort {
                expected: data_offset + 1,
                actual: bytes.len(),
            });
        }
        let data_len = bytes[data_offset] as usize;
        let data_start = data_offset + 1;
        if data_start + data_len > bytes.len() {
            return Err(SccpError::TooShort {
                expected: data_start + data_len,
                actual: bytes.len(),
            });
        }
        let data = bytes[data_start..data_start + data_len].to_vec();

        Ok(Self {
            protocol_class,
            message_handling,
            called_party,
            calling_party,
            data,
        })
    }

    /// Encode to bytes.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let called_bytes = self.called_party.encode()?;
        let calling_bytes = self.calling_party.encode()?;

        // Pointers: each pointer is relative to its own position
        // Position 2: ptr_called → points to called addr length byte
        // Position 3: ptr_calling → points to calling addr length byte
        // Position 4: ptr_data → points to data length byte
        let ptr_called: u8 = 3; // offset from position 2 to position 5
        let ptr_calling: u8 = (3 + 1 + called_bytes.len()) as u8 - 1; // from pos 3
        let ptr_data: u8 = (3 + 1 + called_bytes.len() + 1 + calling_bytes.len()) as u8 - 2; // from pos 4

        let mut buf = vec![
            MessageType::Udt as u8,
            (self.message_handling << 4) | (self.protocol_class & 0x0F),
            ptr_called,
            ptr_calling,
            ptr_data,
            called_bytes.len() as u8,
        ];
        buf.extend_from_slice(&called_bytes);

        // Calling party (length-prefixed)
        buf.push(calling_bytes.len() as u8);
        buf.extend_from_slice(&calling_bytes);

        // Data (length-prefixed)
        buf.push(self.data.len() as u8);
        buf.extend_from_slice(&self.data);

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

/// SCCP Unitdata Service (UDTS) — error response to UDT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnitDataService {
    pub return_cause: ReturnCause,
    pub called_party: SccpAddress,
    pub calling_party: SccpAddress,
    pub data: Vec<u8>,
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

/// Top-level SCCP message enum.
#[derive(Debug, Clone)]
pub enum SccpMessage {
    Udt(UnitData),
    Udts(UnitDataService),
}

impl SccpMessage {
    /// Decode an SCCP message from raw bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.is_empty() {
            return Err(SccpError::TooShort {
                expected: 1,
                actual: 0,
            });
        }

        let msg_type = MessageType::from_u8(bytes[0])
            .ok_or(SccpError::InvalidMessageType(bytes[0]))?;

        match msg_type {
            MessageType::Udt => Ok(Self::Udt(UnitData::decode(bytes)?)),
            _ => Err(SccpError::InvalidMessageType(bytes[0])),
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
            digits: "31612345678".to_string(),
        };
        let calling_gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "31687654321".to_string(),
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
}
