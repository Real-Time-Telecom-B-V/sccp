//! SCCP (Signaling Connection Control Part) codec per ITU-T Q.711-Q.716.
//!
//! Provides types for encoding and decoding SCCP messages including:
//! - SCCP addresses with Global Title (GT) variants
//! - TBCD encoding for GT digits
//! - Unitdata (UDT) and Unitdata Service (UDTS) messages
//! - Subsystem Numbers (SSN)
//!
//! This is a pure codec crate with no transport dependencies.
//!
//! # Example
//!
//! ```
//! use sccp::{SccpAddress, GlobalTitle, SubsystemNumber, UnitData};
//!
//! // Create an address with GT0100 (E.164 number). Digits are synthetic
//! // (fictional +1-555 range).
//! let gt = GlobalTitle::Gt0100 {
//!     translation_type: 0,
//!     numbering_plan: 1,  // E.164
//!     encoding_scheme: 1, // BCD odd
//!     nature_of_address: 4,
//!     digits: "15551234567".to_string(),
//! };
//! let called = SccpAddress::with_gt(gt, Some(SubsystemNumber::Hlr));
//! let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
//!
//! let udt = UnitData::new(called, calling, vec![0x62, 0x40]);
//! let encoded = udt.encode().unwrap();
//! let decoded = UnitData::decode(&encoded).unwrap();
//! assert_eq!(decoded, udt);
//! ```
#![warn(missing_docs)]

pub mod address;
pub mod bcd;
pub mod error;
pub mod global_title;
pub mod message;
pub mod types;

#[cfg(feature = "python")]
pub mod python;

pub use address::SccpAddress;
pub use error::SccpError;
pub use global_title::{GlobalTitle, GtIndicator};
pub use message::{SccpMessage, UnitData, UnitDataService};
pub use types::{MessageType, ReturnCause, SubsystemNumber};
