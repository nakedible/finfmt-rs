use compact_str::CompactString;
use finfmt::primitive::validation::validate_bytes;
use finfmt::{Alphanum, Ascii, AsciiLength, Check, Error, Field, Fixed, Numeric, PadLeft, SignPrefix, UpperHexEven};
use serde::{Deserialize, Serialize};

/// NIBSS Track 2 data uses `D` as well as `=` as the separator.
pub struct Track2Nibss<const MIN: usize, const MAX: usize>;
impl<const MIN: usize, const MAX: usize> Check for Track2Nibss<MIN, MAX> {
    fn validate(input: &[u8]) -> Result<usize, Error> {
        validate_bytes(input, MIN, MAX, |b| matches!(b, b'0'..=b'9' | b'=' | b'D'))
    }
}

/// ISO 8583 primary and secondary bitmaps as uppercase hex text.
pub struct NibssBitmap;
impl finfmt::BitmapFormat for NibssBitmap {
    const LAYOUT: finfmt::BitmapLayout = finfmt::BitmapLayout::iso(1, 2);
    type Word = finfmt::UnpackNibbles<finfmt::primitive::nibble::UpperHexDigits>;
}

/// A primary bitmap only, as uppercase hex text.
pub struct NibssShortBitmap;
impl finfmt::BitmapFormat for NibssShortBitmap {
    const LAYOUT: finfmt::BitmapLayout = finfmt::BitmapLayout::iso(1, 1);
    type Word = finfmt::UnpackNibbles<finfmt::primitive::nibble::UpperHexDigits>;
}

pub type FixedAsciiNumeric<const N: usize> = Field<Numeric<N, N>, Fixed<N>>;
pub type FixedAsciiAmount<const N: usize> = Field<Numeric<1, N>, Fixed<N>, PadLeft<N, b'0', 1>>;
pub type FixedAsciiAlphanum<const N: usize> = Field<Alphanum<N, N>, Fixed<N>>;
pub type FixedAscii<const N: usize> = Field<Ascii<N, N>, Fixed<N>>;
pub type FixedAsciiHex<const N: usize> = Field<UpperHexEven<N, N>, Fixed<N>>;
pub type LlvarAsciiNumeric<const MIN: usize, const MAX: usize> = Field<Numeric<MIN, MAX>, AsciiLength<2>>;
pub type LlvarAsciiAlphanum<const MIN: usize, const MAX: usize> = Field<Alphanum<MIN, MAX>, AsciiLength<2>>;
pub type LlvarAscii<const MIN: usize, const MAX: usize> = Field<Ascii<MIN, MAX>, AsciiLength<2>>;
pub type LllvarAsciiNumeric<const MIN: usize, const MAX: usize> = Field<Numeric<MIN, MAX>, AsciiLength<3>>;
pub type LllvarAscii<const MAX: usize> = Field<Ascii<0, MAX>, AsciiLength<3>>;
pub type LlllvarAscii<const MAX: usize> = Field<Ascii<0, MAX>, AsciiLength<4>>;
pub type LllvarAsciiHex<const MAX: usize> = Field<UpperHexEven<0, MAX>, AsciiLength<3>>;
pub type FixedSignedAsciiAmount8 = SignPrefix<Field<Numeric<1, 8>, Fixed<8>, PadLeft<8, b'0', 1>>>;
pub type LlvarTrack2 = Field<Track2Nibss<1, 37>, AsciiLength<2>>;

finfmt::wire_type! {
    /// NIBSS primary authorization request `0100`.
    ///
    /// Source: <https://nibss-plc.com.ng/wp-content/uploads/2025/07/pos-interface-specification-ver-1-161.pdf>
    ///
    /// This is the serde-facing logical message shape only. Tagged private fields and
    /// EMV/NFC payloads remain opaque placeholders for now.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[wire(bitmap = NibssBitmap)]
    pub struct AuthorizationRequest0100 {
        #[wire(fmt = LlvarAsciiNumeric<1, 19>, bit = 2)]
        pub f002_primary_account_number: CompactString,
        #[wire(fmt = FixedAsciiAlphanum<6>, bit = 3)]
        pub f003_processing_code: CompactString,
        #[wire(fmt = FixedAsciiAmount<12>, bit = 4)]
        pub f004_amount_transaction: u64,
        #[wire(fmt = FixedAsciiNumeric<10>, bit = 7)]
        pub f007_transmission_date_time_utc: CompactString,
        #[wire(fmt = FixedAsciiNumeric<6>, bit = 11)]
        pub f011_systems_trace_audit_number: CompactString,
        #[wire(fmt = FixedAsciiNumeric<6>, bit = 12)]
        pub f012_time_local_transaction: CompactString,
        #[wire(fmt = FixedAsciiNumeric<4>, bit = 13)]
        pub f013_date_local_transaction: CompactString,
        #[wire(fmt = FixedAsciiNumeric<4>, bit = 14)]
        pub f014_date_expiration: CompactString,
        #[wire(fmt = FixedAsciiNumeric<4>, bit = 18)]
        pub f018_merchant_type: CompactString,
        #[wire(fmt = FixedAsciiNumeric<3>, bit = 22)]
        pub f022_pos_entry_mode: CompactString,
        #[wire(fmt = FixedAsciiNumeric<3>, bit = 23)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f023_card_sequence_number: Option<CompactString>,
        #[wire(fmt = FixedAsciiNumeric<2>, bit = 25)]
        pub f025_pos_condition_code: CompactString,
        #[wire(fmt = FixedAsciiNumeric<2>, bit = 26)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f026_pos_pin_capture_code: Option<CompactString>,
        #[wire(fmt = FixedSignedAsciiAmount8, bit = 28)]
        pub f028_amount_transaction_fee: i64,
        #[wire(fmt = LlvarAsciiAlphanum<1, 11>, bit = 32)]
        pub f032_acquiring_institution_id_code: CompactString,
        #[wire(fmt = LlvarTrack2, bit = 35)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f035_track_2_data: Option<CompactString>,
        #[wire(fmt = FixedAsciiAlphanum<12>, bit = 37)]
        pub f037_retrieval_reference_number: CompactString,
        #[wire(fmt = FixedAsciiNumeric<3>, bit = 40)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f040_service_restriction_code: Option<CompactString>,
        #[wire(fmt = FixedAscii<8>, bit = 41)]
        pub f041_card_acceptor_terminal_id: CompactString,
        #[wire(fmt = FixedAscii<15>, bit = 42)]
        pub f042_card_acceptor_id_code: CompactString,
        #[wire(fmt = FixedAscii<40>, bit = 43)]
        pub f043_card_acceptor_name_location: CompactString,
        #[wire(fmt = FixedAsciiNumeric<3>, bit = 49)]
        pub f049_currency_code_transaction: CompactString,
        #[wire(fmt = FixedAsciiHex<16>, bit = 52)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f052_pin_data: Option<CompactString>,
        #[wire(fmt = FixedAsciiHex<96>, bit = 53)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f053_security_related_control_information: Option<CompactString>,
        #[wire(fmt = LllvarAscii<120>, bit = 54)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f054_additional_amounts: Option<CompactString>,
        #[wire(fmt = LllvarAsciiHex<510>, bit = 55)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f055_integrated_circuit_card_system_related_data: Option<CompactString>,
        #[wire(fmt = LllvarAsciiNumeric<1, 4>, bit = 56)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f056_message_reason_code: Option<CompactString>,
        #[wire(fmt = LllvarAscii<255>, bit = 59)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f059_transport_echo_data: Option<CompactString>,
        #[wire(fmt = LllvarAscii<999>, bit = 60)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f060_payment_information: Option<CompactString>,
        #[wire(fmt = LllvarAscii<999>, bit = 62)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f062_private_field_management_data_1: Option<CompactString>,
        #[wire(fmt = LlvarAscii<1, 28>, bit = 102)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f102_account_identification_1: Option<CompactString>,
        #[wire(fmt = LlvarAscii<1, 28>, bit = 103)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f103_account_identification_2: Option<CompactString>,
        #[wire(fmt = Field<Alphanum<15, 15>, AsciiLength<3>>, bit = 123)]
        pub f123_pos_data_code: CompactString,
        #[wire(fmt = LlllvarAscii<9999>, bit = 124)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f124_near_field_communication_data: Option<CompactString>,
        #[wire(fmt = FixedAsciiHex<64>, bit = 128)]
        pub f128_secondary_message_hash_value: CompactString,
    }
}

finfmt::wire_type! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[wire(bitmap = NibssBitmap)]
    pub struct AuthorizationResponse0110 {
        #[wire(fmt = FixedAsciiAlphanum<6>, bit = 3)]
        pub f003_processing_code: CompactString,
        #[wire(fmt = FixedAsciiAmount<12>, bit = 4)]
        pub f004_amount_transaction: u64,
        #[wire(fmt = FixedAsciiNumeric<10>, bit = 7)]
        pub f007_transmission_date_time_utc: CompactString,
        #[wire(fmt = FixedAsciiNumeric<6>, bit = 11)]
        pub f011_systems_trace_audit_number: CompactString,
        #[wire(fmt = FixedAsciiNumeric<6>, bit = 12)]
        pub f012_time_local_transaction: CompactString,
        #[wire(fmt = FixedAsciiNumeric<4>, bit = 13)]
        pub f013_date_local_transaction: CompactString,
        #[wire(fmt = FixedAsciiAlphanum<12>, bit = 37)]
        pub f037_retrieval_reference_number: CompactString,
        #[wire(fmt = FixedAsciiAlphanum<2>, bit = 39)]
        pub f039_response_code: CompactString,
        #[wire(fmt = FixedAscii<8>, bit = 41)]
        pub f041_card_acceptor_terminal_id: CompactString,
        #[wire(fmt = FixedAscii<15>, bit = 42)]
        pub f042_card_acceptor_id_code: CompactString,
        #[wire(fmt = FixedAsciiNumeric<3>, bit = 49)]
        pub f049_currency_code_transaction: CompactString,
        #[wire(fmt = Field<Alphanum<15, 15>, AsciiLength<3>>, bit = 123)]
        pub f123_pos_data_code: CompactString,
        #[wire(fmt = FixedAsciiHex<64>, bit = 128)]
        pub f128_secondary_message_hash_value: CompactString,
    }
}

finfmt::wire_type! {
    /// A NIBSS message: the MTI, then the body it selects.
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[wire(concat)]
    pub struct NibssMessage {
        #[wire(fmt = FixedAscii<4>)]
        pub mti: CompactString,
        #[wire(select = mti)]
        pub body: NibssBody,
    }

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "snake_case")]
    #[allow(clippy::large_enum_variant)]
    #[wire(selected)]
    pub enum NibssBody {
        #[wire(rename = "0100")]
        AuthorizationRequest0100(AuthorizationRequest0100),
        #[wire(rename = "0110")]
        AuthorizationResponse0110(AuthorizationResponse0110),
    }
}

impl NibssMessage {
    /// A message whose MTI matches its body. Used by the tests and the bench.
    #[allow(dead_code)]
    pub fn new(body: NibssBody) -> Self {
        let mti = match body {
            NibssBody::AuthorizationRequest0100(_) => "0100",
            NibssBody::AuthorizationResponse0110(_) => "0110",
        };
        Self { mti: mti.into(), body }
    }
}

finfmt::wire_type! {
    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
    #[allow(dead_code)]
    #[wire(bitmap = NibssShortBitmap)]
    pub struct FlatNibssMessage {
        #[wire(fmt = FixedAscii<4>)]
        pub mti: CompactString,
        #[wire(fmt = FixedAsciiAlphanum<6>, bit = 3)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f003_processing_code: Option<CompactString>,
        #[wire(fmt = FixedAsciiNumeric<6>, bit = 11)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f011_systems_trace_audit_number: Option<CompactString>,
        #[wire(fmt = FixedAscii<8>, bit = 41)]
        #[serde(skip_serializing_if = "Option::is_none")]
        pub f041_card_acceptor_terminal_id: Option<CompactString>,
    }
}

#[cfg(test)]
mod tests {
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
            <AuthorizationRequest0100 as finfmt::FieldEncode<_>>::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };

        assert!(output[..32].iter().all(u8::is_ascii_hexdigit));

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 4096];
        let decoded =
            <AuthorizationRequest0100 as finfmt::FieldDecode<'_, _>>::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap();
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
        let value = NibssMessage::new(NibssBody::AuthorizationRequest0100(AuthorizationRequest0100 {
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
        }));

        let mut output = [0u8; 4096];
        let mut scratch = [0u8; 4096];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            <NibssMessage as finfmt::FieldEncode<_>>::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0100");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 4096];
        assert_eq!(
            <NibssMessage as finfmt::FieldDecode<'_, _>>::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap(),
            value
        );
        assert!(input.is_empty());
    }

    #[test]
    fn test_nibss_message_fmt_roundtrip_response() {
        let value = NibssMessage::new(NibssBody::AuthorizationResponse0110(sample_response()));
        let mut output = [0u8; 2048];
        let mut scratch = [0u8; 2048];
        let total = output.len();
        let used = {
            let mut out_ptr = output.as_mut_slice();
            <NibssMessage as finfmt::FieldEncode<_>>::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0110");

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 2048];
        assert_eq!(
            <NibssMessage as finfmt::FieldDecode<'_, _>>::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap(),
            value
        );
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
            <FlatNibssMessage as finfmt::FieldEncode<_>>::encode_field(&mut out_ptr, scratch.as_mut_slice(), &value).unwrap();
            total - out_ptr.len()
        };
        assert_eq!(&output[..4], b"0200");
        assert!(output[4..20].iter().all(u8::is_ascii_hexdigit));

        let mut input = &output[..used];
        let mut decode_scratch = [0u8; 128];
        assert_eq!(
            <FlatNibssMessage as finfmt::FieldDecode<'_, _>>::decode_field(&mut input, &mut decode_scratch.as_mut_slice()).unwrap(),
            value
        );
        assert!(input.is_empty());
    }
}
