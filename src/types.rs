//! SCCP enumerations: message types, subsystem numbers, and return causes.

use std::fmt;

/// SCCP Message Types (ITU-T Q.713 Section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    /// Connection Request
    Cr = 0x01,
    /// Connection Confirm
    Cc = 0x02,
    /// Connection Refused
    Cref = 0x03,
    /// Released
    Rlsd = 0x04,
    /// Release Complete
    Rlc = 0x05,
    /// Data Form 1
    Dt1 = 0x06,
    /// Data Form 2
    Dt2 = 0x07,
    /// Data Acknowledgement
    Ak = 0x08,
    /// Unitdata
    Udt = 0x09,
    /// Unitdata Service
    Udts = 0x0A,
    /// Expedited Data
    Ed = 0x0B,
    /// Expedited Data Acknowledgement
    Ea = 0x0C,
    /// Reset Request
    Rsr = 0x0D,
    /// Reset Confirmation
    Rsc = 0x0E,
    /// Protocol Data Unit Error
    Err = 0x0F,
    /// Inactivity Test
    It = 0x10,
    /// Extended Unitdata
    Xudt = 0x11,
    /// Extended Unitdata Service
    Xudts = 0x12,
    /// Long Unitdata
    Ludt = 0x13,
    /// Long Unitdata Service
    Ludts = 0x14,
}

impl MessageType {
    /// Map a message-type octet to its [`MessageType`], or `None` if unknown.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(Self::Cr),
            0x02 => Some(Self::Cc),
            0x03 => Some(Self::Cref),
            0x04 => Some(Self::Rlsd),
            0x05 => Some(Self::Rlc),
            0x06 => Some(Self::Dt1),
            0x07 => Some(Self::Dt2),
            0x08 => Some(Self::Ak),
            0x09 => Some(Self::Udt),
            0x0A => Some(Self::Udts),
            0x0B => Some(Self::Ed),
            0x0C => Some(Self::Ea),
            0x0D => Some(Self::Rsr),
            0x0E => Some(Self::Rsc),
            0x0F => Some(Self::Err),
            0x10 => Some(Self::It),
            0x11 => Some(Self::Xudt),
            0x12 => Some(Self::Xudts),
            0x13 => Some(Self::Ludt),
            0x14 => Some(Self::Ludts),
            _ => None,
        }
    }
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Cr => "CR",
            Self::Cc => "CC",
            Self::Cref => "CREF",
            Self::Rlsd => "RLSD",
            Self::Rlc => "RLC",
            Self::Dt1 => "DT1",
            Self::Dt2 => "DT2",
            Self::Ak => "AK",
            Self::Udt => "UDT",
            Self::Udts => "UDTS",
            Self::Ed => "ED",
            Self::Ea => "EA",
            Self::Rsr => "RSR",
            Self::Rsc => "RSC",
            Self::Err => "ERR",
            Self::It => "IT",
            Self::Xudt => "XUDT",
            Self::Xudts => "XUDTS",
            Self::Ludt => "LUDT",
            Self::Ludts => "LUDTS",
        };
        write!(f, "{name}")
    }
}

/// Subsystem Numbers (ITU-T Q.713 Section 3.4.2.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SubsystemNumber {
    /// SSN not known or not used.
    Unknown = 0,
    /// SCCP Management
    SccpMgmt = 1,
    /// ITU-T reserved
    Reserved = 2,
    /// ISUP (ISDN User Part)
    Isup = 3,
    /// OMAP (Operations, Maintenance and Administration Part)
    Omap = 4,
    /// MAP (Mobile Application Part)
    Map = 5,
    /// HLR (Home Location Register)
    Hlr = 6,
    /// VLR (Visitor Location Register)
    Vlr = 7,
    /// MSC (Mobile Switching Centre)
    Msc = 8,
    /// EIR (Equipment Identity Register)
    Eir = 9,
    /// AuC (Authentication Centre)
    Auc = 10,
    /// CAP (CAMEL Application Part)
    Cap = 146,
    /// PCAP
    Pcap = 249,
    /// Any other subsystem number not named above.
    Other(u8),
}

impl SubsystemNumber {
    /// Map a raw SSN octet to a [`SubsystemNumber`]; unknown values become [`SubsystemNumber::Other`].
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::Unknown,
            1 => Self::SccpMgmt,
            2 => Self::Reserved,
            3 => Self::Isup,
            4 => Self::Omap,
            5 => Self::Map,
            6 => Self::Hlr,
            7 => Self::Vlr,
            8 => Self::Msc,
            9 => Self::Eir,
            10 => Self::Auc,
            146 => Self::Cap,
            249 => Self::Pcap,
            other => Self::Other(other),
        }
    }

    /// The raw SSN octet for this subsystem number.
    pub fn value(&self) -> u8 {
        match self {
            Self::Unknown => 0,
            Self::SccpMgmt => 1,
            Self::Reserved => 2,
            Self::Isup => 3,
            Self::Omap => 4,
            Self::Map => 5,
            Self::Hlr => 6,
            Self::Vlr => 7,
            Self::Msc => 8,
            Self::Eir => 9,
            Self::Auc => 10,
            Self::Cap => 146,
            Self::Pcap => 249,
            Self::Other(v) => *v,
        }
    }
}

impl fmt::Display for SubsystemNumber {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown => write!(f, "Unknown(0)"),
            Self::SccpMgmt => write!(f, "SCCP-MGMT(1)"),
            Self::Isup => write!(f, "ISUP(3)"),
            Self::Omap => write!(f, "OMAP(4)"),
            Self::Map => write!(f, "MAP(5)"),
            Self::Hlr => write!(f, "HLR(6)"),
            Self::Vlr => write!(f, "VLR(7)"),
            Self::Msc => write!(f, "MSC(8)"),
            Self::Eir => write!(f, "EIR(9)"),
            Self::Auc => write!(f, "AuC(10)"),
            Self::Cap => write!(f, "CAP(146)"),
            Self::Pcap => write!(f, "PCAP(249)"),
            Self::Reserved => write!(f, "Reserved(2)"),
            Self::Other(v) => write!(f, "SSN({v})"),
        }
    }
}

/// Return Cause for UDTS/XUDTS messages (ITU-T Q.713 Section 3.12).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReturnCause {
    /// No translation for an address of such nature.
    NoTranslationForAddress,
    /// No translation for this specific address.
    NoTranslationForSpecificAddress,
    /// The destination subsystem is congested.
    SubsystemCongestion,
    /// The destination subsystem has failed.
    SubsystemFailure,
    /// The destination subsystem is unequipped.
    Unequipped,
    /// The underlying MTP transport failed.
    MtpFailure,
    /// The network is congested.
    NetworkCongestion,
    /// Message returned for an unqualified reason.
    Unqualified,
    /// Error in message transport (XUDT/LUDT).
    ErrorInMessageTransport,
    /// Error in local processing (XUDT/LUDT).
    ErrorInLocalProcessing,
    /// Destination cannot perform reassembly (XUDT/LUDT).
    DestinationCannotPerformReassembly,
    /// SCCP failure.
    SccpFailure,
    /// The hop counter reached zero (routing loop protection).
    HopCounterViolation,
    /// Segmentation not supported.
    SegmentationNotSupported,
    /// Segmentation failure.
    SegmentationFailure,
    /// Any other return cause not named above.
    Other(u8),
}

impl ReturnCause {
    /// Map a raw return-cause octet to a [`ReturnCause`]; unknown values become [`ReturnCause::Other`].
    pub fn from_u8(value: u8) -> Self {
        match value {
            0 => Self::NoTranslationForAddress,
            1 => Self::NoTranslationForSpecificAddress,
            2 => Self::SubsystemCongestion,
            3 => Self::SubsystemFailure,
            4 => Self::Unequipped,
            5 => Self::MtpFailure,
            6 => Self::NetworkCongestion,
            7 => Self::Unqualified,
            8 => Self::ErrorInMessageTransport,
            9 => Self::ErrorInLocalProcessing,
            10 => Self::DestinationCannotPerformReassembly,
            11 => Self::SccpFailure,
            12 => Self::HopCounterViolation,
            13 => Self::SegmentationNotSupported,
            14 => Self::SegmentationFailure,
            other => Self::Other(other),
        }
    }

    /// The raw return-cause octet for this cause.
    pub fn value(&self) -> u8 {
        match self {
            Self::NoTranslationForAddress => 0,
            Self::NoTranslationForSpecificAddress => 1,
            Self::SubsystemCongestion => 2,
            Self::SubsystemFailure => 3,
            Self::Unequipped => 4,
            Self::MtpFailure => 5,
            Self::NetworkCongestion => 6,
            Self::Unqualified => 7,
            Self::ErrorInMessageTransport => 8,
            Self::ErrorInLocalProcessing => 9,
            Self::DestinationCannotPerformReassembly => 10,
            Self::SccpFailure => 11,
            Self::HopCounterViolation => 12,
            Self::SegmentationNotSupported => 13,
            Self::SegmentationFailure => 14,
            Self::Other(v) => *v,
        }
    }
}

impl fmt::Display for ReturnCause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoTranslationForAddress => write!(f, "No translation for address"),
            Self::NoTranslationForSpecificAddress => {
                write!(f, "No translation for specific address")
            }
            Self::SubsystemCongestion => write!(f, "Subsystem congestion"),
            Self::SubsystemFailure => write!(f, "Subsystem failure"),
            Self::Unequipped => write!(f, "Unequipped"),
            Self::MtpFailure => write!(f, "MTP failure"),
            Self::NetworkCongestion => write!(f, "Network congestion"),
            Self::Unqualified => write!(f, "Unqualified"),
            Self::ErrorInMessageTransport => write!(f, "Error in message transport"),
            Self::ErrorInLocalProcessing => write!(f, "Error in local processing"),
            Self::DestinationCannotPerformReassembly => {
                write!(f, "Destination cannot perform reassembly")
            }
            Self::SccpFailure => write!(f, "SCCP failure"),
            Self::HopCounterViolation => write!(f, "Hop counter violation"),
            Self::SegmentationNotSupported => write!(f, "Segmentation not supported"),
            Self::SegmentationFailure => write!(f, "Segmentation failure"),
            Self::Other(v) => write!(f, "Other({v})"),
        }
    }
}
