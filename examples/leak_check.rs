//! Memory-leak check.
//!
//! A counting global allocator tracks **live bytes** (allocated − freed) — RSS
//! is too noisy (the OS/allocator retains freed pages), but live bytes are
//! exact, so a real leak shows up as monotonic growth. Two phases:
//!
//!   1. **udt** — encode + decode a UDT routed on SSN for many cycles (the
//!      Address Indicator pack/unpack + variable-part pointer arithmetic + body
//!      copy path).
//!   2. **gt** — encode + decode a UDT + UDTS carrying full GT0100 Global Titles
//!      (adds the TBCD digit pack/unpack + `String` build path), over and over.
//!   3. **ext** — encode + decode XUDT / XUDTS / LUDT / LUDTS (the hop-counter
//!      messages: the four-pointer extended variable part, the two-octet-pointer
//!      long variable part, and the opaque optional-part copy).
//!
//! Each phase asserts live bytes return to a flat baseline. Exits non-zero on a
//! leak. Driven by `scripts/mem_leak_test.sh`.
//!
//! Run: `cargo run --release --example leak_check`

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicI64, Ordering};

use sccp::{
    ExtendedUnitData, ExtendedUnitDataService, GlobalTitle, LongUnitData, LongUnitDataService,
    ReturnCause, SccpAddress, SubsystemNumber, UnitData, UnitDataService,
};

// ── Counting allocator ──────────────────────────────────────────────────────
static LIVE: AtomicI64 = AtomicI64::new(0);

struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = System.alloc(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l);
        LIVE.fetch_sub(l.size() as i64, Ordering::Relaxed);
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(l);
        if !p.is_null() {
            LIVE.fetch_add(l.size() as i64, Ordering::Relaxed);
        }
        p
    }
    unsafe fn realloc(&self, ptr: *mut u8, l: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, l, new_size);
        if !p.is_null() {
            LIVE.fetch_add(new_size as i64 - l.size() as i64, Ordering::Relaxed);
        }
        p
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

fn live() -> i64 {
    LIVE.load(Ordering::Relaxed)
}

// ── Phase 1: UDT-on-SSN workload ────────────────────────────────────────────
fn udt_cycle(iters: usize) {
    let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
    let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
    let udt = UnitData::new(called, calling, vec![0x62, 0x40, 0x01, 0x02, 0x03]);
    for _ in 0..iters {
        let bytes = udt.encode().unwrap();
        std::hint::black_box(UnitData::decode(&bytes).unwrap());
    }
}

// ── Phase 2: Global-Title (UDT + UDTS) churn ────────────────────────────────
fn gt_cycle(iters: usize) {
    let called_gt = GlobalTitle::Gt0100 {
        translation_type: 0,
        numbering_plan: 1,
        encoding_scheme: 1,
        nature_of_address: 4,
        digits: "15551234567".to_string(),
    };
    let calling_gt = GlobalTitle::Gt0100 {
        translation_type: 0,
        numbering_plan: 1,
        encoding_scheme: 1,
        nature_of_address: 4,
        digits: "15559876543".to_string(),
    };
    let called = SccpAddress::with_gt(called_gt, Some(SubsystemNumber::Hlr));
    let calling = SccpAddress::with_gt(calling_gt, Some(SubsystemNumber::Msc));
    let udt = UnitData::new(called.clone(), calling.clone(), vec![0x62, 0x40]);
    let udts = UnitDataService::new(
        ReturnCause::SubsystemFailure,
        called,
        calling,
        vec![0x62, 0x40],
    );
    for _ in 0..iters {
        let ub = udt.encode().unwrap();
        std::hint::black_box(UnitData::decode(&ub).unwrap());
        let sb = udts.encode().unwrap();
        std::hint::black_box(UnitDataService::decode(&sb).unwrap());
    }
}

// ── Phase 3: extended / long (XUDT + XUDTS + LUDT + LUDTS) churn ─────────────
fn ext_cycle(iters: usize) {
    let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
    let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);

    let mut xudt = ExtendedUnitData::new(called.clone(), calling.clone(), vec![0x62, 0x40]);
    xudt.optional_part = vec![0x12, 0x01, 0x03, 0x00]; // exercise the optional-part copy
    let xudts = ExtendedUnitDataService::new(
        ReturnCause::HopCounterViolation,
        called.clone(),
        calling.clone(),
        vec![0x62, 0x40],
    );
    let ludt = LongUnitData::new(called.clone(), calling.clone(), vec![0xAB; 400]);
    let ludts = LongUnitDataService::new(
        ReturnCause::HopCounterViolation,
        called,
        calling,
        vec![0x62, 0x40],
    );

    for _ in 0..iters {
        std::hint::black_box(ExtendedUnitData::decode(&xudt.encode().unwrap()).unwrap());
        std::hint::black_box(ExtendedUnitDataService::decode(&xudts.encode().unwrap()).unwrap());
        std::hint::black_box(LongUnitData::decode(&ludt.encode().unwrap()).unwrap());
        std::hint::black_box(LongUnitDataService::decode(&ludts.encode().unwrap()).unwrap());
    }
}

fn report(phase: &str, base: i64) -> i64 {
    let growth = live() - base;
    println!("  {phase}: live = {} bytes (Δ {:+})", live(), growth);
    growth
}

fn main() {
    const ITERS: usize = 200_000;
    const CYCLES: usize = 10;
    const BUDGET: i64 = 64 * 1024;

    // Phase 1: UDT on SSN.
    println!("[udt] {CYCLES} x {ITERS} encode+decode round-trips (UDT routed on SSN)");
    udt_cycle(ITERS); // warm up
    let udt_base = live();
    for c in 1..=CYCLES {
        udt_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), udt_base);
    }
    let udt_growth = live() - udt_base;

    // Phase 2: Global Title (UDT + UDTS).
    println!("\n[gt] {CYCLES} x {ITERS} encode+decode round-trips (UDT + UDTS with GT0100)");
    gt_cycle(ITERS); // warm up
    let gt_base = live();
    for c in 1..=CYCLES {
        gt_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), gt_base);
    }
    let gt_growth = live() - gt_base;

    // Phase 3: extended / long messages (hop counter).
    println!("\n[ext] {CYCLES} x {ITERS} encode+decode round-trips (XUDT + XUDTS + LUDT + LUDTS)");
    ext_cycle(ITERS); // warm up
    let ext_base = live();
    for c in 1..=CYCLES {
        ext_cycle(ITERS);
        report(&format!("cycle {c:>2}/{CYCLES}"), ext_base);
    }
    let ext_growth = live() - ext_base;

    // Verdict.
    println!();
    let mut ok = true;
    if udt_growth > BUDGET {
        eprintln!("FAIL: UDT live bytes grew {udt_growth} (> {BUDGET})");
        ok = false;
    }
    if gt_growth > BUDGET {
        eprintln!("FAIL: GT live bytes grew {gt_growth} (> {BUDGET})");
        ok = false;
    }
    if ext_growth > BUDGET {
        eprintln!("FAIL: extended/long live bytes grew {ext_growth} (> {BUDGET})");
        ok = false;
    }
    if !ok {
        std::process::exit(1);
    }
    println!(
        "PASS: UDT Δ {udt_growth} ≤ {BUDGET}; GT Δ {gt_growth} ≤ {BUDGET}; ext Δ {ext_growth} ≤ {BUDGET}"
    );
}
