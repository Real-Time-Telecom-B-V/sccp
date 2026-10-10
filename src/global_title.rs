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
        /// Odd/even indicator as received (`true` = odd). On encode the
        /// indicator is taken from the digit count, not from this field.
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
        /// Encoding scheme (4 bits): 1 is BCD with an odd number of digits, 2
        /// BCD with an even number. On encode either of those two is written
        /// as the one the digit count makes true.
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
        /// Encoding scheme (4 bits): 1 is BCD with an odd number of digits, 2
        /// BCD with an even number. On encode either of those two is written
        /// as the one the digit count makes true.
        encoding_scheme: u8,
        /// Nature of Address Indicator (7 bits).
        nature_of_address: u8,
        /// Address digits (decoded from TBCD).
        digits: String,
    },
}

/// Encoding scheme "BCD, odd number of digits" (Q.713 3.4.2.3.3).
const SCHEME_BCD_ODD: u8 = 1;
/// Encoding scheme "BCD, even number of digits" (Q.713 3.4.2.3.3).
const SCHEME_BCD_EVEN: u8 = 2;

/// Whether `digits` holds an odd number of address signals.
fn is_odd(digits: &str) -> bool {
    digits.len() % 2 == 1
}

/// The encoding scheme to put on the wire: for the two BCD schemes it is the one
/// the digit count makes true, whichever of the two the caller named, because a
/// receiver drops the last half octet on "odd" and keeps it on "even". Any other
/// scheme says nothing about the count and goes out as given.
fn wire_scheme(encoding_scheme: u8, digits: &str) -> u8 {
    match encoding_scheme & 0x0F {
        SCHEME_BCD_ODD | SCHEME_BCD_EVEN if is_odd(digits) => SCHEME_BCD_ODD,
        SCHEME_BCD_ODD | SCHEME_BCD_EVEN => SCHEME_BCD_EVEN,
        other => other,
    }
}

/// Pack BCD address signals for a title that carries its own odd/even
/// indication: the filler after an odd count is 0000 (Q.713 3.4.2.3.1).
fn encode_bcd(digits: &str) -> Result<Vec<u8>, SccpError> {
    let mut bytes = bcd::encode_tbcd(digits)?;
    if is_odd(digits) {
        if let Some(last) = bytes.last_mut() {
            *last &= 0x0F;
        }
    }
    Ok(bytes)
}

/// Pack the digits of a title that names an encoding scheme: BCD rules for the
/// two BCD schemes, the bare half octets otherwise.
fn encode_digits(scheme: u8, digits: &str) -> Result<Vec<u8>, SccpError> {
    match scheme {
        SCHEME_BCD_ODD | SCHEME_BCD_EVEN => encode_bcd(digits),
        _ => bcd::encode_tbcd(digits),
    }
}

/// Unpack BCD address signals. With `odd` set the high half of the last octet is
/// the filler and is not a digit, whatever its value.
fn decode_bcd(bytes: &[u8], odd: bool) -> String {
    match bytes.split_last() {
        Some((last, head)) if odd => {
            let mut digits = bcd::decode_tbcd(head);
            digits.push_str(&bcd::decode_tbcd(&[last | 0xF0]));
            digits
        }
        _ => bcd::decode_tbcd(bytes),
    }
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
                let digits = decode_bcd(&bytes[1..], odd_even);
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
                let digits = decode_bcd(&bytes[2..], encoding_scheme == SCHEME_BCD_ODD);
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
                let digits = decode_bcd(&bytes[3..], encoding_scheme == SCHEME_BCD_ODD);
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
                digits,
                ..
            } => {
                // The odd/even bit is the digit count's, not the caller's.
                let odd = if is_odd(digits) { 0x80 } else { 0x00 };
                let mut buf = vec![odd | (nature_of_address & 0x7F)];
                buf.extend_from_slice(&encode_bcd(digits)?);
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
                let scheme = wire_scheme(*encoding_scheme, digits);
                let mut buf = vec![*translation_type, (numbering_plan << 4) | scheme];
                buf.extend_from_slice(&encode_digits(scheme, digits)?);
                Ok(buf)
            }
            Self::Gt0100 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                nature_of_address,
                digits,
            } => {
                let scheme = wire_scheme(*encoding_scheme, digits);
                let mut buf = vec![
                    *translation_type,
                    (numbering_plan << 4) | scheme,
                    nature_of_address & 0x7F,
                ];
                buf.extend_from_slice(&encode_digits(scheme, digits)?);
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
            encoding_scheme: 1, // seven digits: BCD odd
            digits: "5550142".to_string(),
        };
        let decoded = GlobalTitle::decode(&gt.encode().unwrap(), GtIndicator::Gt0011).unwrap();
        assert_eq!(decoded, gt);
    }

    // ── Digit parity, Q.713 3.4.2.3 ──────────────────────────────────────────
    // The octets below are written out by hand from Q.713 3.4.2.3.1 and
    // 3.4.2.3.4: two address signals per octet, the earlier one in the low half,
    // encoding scheme 1 for an odd count and 2 for an even one, and filler 0000
    // in the high half of the last octet when the count is odd. The Wireshark
    // SCCP dissector reads each of them back as the digit string named here.

    /// GT 0100, TT 0, NP 1, NAI 4, ten digits.
    const GT0100_TEN: [u8; 8] = [0x00, 0x12, 0x04, 0x51, 0x55, 0x10, 0x00, 0x21];
    /// The same with eleven digits: scheme 1, filler 0 after the last signal.
    const GT0100_ELEVEN: [u8; 9] = [0x00, 0x11, 0x04, 0x51, 0x55, 0x10, 0x00, 0x21, 0x03];
    /// The same with twelve digits.
    const GT0100_TWELVE: [u8; 9] = [0x00, 0x12, 0x04, 0x51, 0x55, 0x10, 0x00, 0x21, 0x43];

    fn gt0100(encoding_scheme: u8, digits: &str) -> GlobalTitle {
        GlobalTitle::Gt0100 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme,
            nature_of_address: 4,
            digits: digits.to_string(),
        }
    }

    #[test]
    fn gt0100_encodes_the_scheme_from_the_digit_count() {
        // Whatever BCD scheme the caller left in the struct, the wire says what
        // the digits are: a receiver drops the last half octet on scheme 1.
        for scheme in [1, 2] {
            assert_eq!(gt0100(scheme, "1555010012").encode().unwrap(), GT0100_TEN);
            assert_eq!(
                gt0100(scheme, "15550100123").encode().unwrap(),
                GT0100_ELEVEN
            );
            assert_eq!(
                gt0100(scheme, "155501001234").encode().unwrap(),
                GT0100_TWELVE
            );
        }
    }

    #[test]
    fn gt0100_decodes_by_the_scheme() {
        assert_eq!(
            GlobalTitle::decode(&GT0100_TEN, GtIndicator::Gt0100).unwrap(),
            gt0100(2, "1555010012")
        );
        // Filler 0000 is not a digit.
        assert_eq!(
            GlobalTitle::decode(&GT0100_ELEVEN, GtIndicator::Gt0100).unwrap(),
            gt0100(1, "15550100123")
        );
        assert_eq!(
            GlobalTitle::decode(&GT0100_TWELVE, GtIndicator::Gt0100).unwrap(),
            gt0100(2, "155501001234")
        );
        // An even title that ends in 0 keeps it.
        assert_eq!(
            GlobalTitle::decode(
                &[0x00, 0x12, 0x04, 0x51, 0x55, 0x10, 0x00],
                GtIndicator::Gt0100
            )
            .unwrap()
            .digits(),
            Some("15550100")
        );
        // A filler of 1111, as some senders write it, is dropped just the same.
        assert_eq!(
            GlobalTitle::decode(
                &[0x00, 0x11, 0x04, 0x51, 0x55, 0x10, 0x00, 0x21, 0xF3],
                GtIndicator::Gt0100
            )
            .unwrap()
            .digits(),
            Some("15550100123")
        );
    }

    #[test]
    fn gt0011_follows_the_digit_count() {
        let gt = |encoding_scheme, digits: &str| GlobalTitle::Gt0011 {
            translation_type: 0,
            numbering_plan: 1,
            encoding_scheme,
            digits: digits.to_string(),
        };
        let even = [0x00, 0x12, 0x51, 0x55, 0x10, 0x00, 0x21];
        let odd = [0x00, 0x11, 0x51, 0x55, 0x10, 0x00, 0x21, 0x03];
        assert_eq!(gt(1, "1555010012").encode().unwrap(), even);
        assert_eq!(gt(2, "15550100123").encode().unwrap(), odd);
        assert_eq!(
            GlobalTitle::decode(&even, GtIndicator::Gt0011).unwrap(),
            gt(2, "1555010012")
        );
        assert_eq!(
            GlobalTitle::decode(&odd, GtIndicator::Gt0011).unwrap(),
            gt(1, "15550100123")
        );
    }

    #[test]
    fn gt0001_follows_the_digit_count() {
        // Q.713 3.4.2.3.1: bit 8 of the first octet is 1 for an odd count.
        let gt = |odd_even, digits: &str| GlobalTitle::Gt0001 {
            nature_of_address: 4,
            odd_even,
            digits: digits.to_string(),
        };
        let even = [0x04, 0x51, 0x55, 0x10, 0x00, 0x21];
        let odd = [0x84, 0x51, 0x55, 0x10, 0x00, 0x21, 0x03];
        assert_eq!(gt(true, "1555010012").encode().unwrap(), even);
        assert_eq!(gt(false, "15550100123").encode().unwrap(), odd);
        assert_eq!(
            GlobalTitle::decode(&even, GtIndicator::Gt0001).unwrap(),
            gt(false, "1555010012")
        );
        assert_eq!(
            GlobalTitle::decode(&odd, GtIndicator::Gt0001).unwrap(),
            gt(true, "15550100123")
        );
    }

    #[test]
    fn a_scheme_that_is_not_bcd_is_left_alone() {
        // 0 (unknown) and 3 (national specific) say nothing about the count, so
        // the octet goes out as given and the digits as before.
        let gt = gt0100(3, "15550100123");
        let encoded = gt.encode().unwrap();
        assert_eq!(
            encoded,
            [0x00, 0x13, 0x04, 0x51, 0x55, 0x10, 0x00, 0x21, 0xF3]
        );
        assert_eq!(
            GlobalTitle::decode(&encoded, GtIndicator::Gt0100).unwrap(),
            gt
        );
    }

    #[test]
    fn an_odd_title_without_digits_decodes_empty() {
        assert_eq!(
            GlobalTitle::decode(&[0x00, 0x11, 0x04], GtIndicator::Gt0100)
                .unwrap()
                .digits(),
            Some("")
        );
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
