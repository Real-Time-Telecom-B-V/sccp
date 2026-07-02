//! Codec micro-benchmarks: SCCP message encode/decode.
//!
//! Run with `cargo bench`. Numbers feed the README "Performance" table.
//!
//! All fixtures are built from the public API, so the benches measure exactly the
//! work this crate does — Address Indicator pack/unpack, the variable-part pointer
//! arithmetic, TBCD digit packing, and the body copy — with no I/O in the path.
//! Digits are synthetic (fictional +1-555 range).

use criterion::{criterion_group, criterion_main, BatchSize, Criterion, Throughput};
use sccp::{GlobalTitle, SccpAddress, SubsystemNumber, UnitData, UnitDataService};
use sccp::{MessageType, ReturnCause};

/// A UDT routed on SSN (HLR ← MSC) carrying a short synthetic TCAP body — the
/// smallest common connectionless case (no Global Title digits to pack).
fn udt_ssn() -> UnitData {
    let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
    let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
    UnitData::new(called, calling, vec![0x62, 0x40, 0x01, 0x02, 0x03])
}

/// A UDT routed on a full GT0100 (E.164) Global Title with an SSN — the address +
/// GT path (TBCD digit pack/unpack on both parties).
fn udt_gt() -> UnitData {
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
    UnitData::new(called, calling, vec![0x62, 0x40, 0x01, 0x02, 0x03])
}

/// A UDTS (error return) with a GT0100 called party, mirroring the UDT-GT layout
/// with a return cause in place of the class octet.
fn udts_gt() -> UnitDataService {
    let udt = udt_gt();
    UnitDataService::new(
        ReturnCause::SubsystemFailure,
        udt.called_party,
        udt.calling_party,
        udt.data,
    )
}

fn bench_codec(c: &mut Criterion) {
    let udt_ssn = udt_ssn();
    let udt_gt = udt_gt();
    let udts_gt = udts_gt();

    let udt_ssn_bytes = udt_ssn.encode().expect("valid udt");
    let udt_gt_bytes = udt_gt.encode().expect("valid udt");
    let udts_gt_bytes = udts_gt.encode().expect("valid udts");

    // Sanity: message-type octets are what we expect.
    assert_eq!(udt_ssn_bytes[0], MessageType::Udt as u8);
    assert_eq!(udts_gt_bytes[0], MessageType::Udts as u8);

    let mut g = c.benchmark_group("codec");
    g.throughput(Throughput::Elements(1));

    g.bench_function("udt_ssn/decode", |b| {
        b.iter(|| UnitData::decode(&udt_ssn_bytes).unwrap())
    });
    g.bench_function("udt_ssn/encode", |b| {
        b.iter_batched(
            || udt_ssn.clone(),
            |m| m.encode().unwrap(),
            BatchSize::SmallInput,
        )
    });
    g.bench_function("udt_gt/decode", |b| {
        b.iter(|| UnitData::decode(&udt_gt_bytes).unwrap())
    });
    g.bench_function("udt_gt/encode", |b| {
        b.iter_batched(
            || udt_gt.clone(),
            |m| m.encode().unwrap(),
            BatchSize::SmallInput,
        )
    });
    g.bench_function("udts_gt/decode", |b| {
        b.iter(|| UnitDataService::decode(&udts_gt_bytes).unwrap())
    });
    g.bench_function("udts_gt/encode", |b| {
        b.iter_batched(
            || udts_gt.clone(),
            |m| m.encode().unwrap(),
            BatchSize::SmallInput,
        )
    });
    g.finish();
}

criterion_group!(benches, bench_codec);
criterion_main!(benches);
