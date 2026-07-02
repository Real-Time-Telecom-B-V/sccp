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

    /// A digit could not be encoded to Telephony-BCD (not `0`–`9`, `*`, `#`, or `a`–`c`).
    #[error("invalid BCD digit: 0x{0:02x}")]
    InvalidBcdDigit(u8),
}
