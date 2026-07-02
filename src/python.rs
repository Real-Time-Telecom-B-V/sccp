//! PyO3 bindings — `pip install sccp` gives a Rust-backed wheel exposing the
//! **same** SCCP (ITU-T Q.711-Q.716) connectionless codec the crate ships.
//!
//! Compiled only with `--features python`; the default crate build is pyo3-free, so
//! `cargo add sccp` / crates.io consumers pull zero pyo3. Two entry points share one
//! `add_contents()`:
//! * `#[pymodule] fn _sccp` — the standalone wheel (maturin `module-name`).
//! * `pub fn register(py, parent)` — mount `sccp` as a submodule of another
//!   extension, so a host can expose sccp without a second shared object.
//!
//! The Python surface is a faithful mirror of the Rust one: [`PyGlobalTitle`] builds
//! the five GT formats, [`PyAddress`] wraps `SccpAddress` (GT/SSN routing), and
//! [`PyUnitData`] / [`PyUnitDataService`] build and parse whole UDT / UDTS messages.
//! `sccp.decode()` dispatches on the message-type octet. The module is declared
//! `gil_used = false`, so it loads on free-threaded ("no-GIL") CPython.

use pyo3::create_exception;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyModule};

use crate::{
    GlobalTitle as CoreGlobalTitle, MessageType, ReturnCause as CoreReturnCause, SccpAddress,
    SccpError as CoreSccpError, SccpMessage, SubsystemNumber, UnitData, UnitDataService,
};

// ── Error mapping ───────────────────────────────────────────────────────────
create_exception!(
    sccp,
    SccpError,
    PyException,
    "SCCP protocol / codec error (ITU-T Q.711-Q.716)."
);

fn sccp_err(e: CoreSccpError) -> PyErr {
    SccpError::new_err(e.to_string())
}

// ── Global Title (Q.713 §3.4.2.3) ───────────────────────────────────────────
/// An SCCP Global Title. Built with one of the classmethods (`no_title`,
/// `gt0001` … `gt0100`); wraps the Rust `GlobalTitle` enum's five variants.
#[pyclass(name = "GlobalTitle", module = "sccp._sccp", from_py_object)]
#[derive(Clone)]
pub struct PyGlobalTitle {
    inner: CoreGlobalTitle,
}

#[pymethods]
impl PyGlobalTitle {
    /// No Global Title (used with SSN routing).
    #[staticmethod]
    fn no_title() -> Self {
        Self {
            inner: CoreGlobalTitle::NoTitle,
        }
    }

    /// GT format 0001: Nature of Address Indicator + digits.
    #[staticmethod]
    #[pyo3(signature = (digits, *, nature_of_address, odd_even = false))]
    fn gt0001(digits: String, nature_of_address: u8, odd_even: bool) -> Self {
        Self {
            inner: CoreGlobalTitle::Gt0001 {
                nature_of_address,
                odd_even,
                digits,
            },
        }
    }

    /// GT format 0010: Translation Type + digits.
    #[staticmethod]
    #[pyo3(signature = (digits, *, translation_type))]
    fn gt0010(digits: String, translation_type: u8) -> Self {
        Self {
            inner: CoreGlobalTitle::Gt0010 {
                translation_type,
                digits,
            },
        }
    }

    /// GT format 0011: Translation Type + Numbering Plan + Encoding Scheme + digits.
    #[staticmethod]
    #[pyo3(signature = (digits, *, translation_type, numbering_plan, encoding_scheme))]
    fn gt0011(
        digits: String,
        translation_type: u8,
        numbering_plan: u8,
        encoding_scheme: u8,
    ) -> Self {
        Self {
            inner: CoreGlobalTitle::Gt0011 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                digits,
            },
        }
    }

    /// GT format 0100: Translation Type + Numbering Plan + Encoding Scheme +
    /// Nature of Address + digits (the fullest, most common form — E.164).
    #[staticmethod]
    #[pyo3(signature = (digits, *, translation_type, numbering_plan, encoding_scheme, nature_of_address))]
    fn gt0100(
        digits: String,
        translation_type: u8,
        numbering_plan: u8,
        encoding_scheme: u8,
        nature_of_address: u8,
    ) -> Self {
        Self {
            inner: CoreGlobalTitle::Gt0100 {
                translation_type,
                numbering_plan,
                encoding_scheme,
                nature_of_address,
                digits,
            },
        }
    }

    /// The GT indicator value (0-4) for this Global Title's format.
    #[getter]
    fn indicator(&self) -> u8 {
        self.inner.indicator() as u8
    }

    /// The decoded address digits, or `None` for a `no_title` GT.
    #[getter]
    fn digits(&self) -> Option<String> {
        self.inner.digits().map(str::to_string)
    }

    fn __repr__(&self) -> String {
        format!("GlobalTitle({})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ── SccpAddress (Q.713 §3.4) ────────────────────────────────────────────────
/// An SCCP Called / Calling Party Address: an Address Indicator, an optional
/// point code, an optional Subsystem Number, and a [`PyGlobalTitle`].
#[pyclass(name = "Address", module = "sccp._sccp", from_py_object)]
#[derive(Clone)]
pub struct PyAddress {
    inner: SccpAddress,
}

#[pymethods]
impl PyAddress {
    /// Build an address that routes on a Global Title, optionally landing on `ssn`.
    #[staticmethod]
    #[pyo3(signature = (global_title, ssn = None))]
    fn with_gt(global_title: PyGlobalTitle, ssn: Option<u8>) -> Self {
        Self {
            inner: SccpAddress::with_gt(global_title.inner, ssn.map(SubsystemNumber::from_u8)),
        }
    }

    /// Build an address that routes on the Subsystem Number, optionally with a
    /// 2-byte point code.
    #[staticmethod]
    #[pyo3(signature = (ssn, point_code = None))]
    fn with_ssn(ssn: u8, point_code: Option<u16>) -> Self {
        Self {
            inner: SccpAddress::with_ssn(SubsystemNumber::from_u8(ssn), point_code),
        }
    }

    /// Route on SSN (`True`) or on GT (`False`).
    #[getter]
    fn route_on_ssn(&self) -> bool {
        self.inner.route_on_ssn
    }

    /// The optional point code.
    #[getter]
    fn point_code(&self) -> Option<u16> {
        self.inner.point_code
    }

    /// The optional Subsystem Number, as its raw octet.
    #[getter]
    fn ssn(&self) -> Option<u8> {
        self.inner.ssn.map(|s| s.value())
    }

    /// The Global Title.
    #[getter]
    fn global_title(&self) -> PyGlobalTitle {
        PyGlobalTitle {
            inner: self.inner.global_title.clone(),
        }
    }

    /// Encode just this address (no length prefix — that is added by the
    /// containing message).
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.encode().map_err(sccp_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    /// Decode an address from its own bytes (as carried, length-prefix stripped).
    #[staticmethod]
    fn decode(data: &[u8]) -> PyResult<Self> {
        Ok(Self {
            inner: SccpAddress::decode(data).map_err(sccp_err)?,
        })
    }

    fn __repr__(&self) -> String {
        format!("Address({})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ── UnitData (UDT, type 0x09) ────────────────────────────────────────────────
/// An SCCP Unitdata (UDT) message — connectionless data transfer. `encode()`
/// produces the full message (type octet, class/handling, pointers, addresses,
/// data); `sccp.decode(...)` returns one of these.
#[pyclass(name = "UnitData", module = "sccp._sccp", skip_from_py_object)]
#[derive(Clone)]
pub struct PyUnitData {
    inner: UnitData,
}

#[pymethods]
impl PyUnitData {
    #[new]
    #[pyo3(signature = (called_party, calling_party, data, *, protocol_class = 0, message_handling = 0))]
    fn new(
        called_party: PyAddress,
        calling_party: PyAddress,
        data: Vec<u8>,
        protocol_class: u8,
        message_handling: u8,
    ) -> Self {
        let mut inner = UnitData::new(called_party.inner, calling_party.inner, data);
        inner.protocol_class = protocol_class;
        inner.message_handling = message_handling;
        Self { inner }
    }

    /// Protocol class (0 or 1 for connectionless).
    #[getter]
    fn protocol_class(&self) -> u8 {
        self.inner.protocol_class
    }

    /// Message handling (0 = discard on error, 1 = return on error).
    #[getter]
    fn message_handling(&self) -> u8 {
        self.inner.message_handling
    }

    /// Called (destination) party address.
    #[getter]
    fn called_party(&self) -> PyAddress {
        PyAddress {
            inner: self.inner.called_party.clone(),
        }
    }

    /// Calling (source) party address.
    #[getter]
    fn calling_party(&self) -> PyAddress {
        PyAddress {
            inner: self.inner.calling_party.clone(),
        }
    }

    /// The user data (typically a TCAP payload) as `bytes`.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.data)
    }

    /// Encode the complete UDT message (including the leading type octet).
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.encode().map_err(sccp_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn __repr__(&self) -> String {
        format!("UnitData({})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ── UnitDataService (UDTS, type 0x0A) ────────────────────────────────────────
/// An SCCP Unitdata Service (UDTS) message — the error response returned when a
/// UDT cannot be delivered, carrying a [`ReturnCause`] octet.
#[pyclass(name = "UnitDataService", module = "sccp._sccp", skip_from_py_object)]
#[derive(Clone)]
pub struct PyUnitDataService {
    inner: UnitDataService,
}

#[pymethods]
impl PyUnitDataService {
    #[new]
    fn new(
        return_cause: u8,
        called_party: PyAddress,
        calling_party: PyAddress,
        data: Vec<u8>,
    ) -> Self {
        Self {
            inner: UnitDataService::new(
                CoreReturnCause::from_u8(return_cause),
                called_party.inner,
                calling_party.inner,
                data,
            ),
        }
    }

    /// The return cause, as its raw octet.
    #[getter]
    fn return_cause(&self) -> u8 {
        self.inner.return_cause.value()
    }

    /// Called (destination) party address, copied from the returned UDT.
    #[getter]
    fn called_party(&self) -> PyAddress {
        PyAddress {
            inner: self.inner.called_party.clone(),
        }
    }

    /// Calling (source) party address, copied from the returned UDT.
    #[getter]
    fn calling_party(&self) -> PyAddress {
        PyAddress {
            inner: self.inner.calling_party.clone(),
        }
    }

    /// The returned user data as `bytes`.
    #[getter]
    fn data<'py>(&self, py: Python<'py>) -> Bound<'py, PyBytes> {
        PyBytes::new(py, &self.inner.data)
    }

    /// Encode the complete UDTS message (including the leading type octet).
    fn encode<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        let bytes = self.inner.encode().map_err(sccp_err)?;
        Ok(PyBytes::new(py, &bytes))
    }

    fn __repr__(&self) -> String {
        format!("UnitDataService({})", self.inner)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.inner == other.inner
    }
}

// ── decode() ────────────────────────────────────────────────────────────────
/// Decode a complete connectionless SCCP message, returning a [`UnitData`] (UDT)
/// or [`UnitDataService`] (UDTS). Any other message type raises `SccpError`.
#[pyfunction]
fn decode(py: Python<'_>, data: &[u8]) -> PyResult<Py<PyAny>> {
    let msg = SccpMessage::decode(data).map_err(sccp_err)?;
    match msg {
        SccpMessage::Udt(udt) => {
            let obj = Bound::new(py, PyUnitData { inner: udt })?;
            Ok(obj.into_any().unbind())
        }
        SccpMessage::Udts(udts) => {
            let obj = Bound::new(py, PyUnitDataService { inner: udts })?;
            Ok(obj.into_any().unbind())
        }
    }
}

// ── Module wiring ───────────────────────────────────────────────────────────
fn add_contents(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add("SccpError", m.py().get_type::<SccpError>())?;
    m.add_class::<PyGlobalTitle>()?;
    m.add_class::<PyAddress>()?;
    m.add_class::<PyUnitData>()?;
    m.add_class::<PyUnitDataService>()?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;

    // Message types (Q.713 §4). Constants mirror the `MessageType` table.
    m.add("MESSAGE_TYPE_UDT", MessageType::Udt as u8)?;
    m.add("MESSAGE_TYPE_UDTS", MessageType::Udts as u8)?;
    m.add("MESSAGE_TYPE_XUDT", MessageType::Xudt as u8)?;
    m.add("MESSAGE_TYPE_XUDTS", MessageType::Xudts as u8)?;
    m.add("MESSAGE_TYPE_LUDT", MessageType::Ludt as u8)?;
    m.add("MESSAGE_TYPE_LUDTS", MessageType::Ludts as u8)?;

    // Subsystem Numbers (Q.713 §3.4.2.2).
    m.add("SSN_UNKNOWN", SubsystemNumber::Unknown.value())?;
    m.add("SSN_SCCP_MGMT", SubsystemNumber::SccpMgmt.value())?;
    m.add("SSN_ISUP", SubsystemNumber::Isup.value())?;
    m.add("SSN_OMAP", SubsystemNumber::Omap.value())?;
    m.add("SSN_MAP", SubsystemNumber::Map.value())?;
    m.add("SSN_HLR", SubsystemNumber::Hlr.value())?;
    m.add("SSN_VLR", SubsystemNumber::Vlr.value())?;
    m.add("SSN_MSC", SubsystemNumber::Msc.value())?;
    m.add("SSN_EIR", SubsystemNumber::Eir.value())?;
    m.add("SSN_AUC", SubsystemNumber::Auc.value())?;
    m.add("SSN_CAP", SubsystemNumber::Cap.value())?;
    m.add("SSN_PCAP", SubsystemNumber::Pcap.value())?;

    // Return causes for UDTS (Q.713 §3.12).
    m.add(
        "RETURN_CAUSE_NO_TRANSLATION_FOR_ADDRESS",
        CoreReturnCause::NoTranslationForAddress.value(),
    )?;
    m.add(
        "RETURN_CAUSE_NO_TRANSLATION_FOR_SPECIFIC_ADDRESS",
        CoreReturnCause::NoTranslationForSpecificAddress.value(),
    )?;
    m.add(
        "RETURN_CAUSE_SUBSYSTEM_CONGESTION",
        CoreReturnCause::SubsystemCongestion.value(),
    )?;
    m.add(
        "RETURN_CAUSE_SUBSYSTEM_FAILURE",
        CoreReturnCause::SubsystemFailure.value(),
    )?;
    m.add(
        "RETURN_CAUSE_UNEQUIPPED",
        CoreReturnCause::Unequipped.value(),
    )?;
    m.add(
        "RETURN_CAUSE_MTP_FAILURE",
        CoreReturnCause::MtpFailure.value(),
    )?;
    m.add(
        "RETURN_CAUSE_NETWORK_CONGESTION",
        CoreReturnCause::NetworkCongestion.value(),
    )?;
    m.add(
        "RETURN_CAUSE_UNQUALIFIED",
        CoreReturnCause::Unqualified.value(),
    )?;
    m.add(
        "RETURN_CAUSE_HOP_COUNTER_VIOLATION",
        CoreReturnCause::HopCounterViolation.value(),
    )?;
    Ok(())
}

/// Standalone wheel entry point (maturin `module-name = "sccp._sccp"`).
#[pymodule]
fn _sccp(m: &Bound<'_, PyModule>) -> PyResult<()> {
    add_contents(m)
}

/// Embedding entry point: build an `sccp` submodule and attach it to `parent`,
/// so a host extension can expose sccp without a second shared object.
pub fn register(py: Python<'_>, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "sccp")?;
    add_contents(&m)?;
    parent.setattr("sccp", &m)?;
    Ok(())
}
