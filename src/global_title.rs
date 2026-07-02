//! Global Title (GT) types: the [`GtIndicator`] field and the five
//! [`GlobalTitle`] formats, with TBCD-coded digits.

use std::fmt;

use crate::bcd;
use crate::error::SccpError;

/// Global Title Indicator values (bits 2-5 of Address Indicator).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GtIndicator {
    /// No Global Title included.
    NoGt = 0,
    /// GT includes Nature of Address only.
    Gt0001 = 1,
    /// GT includes Translation Type only.
    Gt0010 = 2,
    /// GT includes Translation Type, Numbering Plan, Encoding Scheme.
    Gt0011 = 3,
    /// GT includes Translation Type, Numbering Plan, Encoding Scheme, Nature of Address.
    Gt0100 = 4,
}

impl GtIndicator {
    /// Map the 4-bit GT-indicator field to a [`GtIndicator`], or error on a value above 4.
    pub fn from_u8(value: u8) -> Result<Self, SccpError> {
        match value {
            0 => Ok(Self::NoGt),
            1 => Ok(Self::Gt0001),
            2 => Ok(Self::Gt0010),
            3 => Ok(Self::Gt0011),
            4 => Ok(Self::Gt0100),
            other => Err(SccpError::InvalidGtIndicator(other)),
        }
    }
}

/// Global Title variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlobalTitle {
    /// No Global Title.
    NoTitle,
    /// GT format 0001: Nature of Address Indicator + digits.
    Gt0001 {
        /// Nature of Address Indicator (7 bits).
        nature_of_address: u8,
        /// Odd/even indicator for the digit count.
        odd_even: bool,
        /// Address digits (decoded from TBCD).
        digits: String,
    },
    /// GT format 0010: Translation Type + digits.
    Gt0010 {
        /// Translation type.
        translation_type: u8,
        /// Address digits (decoded from TBCD).
        digits: String,
    },
    /// GT format 0011: Translation Type + Numbering Plan + Encoding Scheme + digits.
    Gt0011 {
        /// Translation type.
        translation_type: u8,
        /// Numbering plan (4 bits).
        numbering_plan: u8,
        /// Encoding scheme (4 bits).
        encoding_scheme: u8,
        /// Address digits (decoded from TBCD).
        digits: String,
    },
    /// GT format 0100: Translation Type + Numbering Plan + Encoding Scheme + Nature of Address + digits.
    Gt0100 {
        /// Translation type.
        translation_type: u8,
        /// Numbering plan (4 bits).
        numbering_plan: u8,
        /// Encoding scheme (4 bits).
        encoding_scheme: u8,
        /// Nature of Address Indicator (7 bits).
        nature_of_address: u8,
        /// Address digits (decoded from TBCD).
        digits: String,
    },
}

impl GlobalTitle {
    /// The [`GtIndicator`] corresponding to this Global Title variant.
    pub fn indicator(&self) -> GtIndicator {
        match self {
            Self::NoTitle => GtIndicator::NoGt,
            Self::Gt0001 { .. } => GtIndicator::Gt0001,
            Self::Gt0010 { .. } => GtIndicator::Gt0010,
            Self::Gt0011 { .. } => GtIndicator::Gt0011,
            Self::Gt0100 { .. } => GtIndicator::Gt0100,
        }
    }

    /// The decoded address digits, or `None` for [`GlobalTitle::NoTitle`].
    pub fn digits(&self) -> Option<&str> {
        match self {
            Self::NoTitle => None,
            Self::Gt0001 { digits, .. }
            | Self::Gt0010 { digits, .. }
            | Self::Gt0011 { digits, .. }
            | Self::Gt0100 { digits, .. } => Some(digits),
        }
    }

    /// Decode a Global Title from bytes given the GT indicator.
    pub fn decode(bytes: &[u8], gti: GtIndicator) -> Result<Self, SccpError> {
        match gti {
            GtIndicator::NoGt => Ok(Self::NoTitle),
            GtIndicator::Gt0001 => {
                if bytes.is_empty() {
                    return Err(SccpError::TooShort {
                        expected: 1,
                        actual: 0,
                    });
                }
                let odd_even = (bytes[0] >> 7) & 1 == 1;
                let nature_of_address = bytes[0] & 0x7F;
                let digits = bcd::decode_tbcd(&bytes[1..]);
                Ok(Self::Gt0001 {
                    nature_of_address,
                    odd_even,
                    digits,
                })
            }
            GtIndicator::Gt0010 => {
                if bytes.is_empty() {
                    return Err(SccpError::TooShort {
                        expected: 1,
                        actual: 0,
                    });
                }
                let translation_type = bytes[0];
                let digits = bcd::decode_tbcd(&bytes[1..]);
                Ok(Self::Gt0010 {
                    translation_type,
                    digits,
                })
            }
            GtIndicator::Gt0011 => {
                if bytes.len() < 2 {
                    return Err(SccpError::TooShort {
                        expected: 2,
                        actual: bytes.len(),
                    });
                }
                let translation_type = bytes[0];
                let numbering_plan = (bytes[1] >> 4) & 0x0F;
                let encoding_scheme = bytes[1] & 0x0F;
                let digits = bcd::decode_tbcd(&bytes[2..]);
                Ok(Self::Gt0011 {
                    translation_type,
                    numbering_plan,
                    encoding_scheme,
                    digits,
                })
            }
            GtIndicator::Gt0100 => {
                if bytes.len() < 3 {
                    return Err(SccpError::TooShort {
                        expected: 3,
                        actual: bytes.len(),
                    });
                }
                let translation_type = bytes[0];
                let numbering_plan = (bytes[1] >> 4) & 0x0F;
                let encoding_scheme = bytes[1] & 0x0F;
                let nature_of_address = bytes[2] & 0x7F;
                let digits = bcd::decode_tbcd(&bytes[3..]);
                Ok(Self::Gt0100 {
                    translation_type,
                    numbering_plan,
                    encoding_scheme,
                    nature_of_address,
                    digits,
                })
            }
        }
    }

    /// Encode a Global Title to bytes.
    pub fn encode(&self) -> Result<Vec<u8>, SccpError> {
        match self {
            Self::NoTitle => Ok(vec![]),
            Self::Gt0001 {
                nature_of_address,
                odd_even,
                digits,
            } => {
                let first_byte = (if *odd_even { 0x80 } else { 0x00 }) | (nature_of_address & 0x7F);
                let mut buf = vec![first_byte];
                buf.extend_from_slice(&bcd::encode_tbcd(digits)?);
                Ok(buf)
            }
            Self::Gt0010 {
                translation_type,
                digits,
            } => {
                let mut buf = vec![*translation_type];
                buf.extend_from_slice(&bcd::encode_tbcd(digits)?);
                Ok(buf)
            }
            Self::Gt0011 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                digits,
            } => {
                let mut buf = vec![
                    *translation_type,
                    (numbering_plan << 4) | (encoding_scheme & 0x0F),
                ];
                buf.extend_from_slice(&bcd::encode_tbcd(digits)?);
                Ok(buf)
            }
            Self::Gt0100 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                nature_of_address,
                digits,
            } => {
                let mut buf = vec![
                    *translation_type,
                    (numbering_plan << 4) | (encoding_scheme & 0x0F),
                    nature_of_address & 0x7F,
                ];
                buf.extend_from_slice(&bcd::encode_tbcd(digits)?);
                Ok(buf)
            }
        }
    }
}

impl fmt::Display for GlobalTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTitle => write!(f, "NoGT"),
            Self::Gt0001 {
                nature_of_address,
                digits,
                ..
            } => write!(f, "GT0001 [noa={nature_of_address}, digits={digits}]"),
            Self::Gt0010 {
                translation_type,
                digits,
            } => write!(f, "GT0010 [tt={translation_type}, digits={digits}]"),
            Self::Gt0011 {
                translation_type,
                numbering_plan,
                digits,
                ..
            } => write!(
                f,
                "GT0011 [tt={translation_type}, np={numbering_plan}, digits={digits}]"
            ),
            Self::Gt0100 {
                translation_type,
                numbering_plan,
                nature_of_address,
                digits,
                ..
            } => write!(
                f,
                "GT0100 [tt={translation_type}, np={numbering_plan}, noa={nature_of_address}, digits={digits}]"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gt0100_round_trip() {
        let gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,  // E.164
            encoding_scheme: 1, // BCD odd
            nature_of_address: 4,
            digits: "15551234567".to_string(),
        };

        let encoded = gt.encode().unwrap();
        let decoded = GlobalTitle::decode(&encoded, GtIndicator::Gt0100).unwrap();

        match &decoded {
            GlobalTitle::Gt0100 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                nature_of_address,
                digits,
            } => {
                assert_eq!(*translation_type, 0);
                assert_eq!(*numbering_plan, 1);
                assert_eq!(*encoding_scheme, 1);
                assert_eq!(*nature_of_address, 4);
                assert_eq!(digits, "15551234567");
            }
            _ => panic!("Expected Gt0100"),
        }
    }

    #[test]
    fn gt0001_round_trip() {
        let gt = GlobalTitle::Gt0001 {
            nature_of_address: 4,
            odd_even: true,
            digits: "12345".to_string(),
        };
        let encoded = gt.encode().unwrap();
        let decoded = GlobalTitle::decode(&encoded, GtIndicator::Gt0001).unwrap();
        assert_eq!(decoded.digits(), Some("12345"));
    }

    #[test]
    fn no_title() {
        let gt = GlobalTitle::NoTitle;
        let encoded = gt.encode().unwrap();
        assert!(encoded.is_empty());
        let decoded = GlobalTitle::decode(&[], GtIndicator::NoGt).unwrap();
        assert_eq!(decoded, GlobalTitle::NoTitle);
    }

    #[test]
    fn display() {
        let gt = GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            nature_of_address: 4,
            digits: "15551234567".to_string(),
        };
        let s = format!("{gt}");
        assert!(s.contains("15551234567"));
        assert!(s.contains("GT0100"));
    }

    #[test]
    fn indicator_matches_variant() {
        assert_eq!(GlobalTitle::NoTitle.indicator(), GtIndicator::NoGt);
        let gt = GlobalTitle::Gt0011 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 1,
            digits: "5550100".to_string(),
        };
        assert_eq!(gt.indicator(), GtIndicator::Gt0011);
    }

    #[test]
    fn gt0010_round_trip() {
        let gt = GlobalTitle::Gt0010 {
            translation_type: 9,
            digits: "5550199".to_string(),
        };
        let decoded = GlobalTitle::decode(&gt.encode().unwrap(), GtIndicator::Gt0010).unwrap();
        assert_eq!(decoded, gt);
    }

    #[test]
    fn gt0011_round_trip() {
        let gt = GlobalTitle::Gt0011 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme: 2,
            digits: "5550142".to_string(),
        };
        let decoded = GlobalTitle::decode(&gt.encode().unwrap(), GtIndicator::Gt0011).unwrap();
        assert_eq!(decoded, gt);
    }

    #[test]
    fn indicator_from_u8_rejects_out_of_range() {
        assert!(matches!(
            GtIndicator::from_u8(5),
            Err(SccpError::InvalidGtIndicator(5))
        ));
        assert_eq!(GtIndicator::from_u8(0).unwrap(), GtIndicator::NoGt);
    }

    #[test]
    fn decode_truncated_gt_headers() {
        // Each format needs at least its fixed header bytes before the digits.
        assert!(matches!(
            GlobalTitle::decode(&[], GtIndicator::Gt0001),
            Err(SccpError::TooShort { .. })
        ));
        assert!(matches!(
            GlobalTitle::decode(&[0x00], GtIndicator::Gt0011),
            Err(SccpError::TooShort { .. })
        ));
        assert!(matches!(
            GlobalTitle::decode(&[0x00, 0x11], GtIndicator::Gt0100),
            Err(SccpError::TooShort { .. })
        ));
    }
}
