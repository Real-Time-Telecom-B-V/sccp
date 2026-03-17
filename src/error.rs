/// Errors that can occur during SCCP message processing.
#[derive(Debug, thiserror::Error)]
pub enum SccpError {
    #[error("message too short: expected at least {expected} bytes, got {actual}")]
    TooShort { expected: usize, actual: usize },

    #[error("invalid message type: 0x{0:02x}")]
    InvalidMessageType(u8),

    #[error("invalid global title indicator: {0}")]
    InvalidGtIndicator(u8),

    #[error("invalid address: {0}")]
    InvalidAddress(String),

    #[error("invalid BCD digit: 0x{0:02x}")]
    InvalidBcdDigit(u8),
}
