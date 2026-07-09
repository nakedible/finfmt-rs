use compact_str::CompactString;
use finfmt::primitive::validation::validate_hex_upper_even;
use finfmt::{Alphanum, Ascii, AsciiLength, Check, Error, Field, Fixed, Numeric, PadLeft, SignPrefix};
use serde::{Deserialize, Serialize};

#[cold]
const fn cold_path() {}

pub struct HexUpperEven<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for HexUpperEven<MIN, MAX> {
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_hex_upper_even(input, MIN, MAX)
    }
}

pub struct Track2Nibss<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Track2Nibss<MIN, MAX> {
    fn validate(input: &[u8]) -> Result<usize, Error> {
        if input.len() < MIN || input.len() > MAX {
            cold_path();
            return Err(Error::InvalidValueLength);
        }
        if !input.iter().all(|b| matches!(b, b'0'..=b'9' | b'=' | b'D')) {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(input.len())
    }
}

type BitmapAsciiHexWord = finfmt::Field<finfmt::Binary<8, 8>, finfmt::Fixed<8>, finfmt::UnpackNibbles<finfmt::primitive::nibble::HexUpper>>;

pub type FixedAsciiNumeric<const N: usize> = Field<Numeric<N, N>, Fixed<N>>;
pub type FixedAsciiAmount<const N: usize> = Field<Numeric<1, N>, Fixed<N>, PadLeft<N, b'0'>>;
pub type FixedAsciiAlphanum<const N: usize> = Field<Alphanum<N, N>, Fixed<N>>;
pub type FixedAscii<const N: usize> = Field<Ascii<N, N>, Fixed<N>>;
pub type FixedAsciiHex<const N: usize> = Field<HexUpperEven<N, N>, Fixed<N>>;
pub type LlvarAsciiNumeric<const MIN: usize, const MAX: usize> = Field<Numeric<MIN, MAX>, AsciiLength<2>>;
pub type LlvarAsciiAlphanum<const MIN: usize, const MAX: usize> = Field<Alphanum<MIN, MAX>, AsciiLength<2>>;
pub type LlvarAscii<const MIN: usize, const MAX: usize> = Field<Ascii<MIN, MAX>, AsciiLength<2>>;
pub type LllvarAsciiNumeric<const MIN: usize, const MAX: usize> = Field<Numeric<MIN, MAX>, AsciiLength<3>>;
pub type LllvarAscii<const MAX: usize> = Field<Ascii<0, MAX>, AsciiLength<3>>;
pub type LlllvarAscii<const MAX: usize> = Field<Ascii<0, MAX>, AsciiLength<4>>;
pub type LllvarAsciiHex<const MAX: usize> = Field<HexUpperEven<0, MAX>, AsciiLength<3>>;
pub type FixedSignedAsciiAmount8 = SignPrefix<Field<Numeric<1, 8>, Fixed<8>, PadLeft<8, b'0'>>>;
pub type LlvarTrack2 = Field<Track2Nibss<1, 37>, AsciiLength<2>>;

/// NIBSS primary authorization request `0100`.
///
/// Source: <https://nibss-plc.com.ng/wp-content/uploads/2025/07/pos-interface-specification-ver-1-161.pdf>
///
/// This is the serde-facing logical message shape only. Tagged private fields and
/// EMV/NFC payloads remain opaque placeholders for now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthorizationRequest0100 {
    pub f002_primary_account_number: CompactString,
    pub f003_processing_code: CompactString,
    pub f004_amount_transaction: u64,
    pub f007_transmission_date_time_utc: CompactString,
    pub f011_systems_trace_audit_number: CompactString,
    pub f012_time_local_transaction: CompactString,
    pub f013_date_local_transaction: CompactString,
    pub f014_date_expiration: CompactString,
    pub f018_merchant_type: CompactString,
    pub f022_pos_entry_mode: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f023_card_sequence_number: Option<CompactString>,
    pub f025_pos_condition_code: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f026_pos_pin_capture_code: Option<CompactString>,
    pub f028_amount_transaction_fee: i64,
    pub f032_acquiring_institution_id_code: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f035_track_2_data: Option<CompactString>,
    pub f037_retrieval_reference_number: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f040_service_restriction_code: Option<CompactString>,
    pub f041_card_acceptor_terminal_id: CompactString,
    pub f042_card_acceptor_id_code: CompactString,
    pub f043_card_acceptor_name_location: CompactString,
    pub f049_currency_code_transaction: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f052_pin_data: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f053_security_related_control_information: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f054_additional_amounts: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f055_integrated_circuit_card_system_related_data: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f056_message_reason_code: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f059_transport_echo_data: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f060_payment_information: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f062_private_field_management_data_1: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f102_account_identification_1: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f103_account_identification_2: Option<CompactString>,
    pub f123_pos_data_code: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f124_near_field_communication_data: Option<CompactString>,
    pub f128_secondary_message_hash_value: CompactString,
}

finfmt::bitmap_format! {
    pub struct AuthorizationRequest0100BodyFmt for AuthorizationRequest0100, finfmt::bitmap::BitmapLayout::iso(2), BitmapAsciiHexWord {
        2 => f002_primary_account_number: LlvarAsciiNumeric<1, 19>,
        3 => f003_processing_code: FixedAsciiAlphanum<6>,
        4 => f004_amount_transaction: FixedAsciiAmount<12>,
        7 => f007_transmission_date_time_utc: FixedAsciiNumeric<10>,
        11 => f011_systems_trace_audit_number: FixedAsciiNumeric<6>,
        12 => f012_time_local_transaction: FixedAsciiNumeric<6>,
        13 => f013_date_local_transaction: FixedAsciiNumeric<4>,
        14 => f014_date_expiration: FixedAsciiNumeric<4>,
        18 => f018_merchant_type: FixedAsciiNumeric<4>,
        22 => f022_pos_entry_mode: FixedAsciiNumeric<3>,
        23 => f023_card_sequence_number: Option<FixedAsciiNumeric<3>>,
        25 => f025_pos_condition_code: FixedAsciiNumeric<2>,
        26 => f026_pos_pin_capture_code: Option<FixedAsciiNumeric<2>>,
        28 => f028_amount_transaction_fee: FixedSignedAsciiAmount8,
        32 => f032_acquiring_institution_id_code: LlvarAsciiAlphanum<1, 11>,
        35 => f035_track_2_data: Option<LlvarTrack2>,
        37 => f037_retrieval_reference_number: FixedAsciiAlphanum<12>,
        40 => f040_service_restriction_code: Option<FixedAsciiNumeric<3>>,
        41 => f041_card_acceptor_terminal_id: FixedAscii<8>,
        42 => f042_card_acceptor_id_code: FixedAscii<15>,
        43 => f043_card_acceptor_name_location: FixedAscii<40>,
        49 => f049_currency_code_transaction: FixedAsciiNumeric<3>,
        52 => f052_pin_data: Option<FixedAsciiHex<16>>,
        53 => f053_security_related_control_information: Option<FixedAsciiHex<96>>,
        54 => f054_additional_amounts: Option<LllvarAscii<120>>,
        55 => f055_integrated_circuit_card_system_related_data: Option<LllvarAsciiHex<510>>,
        56 => f056_message_reason_code: Option<LllvarAsciiNumeric<1, 4>>,
        59 => f059_transport_echo_data: Option<LllvarAscii<255>>,
        60 => f060_payment_information: Option<LllvarAscii<999>>,
        62 => f062_private_field_management_data_1: Option<LllvarAscii<999>>,
        102 => f102_account_identification_1: Option<LlvarAscii<1, 28>>,
        103 => f103_account_identification_2: Option<LlvarAscii<1, 28>>,
        123 => f123_pos_data_code: Field<Alphanum<15, 15>, AsciiLength<3>>,
        124 => f124_near_field_communication_data: Option<LlllvarAscii<9999>>,
        128 => f128_secondary_message_hash_value: FixedAsciiHex<64>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuthorizationResponse0110 {
    pub f003_processing_code: CompactString,
    pub f004_amount_transaction: u64,
    pub f007_transmission_date_time_utc: CompactString,
    pub f011_systems_trace_audit_number: CompactString,
    pub f012_time_local_transaction: CompactString,
    pub f013_date_local_transaction: CompactString,
    pub f037_retrieval_reference_number: CompactString,
    pub f039_response_code: CompactString,
    pub f041_card_acceptor_terminal_id: CompactString,
    pub f042_card_acceptor_id_code: CompactString,
    pub f049_currency_code_transaction: CompactString,
    pub f123_pos_data_code: CompactString,
    pub f128_secondary_message_hash_value: CompactString,
}

finfmt::bitmap_format! {
    pub struct AuthorizationResponse0110BodyFmt for AuthorizationResponse0110, finfmt::bitmap::BitmapLayout::iso(2), BitmapAsciiHexWord {
        3 => f003_processing_code: FixedAsciiAlphanum<6>,
        4 => f004_amount_transaction: FixedAsciiAmount<12>,
        7 => f007_transmission_date_time_utc: FixedAsciiNumeric<10>,
        11 => f011_systems_trace_audit_number: FixedAsciiNumeric<6>,
        12 => f012_time_local_transaction: FixedAsciiNumeric<6>,
        13 => f013_date_local_transaction: FixedAsciiNumeric<4>,
        37 => f037_retrieval_reference_number: FixedAsciiAlphanum<12>,
        39 => f039_response_code: FixedAsciiAlphanum<2>,
        41 => f041_card_acceptor_terminal_id: FixedAscii<8>,
        42 => f042_card_acceptor_id_code: FixedAscii<15>,
        49 => f049_currency_code_transaction: FixedAsciiNumeric<3>,
        123 => f123_pos_data_code: Field<Alphanum<15, 15>, AsciiLength<3>>,
        128 => f128_secondary_message_hash_value: FixedAsciiHex<64>,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)]
pub enum NibssMessage {
    AuthorizationRequest0100(AuthorizationRequest0100),
    AuthorizationResponse0110(AuthorizationResponse0110),
}

finfmt::tagged_format! {
    pub struct NibssMessageFmt for NibssMessage {
        _: FixedAscii<4> = b"0100" => AuthorizationRequest0100(AuthorizationRequest0100BodyFmt),
        _: FixedAscii<4> = b"0110" => AuthorizationResponse0110(AuthorizationResponse0110BodyFmt),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[allow(dead_code)]
pub struct FlatNibssMessage {
    pub mti: CompactString,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f003_processing_code: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f011_systems_trace_audit_number: Option<CompactString>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub f041_card_acceptor_terminal_id: Option<CompactString>,
}

finfmt::bitmap_format! {
    pub struct FlatNibssMessageFmt for FlatNibssMessage, finfmt::bitmap::BitmapLayout::iso(1), BitmapAsciiHexWord {
        head: {
            mti: FixedAscii<4>,
        }
        3 => f003_processing_code: Option<FixedAsciiAlphanum<6>>,
        11 => f011_systems_trace_audit_number: Option<FixedAsciiNumeric<6>>,
        41 => f041_card_acceptor_terminal_id: Option<FixedAscii<8>>,
    }
}

#[cfg(test)]
mod tests {
    use finfmt::CompositeFmt;

    use super::*;

    #[test]
    fn test_authorization_request_0100_json_shape() {
        let value = AuthorizationRequest0100 {
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
            f035_track_2_data: Some("5399838383838381=26011234567890000000".into()),
            f037_retrieval_reference_number: "ABC123456789".into(),
            f040_service_restriction_code: Some("201".into()),
            f041_card_acceptor_terminal_id: "TERMID01".into(),
            f042_card_acceptor_id_code: "MERCHANT0000001".into(),
            f043_card_acceptor_name_location: CompactString::from(format!("{:<40}", "SHOP 12 LAGOS NG")),
            f049_currency_code_transaction: "566".into(),
            f052_pin_data: Some("1234567890ABCDEF".into()),
            f053_security_related_control_information: Some(
                "1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF".into(),
            ),
            f054_additional_amounts: Some("1001566C000000001234".into()),
            f055_integrated_circuit_card_system_related_data: Some("9F260801020304050607089F270180".into()),
            f056_message_reason_code: Some("4000".into()),
            f059_transport_echo_data: Some("echo-123".into()),
            f060_payment_information: Some("*41015BILLER000000001".into()),
            f062_private_field_management_data_1: Some("01015SERIAL123456789".into()),
            f102_account_identification_1: Some("SAVINGS-001".into()),
            f103_account_identification_2: None,
            f123_pos_data_code: "511101511344101".into(),
            f124_near_field_communication_data: Some("NFC-DATA".into()),
            f128_secondary_message_hash_value: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
        };

        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json["f003_processing_code"], "310000");
        assert_eq!(json["f004_amount_transaction"], 12345);
        assert_eq!(json["f028_amount_transaction_fee"], -150);
        assert_eq!(json["f052_pin_data"], "1234567890ABCDEF");
        assert_eq!(
            json["f128_secondary_message_hash_value"],
            "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF"
        );

        let decoded: AuthorizationRequest0100 = serde_json::from_value(json).unwrap();
        assert_eq!(decoded, value);
    }

    #[test]
    fn test_authorization_request_0100_body_binary_roundtrip() {
        let value = AuthorizationRequest0100 {
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
            f053_security_related_control_information: Some(
                "1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF".into(),
            ),
            f054_additional_amounts: Some("1001566C000000001234".into()),
            f055_integrated_circuit_card_system_related_data: Some("9F260801020304050607089F270180".into()),
            f056_message_reason_code: Some("4000".into()),
            f059_transport_echo_data: Some("echo-123".into()),
            f060_payment_information: Some("*41015BILLER000000001".into()),
            f062_private_field_management_data_1: Some("01015SERIAL123456789".into()),
            f102_account_identification_1: Some("SAVINGS-001".into()),
            f103_account_identification_2: Some("CURRENT-002".into()),
            f123_pos_data_code: "511101511344101".into(),
            f124_near_field_communication_data: Some("NFC-DATA".into()),
            f128_secondary_message_hash_value: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
        };

        let mut output = [0u8; 4096];
        let mut scratch = [0u8; 4096];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            AuthorizationRequest0100BodyFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert!(output[..32].iter().all(u8::is_ascii_hexdigit));

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 4096];
        let decoded = AuthorizationRequest0100BodyFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap();
        assert_eq!(decoded, value);
        assert!(input.is_empty());
    }

    fn sample_response() -> AuthorizationResponse0110 {
        AuthorizationResponse0110 {
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
        }
    }

    #[test]
    fn test_nibss_message_fmt_roundtrip_request() {
        let value = NibssMessage::AuthorizationRequest0100(AuthorizationRequest0100 {
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
            f053_security_related_control_information: Some(
                "1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF1234567890ABCDEF".into(),
            ),
            f054_additional_amounts: Some("1001566C000000001234".into()),
            f055_integrated_circuit_card_system_related_data: Some("9F260801020304050607089F270180".into()),
            f056_message_reason_code: Some("4000".into()),
            f059_transport_echo_data: Some("echo-123".into()),
            f060_payment_information: Some("*41015BILLER000000001".into()),
            f062_private_field_management_data_1: Some("01015SERIAL123456789".into()),
            f102_account_identification_1: Some("SAVINGS-001".into()),
            f103_account_identification_2: Some("CURRENT-002".into()),
            f123_pos_data_code: "511101511344101".into(),
            f124_near_field_communication_data: Some("NFC-DATA".into()),
            f128_secondary_message_hash_value: "0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF0123456789ABCDEF".into(),
        });

        let mut output = [0u8; 4096];
        let mut scratch = [0u8; 4096];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            NibssMessageFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0100");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 4096];
        assert_eq!(NibssMessageFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap(), value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_nibss_message_fmt_roundtrip_response() {
        let value = NibssMessage::AuthorizationResponse0110(sample_response());
        let mut output = [0u8; 2048];
        let mut scratch = [0u8; 2048];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            NibssMessageFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0110");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 2048];
        assert_eq!(NibssMessageFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap(), value);
        assert!(input.is_empty());
    }

    #[test]
    fn test_flat_nibss_message_fmt_roundtrip() {
        let value = FlatNibssMessage {
            mti: "0200".into(),
            f003_processing_code: Some("310000".into()),
            f011_systems_trace_audit_number: None,
            f041_card_acceptor_terminal_id: Some("TERMID01".into()),
        };

        let mut output = [0u8; 128];
        let mut scratch = [0u8; 128];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            FlatNibssMessageFmt::encode(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0200");
        assert!(output[4..20].iter().all(u8::is_ascii_hexdigit));

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        assert_eq!(
            FlatNibssMessageFmt::decode(&mut input, decode_scratch.as_mut_slice()).unwrap(),
            value
        );
        assert!(input.is_empty());
    }
}
