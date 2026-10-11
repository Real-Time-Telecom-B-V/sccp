//! Error type for SCCP encoding and decoding.

/// Errors that can occur during SCCP message processing.
#[derive(Debug, thiserror::Error)]
pub enum SccpError {
    /// The input buffer ended before a required field could be read.
    #[error("message too short: expected at least {expected} bytes, got {actual}")]
    TooShort {
        /// Number of bytes the decoder needed to be present.
        expected: usize,
        /// Number of bytes actually available.
        actual: usize,
    },

    /// The message-type octet did not match a type this codec can decode.
    #[error("invalid message type: 0x{0:02x}")]
    InvalidMessageType(u8),

    /// The Global Title Indicator field held a value outside 0–4.
    #[error("invalid global title indicator: {0}")]
    InvalidGtIndicator(u8),

    /// An address field was structurally invalid; the string describes why.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// A parameter or pointer does not fit the field Q.713 gives it, so the
    /// message cannot be encoded. A length indicator is one octet (two for the
    /// long data of LUDT / LUDTS) and so is a pointer (two in LUDT / LUDTS):
    /// Q.713 1.4, 2.2, 3.16 and 3.20.
    #[error("{field} does not fit its field: {value} is more than {maximum}")]
    TooLong {
        /// What does not fit: `"data"`, `"long data"`, `"called party address"`,
        /// `"calling party address"`, or `"pointer to ..."` when the parameters
        /// before the one pointed at are too long together.
        field: &'static str,
        /// The length in octets, or the pointer value, that was needed.
        value: usize,
        /// The largest value the field carries: 255, or 65 535 for a two-octet
        /// field.
        maximum: usize,
    },

    /// A digit could not be encoded to Telephony-BCD (not `0`–`9`, `*`, `#`, or `a`–`c`).
    #[error("invalid BCD digit: 0x{0:02x}")]
    InvalidBcdDigit(u8),
}
