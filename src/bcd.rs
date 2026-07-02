//! Telephony-BCD (TBCD) digit packing, as used for Global Title digits.
//!
//! Two digits are packed per byte, low nibble first; an odd digit count is
//! padded with a `0xF` filler nibble. The `*`, `#`, and `a`–`c` extension
//! nibbles are supported.

use crate::error::SccpError;

/// Encode a digit string to TBCD (Telephony BCD) format.
///
/// Two digits per byte, low nibble first. If odd number of digits,
/// the last byte's high nibble is 0x0F (filler).
///
/// Example: "15551234567" → [0x51, 0x55, 0x21, 0x43, 0x65, 0xF7]
pub fn encode_tbcd(digits: &str) -> Result<Vec<u8>, SccpError> {
    let mut bytes = Vec::with_capacity(digits.len().div_ceil(2));

    let chars: Vec<u8> = digits
        .bytes()
        .map(|b| match b {
            b'0'..=b'9' => Ok(b - b'0'),
            b'*' => Ok(0x0A),
            b'#' => Ok(0x0B),
            b'a' | b'A' => Ok(0x0C),
            b'b' | b'B' => Ok(0x0D),
            b'c' | b'C' => Ok(0x0E),
            _ => Err(SccpError::InvalidBcdDigit(b)),
        })
        .collect::<Result<_, _>>()?;

    let mut i = 0;
    while i < chars.len() {
        let low = chars[i];
        let high = if i + 1 < chars.len() {
            chars[i + 1]
        } else {
            0x0F // filler
        };
        bytes.push((high << 4) | low);
        i += 2;
    }

    Ok(bytes)
}

/// Decode TBCD encoded bytes to a digit string.
///
/// Stops at filler nibble (0x0F) or end of bytes.
pub fn decode_tbcd(bytes: &[u8]) -> String {
    let mut digits = String::with_capacity(bytes.len() * 2);

    for &byte in bytes {
        let low = byte & 0x0F;
        let high = (byte >> 4) & 0x0F;

        if low < 0x0F {
            digits.push(nibble_to_char(low));
        }
        if high < 0x0F {
            digits.push(nibble_to_char(high));
        }
    }

    digits
}

fn nibble_to_char(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        0x0A => '*',
        0x0B => '#',
        0x0C => 'a',
        0x0D => 'b',
        0x0E => 'c',
        _ => '?',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_even_digits() {
        let encoded = encode_tbcd("1234").unwrap();
        assert_eq!(encoded, vec![0x21, 0x43]);
    }

    #[test]
    fn encode_odd_digits() {
        let encoded = encode_tbcd("12345").unwrap();
        assert_eq!(encoded, vec![0x21, 0x43, 0xF5]);
    }

    #[test]
    fn encode_phone_number() {
        // "15551234567" (odd length): digits pair low-nibble-first, and the
        // final unpaired digit gets a 0xF filler in its high nibble.
        let encoded = encode_tbcd("15551234567").unwrap();
        assert_eq!(encoded, vec![0x51, 0x55, 0x21, 0x43, 0x65, 0xF7]);
    }

    #[test]
    fn decode_even_digits() {
        let decoded = decode_tbcd(&[0x21, 0x43]);
        assert_eq!(decoded, "1234");
    }

    #[test]
    fn decode_odd_digits() {
        let decoded = decode_tbcd(&[0x21, 0x43, 0xF5]);
        assert_eq!(decoded, "12345");
    }

    #[test]
    fn round_trip_even() {
        // Even digit count → no filler nibble.
        let original = "1234567890";
        let encoded = encode_tbcd(original).unwrap();
        assert_eq!(encoded.len(), original.len() / 2);
        let decoded = decode_tbcd(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_odd() {
        let original = "15551234567";
        let encoded = encode_tbcd(original).unwrap();
        let decoded = decode_tbcd(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn empty() {
        let encoded = encode_tbcd("").unwrap();
        assert!(encoded.is_empty());
        assert_eq!(decode_tbcd(&[]), "");
    }

    #[test]
    fn invalid_digit() {
        match encode_tbcd("123x456") {
            Err(SccpError::InvalidBcdDigit(b'x')) => {}
            other => panic!("expected InvalidBcdDigit, got {other:?}"),
        }
    }

    #[test]
    fn extension_nibbles_round_trip() {
        // `*`, `#`, and `a`-`c` are valid TBCD extension nibbles.
        let original = "12*34#5abc";
        let encoded = encode_tbcd(original).unwrap();
        assert_eq!(decode_tbcd(&encoded), original);
    }

    #[test]
    fn uppercase_hex_letters_normalise_to_lowercase() {
        // Uppercase A/B/C encode to the same nibbles as lowercase and decode back
        // to lowercase.
        let encoded = encode_tbcd("ABC").unwrap();
        assert_eq!(decode_tbcd(&encoded), "abc");
    }
}
