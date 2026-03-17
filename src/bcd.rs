use crate::error::SccpError;

/// Encode a digit string to TBCD (Telephony BCD) format.
///
/// Two digits per byte, low nibble first. If odd number of digits,
/// the last byte's high nibble is 0x0F (filler).
///
/// Example: "31612345678" → [0x13, 0x16, 0x32, 0x54, 0x76, 0xF8]
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
        let encoded = encode_tbcd("31612345678").unwrap();
        // 3→1, 1→6, 1→2, 2→3, 3→4, 4→5, 5→6, 6→7, 7→8, 8→F
        assert_eq!(
            encoded,
            vec![0x13, 0x16, 0x32, 0x54, 0x76, 0xF8]
        );
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
        let original = "31612345678";
        // This has 11 digits (odd), so let's test an even one
        let original = "3161234567";
        let encoded = encode_tbcd(original).unwrap();
        let decoded = decode_tbcd(&encoded);
        assert_eq!(decoded, original);
    }

    #[test]
    fn round_trip_odd() {
        let original = "31612345678";
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
        assert!(encode_tbcd("123x456").is_err());
    }
}
