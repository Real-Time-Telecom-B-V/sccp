use std::fmt;

use crate::error::SccpError;
use crate::global_title::{GtIndicator, GlobalTitle};
use crate::types::SubsystemNumber;

/// SCCP Address — Called/Calling Party Address.
///
/// Address Indicator byte layout:
/// ```ignore
/// Bit 0:   Point Code Indicator (1 = PC present)
/// Bit 1:   SSN Indicator (1 = SSN present)
/// Bits 2-5: Global Title Indicator (0-4)
/// Bit 6:   Routing Indicator (0 = route on GT, 1 = route on SSN)
/// Bit 7:   Reserved (national use)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SccpAddress {
    /// Route on GT (false) or SSN (true).
    pub route_on_ssn: bool,
    /// Optional point code (ITU: 2 bytes LE).
    pub point_code: Option<u16>,
    /// Optional Subsystem Number.
    pub ssn: Option<SubsystemNumber>,
    /// Global Title.
    pub global_title: GlobalTitle,
}

impl SccpAddress {
    /// Create an address with GT routing.
    pub fn with_gt(gt: GlobalTitle, ssn: Option<SubsystemNumber>) -> Self {
        Self {
            route_on_ssn: false,
            point_code: None,
            ssn,
            global_title: gt,
        }
    }

    /// Create an address with SSN routing.
    pub fn with_ssn(ssn: SubsystemNumber, point_code: Option<u16>) -> Self {
        Self {
            route_on_ssn: true,
            point_code,
            ssn: Some(ssn),
            global_title: GlobalTitle::NoTitle,
        }
    }

    /// Decode from bytes (length-prefixed in the message, but here we get the address bytes).
    pub fn decode(bytes: &[u8]) -> Result<Self, SccpError> {
        if bytes.is_empty() {
            return Err(SccpError::TooShort {
                expected: 1,
                actual: 0,
            });
        }

        let ai = bytes[0];
        let pc_indicator = ai & 0x01;
        let ssn_indicator = (ai >> 1) & 0x01;
        let gti = GtIndicator::from_u8((ai >> 2) & 0x0F)?;
        let route_on_ssn = (ai >> 6) & 0x01 == 1;

        let mut offset = 1;

        // Point Code (2 bytes, little-endian) if present
        let point_code = if pc_indicator == 1 {
            if bytes.len() < offset + 2 {
                return Err(SccpError::TooShort {
                    expected: offset + 2,
                    actual: bytes.len(),
                });
            }
            let pc = u16::from_le_bytes([bytes[offset], bytes[offset + 1]]);
            offset += 2;
            Some(pc)
        } else {
            None
        };

        // SSN (1 byte) if present
        let ssn = if ssn_indicator == 1 {
            if bytes.len() < offset + 1 {
                return Err(SccpError::TooShort {
                    expected: offset + 1,
                    actual: bytes.len(),
                });
            }
            let ssn = SubsystemNumber::from_u8(bytes[offset]);
            offset += 1;
            Some(ssn)
        } else {
            None
        };

        // Global Title (remaining bytes)
        let global_title = GlobalTitle::decode(&bytes[offset..], gti)?;

        Ok(Self {
            route_on_ssn,
            point_code,
            ssn,
            global_title,
        })
    }

    /// Encode to bytes.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        let gti = self.global_title.indicator();
        let pc_indicator = if self.point_code.is_some() { 1u8 } else { 0 };
        let ssn_indicator = if self.ssn.is_some() { 1u8 } else { 0 };
        let ri = if self.route_on_ssn { 1u8 } else { 0 };

        let ai = pc_indicator
            | (ssn_indicator << 1)
            | ((gti as u8) << 2)
            | (ri << 6);

        let mut buf = vec![ai];

        if let Some(pc) = self.point_code {
            buf.extend_from_slice(&pc.to_le_bytes());
        }

        if let Some(ref ssn) = self.ssn {
            buf.push(ssn.value());
        }

        let gt_bytes = self.global_title.encode()?;
        buf.extend_from_slice(&gt_bytes);

        Ok(buf)
    }
}

impl fmt::Display for SccpAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SccpAddress [")?;
        if self.route_on_ssn {
            write!(f, "route=SSN")?;
        } else {
            write!(f, "route=GT")?;
        }
        if let Some(pc) = self.point_code {
            write!(f, ", pc={pc}")?;
        }
        if let Some(ref ssn) = self.ssn {
            write!(f, ", ssn={ssn}")?;
        }
        if !matches!(self.global_title, GlobalTitle::NoTitle) {
            write!(f, ", gt={}", self.global_title)?;
        }
        write!(f, "]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn address_with_gt_round_trip() {
        let gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "31612345678".to_string(),
        };
        let addr = SccpAddress::with_gt(gt, Some(SubsystemNumber::Hlr));

        let encoded = addr.encode().unwrap();
        let decoded = SccpAddress::decode(&encoded).unwrap();
        assert_eq!(decoded, addr);
    }

    #[test]
    fn address_with_ssn_round_trip() {
        let addr = SccpAddress::with_ssn(SubsystemNumber::Hlr, Some(1234));
        let encoded = addr.encode().unwrap();
        let decoded = SccpAddress::decode(&encoded).unwrap();
        assert_eq!(decoded, addr);
    }

    #[test]
    fn address_ssn_only_no_pc() {
        let addr = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
        let encoded = addr.encode().unwrap();
        let decoded = SccpAddress::decode(&encoded).unwrap();
        assert_eq!(decoded.ssn, Some(SubsystemNumber::Msc));
        assert_eq!(decoded.point_code, None);
        assert!(decoded.route_on_ssn);
    }

    #[test]
    fn display_gt() {
        let gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "31612345678".to_string(),
        };
        let addr = SccpAddress::with_gt(gt, Some(SubsystemNumber::Hlr));
        let s = format!("{addr}");
        assert!(s.contains("route=GT"));
        assert!(s.contains("HLR"));
        assert!(s.contains("31612345678"));
    }

    #[test]
    fn display_ssn() {
        let addr = SccpAddress::with_ssn(SubsystemNumber::Msc, Some(100));
        let s = format!("{addr}");
        assert!(s.contains("route=SSN"));
        assert!(s.contains("MSC"));
    }
}
