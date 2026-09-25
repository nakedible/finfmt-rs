//! Whole-message benchmarks over the real message formats in `examples/`.

use std::time::Duration;

use compact_str::CompactString;
use zenbench::prelude::*;

#[path = "../examples/codecs/nibss.rs"]
#[allow(dead_code)]
mod nibss;

use nibss::{AuthorizationRequest0100, AuthorizationResponse0110, NibssMessage, NibssMessageFmt};

fn quick(group: &mut BenchGroup) {
    group.config().max_rounds(20).max_time(Duration::from_millis(300));
}

fn request_0100() -> NibssMessage {
    NibssMessage::AuthorizationRequest0100(AuthorizationRequest0100 {
        f002_primary_account_number: "5399838383838381".into(),
        f003_processing_code: "310000".into(),
        f004_amount_transaction: 12345,
        f007_transmission_date_time_utc: "0101123456".into(),
        f011_systems_trace_audit_number: "123456".into(),
        f012_time_local_transaction: "123456".into(),
        f013_date_local_transaction: "0101".into(),
        f014_date_expiration: "2601".into(),
        f018_merchant_type: "5999".into(),
        f022_pos_entry_mode: "051".into(),
        f023_card_sequence_number: Some("001".into()),
        f025_pos_condition_code: "00".into(),
        f026_pos_pin_capture_code: Some("12".into()),
        f028_amount_transaction_fee: -150,
        f032_acquiring_institution_id_code: "12345678901".into(),
        f035_track_2_data: Some("5399838383838381D26011234567890000000".into()),
        f037_retrieval_reference_number: "ABC123456789".into(),
        f040_service_restriction_code: Some("201".into()),
        f041_card_acceptor_terminal_id: "TERMID01".into(),
        f042_card_acceptor_id_code: "MERCHANT0000001".into(),
        f043_card_acceptor_name_location: CompactString::from(format!("{:<40}", "SHOP 12 LAGOS NG")),
        f049_currency_code_transaction: "566".into(),
        f052_pin_data: Some("1234567890ABCDEF".into()),
        f053_security_related_control_information: None,
        f054_additional_amounts: None,
        f055_integrated_circuit_card_system_related_data: Some(
            "9F2608C2E0F5A1B3D4E6F79F2701809F10120110A04003220000000000000000000000FF9F3704A1B2C3D49F36020042950500000080009A032609259C01009F02060000000123455F2A020566820239009F1A0205669F34034103029F3303E0F8C89F3501229F1E0831323334353637388407A00000000410109F090200029F4104000001235F340101".into(),
        ),
        f056_message_reason_code: None,
        f059_transport_echo_data: Some("echo-123".into()),
        f060_payment_information: None,
        f062_private_field_management_data_1: None,
        f102_account_identification_1: None,
        f103_account_identification_2: None,
        f123_pos_data_code: "511101511344101".into(),
        f124_near_field_communication_data: None,
        f128_secondary_message_hash_value: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
    })
}

fn response_0110() -> NibssMessage {
    NibssMessage::AuthorizationResponse0110(AuthorizationResponse0110 {
        f003_processing_code: "310000".into(),
        f004_amount_transaction: 12345,
        f007_transmission_date_time_utc: "0101123456".into(),
        f011_systems_trace_audit_number: "123456".into(),
        f012_time_local_transaction: "123456".into(),
        f013_date_local_transaction: "0101".into(),
        f037_retrieval_reference_number: "ABC123456789".into(),
        f039_response_code: "00".into(),
        f041_card_acceptor_terminal_id: "TERMID01".into(),
        f042_card_acceptor_id_code: "MERCHANT0000001".into(),
        f049_currency_code_transaction: "566".into(),
        f123_pos_data_code: "511101511344101".into(),
        f128_secondary_message_hash_value: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
    })
}

fn wire(value: &NibssMessage) -> Vec<u8> {
    let mut output = vec![0; 4096];
    let used = finfmt::encode::<NibssMessageFmt, _>(&mut output, &mut [0; 4096], value).unwrap();
    output.truncate(used);
    output
}

fn bench_message(suite: &mut Suite) {
    suite.group("nibss", |group| {
        quick(group);
        for (name, value) in [("0100", request_0100()), ("0110", response_0110())] {
            let encoded = wire(&value);
            let mut scratch = [0; 4096];
            let decoded = finfmt::decode::<NibssMessageFmt, _>(&encoded, &mut scratch).unwrap();
            assert_eq!(decoded, value);

            group.bench(format!("encode_{name}"), move |b| {
                b.iter(|| {
                    let mut output = [0u8; 2048];
                    let mut scratch = [0u8; 2048];
                    black_box(finfmt::encode::<NibssMessageFmt, _>(&mut output, &mut scratch, black_box(&value)).ok());
                    black_box(output)
                })
            });
            group.bench(format!("decode_{name}"), move |b| {
                b.iter(|| {
                    let mut scratch = [0u8; 2048];
                    black_box(finfmt::decode::<NibssMessageFmt, _>(black_box(&encoded), &mut scratch).ok());
                })
            });
        }
    });
}

zenbench::main!(bench_message);
