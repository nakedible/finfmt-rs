use crate::Error;
use crate::primitive::bertlv::{
    BerTlvEntry, MAX_BER_TAG_BYTES, ber_length_width, ber_tag_matches_hex, decode_ber_length, decode_ber_tag, decode_ber_tlv_entry,
    encode_ber_length, encode_ber_tag, parse_ber_tag_hex,
};

#[inline(never)]
pub fn encode_ber_tag_runtime<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_ber_tag(output, input)
}

#[inline(never)]
pub fn decode_ber_tag_runtime<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_ber_tag(input)
}

#[inline(never)]
pub fn encode_ber_length_runtime<'a>(output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    encode_ber_length(output, len)
}

#[inline(never)]
pub fn decode_ber_length_runtime(input: &mut &[u8]) -> Result<usize, Error> {
    decode_ber_length(input)
}

#[inline(never)]
pub fn ber_length_width_runtime(len: usize) -> Result<usize, Error> {
    ber_length_width(len)
}

#[inline(never)]
pub fn parse_ber_tag_hex_runtime(tag: &str) -> Result<([u8; MAX_BER_TAG_BYTES], usize), Error> {
    parse_ber_tag_hex(tag)
}

#[inline(never)]
pub fn ber_tag_matches_hex_9f02(tag_bytes: &[u8]) -> Result<bool, Error> {
    ber_tag_matches_hex(tag_bytes, "9F02")
}

#[inline(never)]
pub fn decode_ber_tlv_entry_runtime<'a>(input: &mut &'a [u8]) -> Result<Option<BerTlvEntry<'a>>, Error> {
    decode_ber_tlv_entry(input)
}
