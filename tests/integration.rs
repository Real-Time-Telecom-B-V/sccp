//! Integration tests — SCCP address encoding and UDT message tests.

use sccp::*;

/// GT0100 with E.164 number — full round-trip through UDT.
#[test]
fn udt_with_gt0100_e164() {
    let called_gt = GlobalTitle::Gt0100 {
        translation_type: 0,
        numbering_plan: 1,  // E.164
        encoding_scheme: 1, // BCD odd
        nature_of_address: 4, // International
        digits: "31612345678".to_string(),
    };
    let calling_gt = GlobalTitle::Gt0100 {
        translation_type: 0,
        numbering_plan: 1,
        encoding_scheme: 1,
        nature_of_address: 4,
        digits: "31687654321".to_string(),
    };

    let called = SccpAddress::with_gt(called_gt, Some(SubsystemNumber::Hlr));
    let calling = SccpAddress::with_gt(calling_gt, Some(SubsystemNumber::Msc));
    let tcap_data = vec![0x62, 0x48, 0x04, 0x01, 0x00, 0x01]; // TCAP Begin stub

    let udt = UnitData::new(called.clone(), calling.clone(), tcap_data.clone());
    let encoded = udt.encode().unwrap();
    let decoded = UnitData::decode(&encoded).unwrap();

    // Verify called party
    assert!(!decoded.called_party.route_on_ssn);
    assert_eq!(decoded.called_party.ssn, Some(SubsystemNumber::Hlr));
    assert_eq!(
        decoded.called_party.global_title.digits().unwrap(),
        "31612345678"
    );

    // Verify calling party
    assert_eq!(decoded.calling_party.ssn, Some(SubsystemNumber::Msc));
    assert_eq!(
        decoded.calling_party.global_title.digits().unwrap(),
        "31687654321"
    );

    // Verify data
    assert_eq!(decoded.data, tcap_data);
}

/// SSN-only addressing (route on SSN, no GT).
#[test]
fn udt_ssn_only() {
    let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, Some(1234));
    let calling = SccpAddress::with_ssn(SubsystemNumber::Vlr, Some(5678));

    let udt = UnitData::new(called.clone(), calling.clone(), vec![0xAA, 0xBB]);
    let encoded = udt.encode().unwrap();
    let decoded = UnitData::decode(&encoded).unwrap();

    assert!(decoded.called_party.route_on_ssn);
    assert_eq!(decoded.called_party.point_code, Some(1234));
    assert_eq!(decoded.called_party.ssn, Some(SubsystemNumber::Hlr));
    assert!(decoded.calling_party.route_on_ssn);
    assert_eq!(decoded.calling_party.point_code, Some(5678));
}

/// GT0001 format.
#[test]
fn gt0001_address() {
    let gt = GlobalTitle::Gt0001 {
        nature_of_address: 4,
        odd_even: true,
        digits: "12345".to_string(),
    };
    let addr = SccpAddress::with_gt(gt, None);
    let encoded = addr.encode().unwrap();
    let decoded = SccpAddress::decode(&encoded).unwrap();

    match &decoded.global_title {
        GlobalTitle::Gt0001 { nature_of_address, digits, .. } => {
            assert_eq!(*nature_of_address, 4);
            assert_eq!(digits, "12345");
        }
        _ => panic!("Expected Gt0001"),
    }
}

/// GT0010 format.
#[test]
fn gt0010_address() {
    let gt = GlobalTitle::Gt0010 {
        translation_type: 5,
        digits: "9876543210".to_string(),
    };
    let addr = SccpAddress::with_gt(gt, Some(SubsystemNumber::Map));
    let encoded = addr.encode().unwrap();
    let decoded = SccpAddress::decode(&encoded).unwrap();

    match &decoded.global_title {
        GlobalTitle::Gt0010 { translation_type, digits } => {
            assert_eq!(*translation_type, 5);
            assert_eq!(digits, "9876543210");
        }
        _ => panic!("Expected Gt0010"),
    }
    assert_eq!(decoded.ssn, Some(SubsystemNumber::Map));
}

/// GT0011 format.
#[test]
fn gt0011_address() {
    let gt = GlobalTitle::Gt0011 {
        translation_type: 0,
        numbering_plan: 1,
        encoding_scheme: 2,
        digits: "1234567890".to_string(),
    };
    let addr = SccpAddress::with_gt(gt, None);
    let encoded = addr.encode().unwrap();
    let decoded = SccpAddress::decode(&encoded).unwrap();

    match &decoded.global_title {
        GlobalTitle::Gt0011 { translation_type, numbering_plan, encoding_scheme, digits } => {
            assert_eq!(*translation_type, 0);
            assert_eq!(*numbering_plan, 1);
            assert_eq!(*encoding_scheme, 2);
            assert_eq!(digits, "1234567890");
        }
        _ => panic!("Expected Gt0011"),
    }
}

/// All SSN values.
#[test]
fn subsystem_numbers() {
    let ssns = vec![
        (SubsystemNumber::Hlr, 6),
        (SubsystemNumber::Vlr, 7),
        (SubsystemNumber::Msc, 8),
        (SubsystemNumber::Eir, 9),
        (SubsystemNumber::Auc, 10),
        (SubsystemNumber::Isup, 3),
        (SubsystemNumber::Map, 5),
        (SubsystemNumber::Cap, 146),
    ];

    for (ssn, expected_value) in ssns {
        assert_eq!(ssn.value(), expected_value);
        assert_eq!(SubsystemNumber::from_u8(expected_value), ssn);
    }
}

/// TBCD encoding edge cases.
#[test]
fn tbcd_edge_cases() {
    // Single digit
    let encoded = sccp::bcd::encode_tbcd("5").unwrap();
    let decoded = sccp::bcd::decode_tbcd(&encoded);
    assert_eq!(decoded, "5");

    // Long number
    let long = "123456789012345678901234567890";
    let encoded = sccp::bcd::encode_tbcd(long).unwrap();
    let decoded = sccp::bcd::decode_tbcd(&encoded);
    assert_eq!(decoded, long);

    // Special chars
    let special = "123*#456";
    let encoded = sccp::bcd::encode_tbcd(special).unwrap();
    let decoded = sccp::bcd::decode_tbcd(&encoded);
    assert_eq!(decoded, special);
}

/// Large UDT with maximum-size TCAP payload.
#[test]
fn udt_large_payload() {
    let called = SccpAddress::with_ssn(SubsystemNumber::Hlr, None);
    let calling = SccpAddress::with_ssn(SubsystemNumber::Msc, None);
    let data = vec![0xAA; 200]; // 200 bytes of TCAP

    let udt = UnitData::new(called, calling, data.clone());
    let encoded = udt.encode().unwrap();
    let decoded = UnitData::decode(&encoded).unwrap();
    assert_eq!(decoded.data.len(), 200);
    assert_eq!(decoded.data, data);
}

/// Address display formatting.
#[test]
fn address_display() {
    let gt = GlobalTitle::Gt0100 {
        translation_type: 0,
        numbering_plan: 1,
        encoding_scheme: 1,
        nature_of_address: 4,
        digits: "31612345678".to_string(),
    };
    let addr = SccpAddress::with_gt(gt, Some(SubsystemNumber::Hlr));
    let display = format!("{addr}");
    assert!(display.contains("GT0100"));
    assert!(display.contains("31612345678"));
    assert!(display.contains("HLR"));
    assert!(display.contains("route=GT"));
}
