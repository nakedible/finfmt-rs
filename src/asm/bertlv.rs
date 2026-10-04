use crate::Error;
use crate::primitive::bertlv::{
    BerTlvEntry, MAX_BER_TAG_BYTES, MAX_BER_TAG_HEX, ber_length_width, decode_ber_length, decode_ber_tag, decode_ber_tlv_entry,
    encode_ber_length, encode_ber_tag, encode_ber_tlv_head, format_ber_tag_hex, pack_ber_tag_hex, parse_ber_tag_hex,
};

#[inline(never)]
pub fn encode_ber_tag_runtime<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_ber_tag(output, input)
}

#[inline(never)]
pub fn decode_ber_tag_runtime<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_ber_tag::<true>(input)
}

#[inline(never)]
pub fn decode_ber_tag_lenient_runtime<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_ber_tag::<false>(input)
}

#[inline(never)]
pub fn parse_ber_tag_hex_lenient_runtime(tag: &str) -> Result<([u8; MAX_BER_TAG_BYTES], usize), Error> {
    parse_ber_tag_hex::<false>(tag)
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
    parse_ber_tag_hex::<true>(tag)
}

#[inline(never)]
pub fn format_ber_tag_hex_runtime<'a>(output: &'a mut [u8; MAX_BER_TAG_HEX], tag: &[u8]) -> &'a [u8] {
    format_ber_tag_hex(output, tag)
}

#[inline(never)]
pub fn decode_ber_tlv_entry_runtime<'a>(input: &mut &'a [u8]) -> Result<Option<BerTlvEntry<'a>>, Error> {
    decode_ber_tlv_entry(input)
}

#[inline(never)]
pub fn encode_ber_tlv_head_9f02<'a>(output: &mut &'a mut [u8], value_len: usize) -> Result<&'a mut [u8], Error> {
    encode_ber_tlv_head(output, &[0x9F, 0x02], value_len)
}

#[inline(never)]
pub fn pack_ber_tag_hex_9f02(output: &mut [u8; MAX_BER_TAG_BYTES]) -> &[u8] {
    pack_ber_tag_hex(output, "9F02")
}
