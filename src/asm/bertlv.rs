use crate::Error;
use crate::primitive::bertlv::{
    BerTlvEntry, decode_ber_tlv_entry, decode_berlen, decode_bertag, encode_berlen, encode_bertag, encoded_berlen, parse_hex_tag,
    tag_eq_hex,
};

#[inline(never)]
pub fn encode_bertag_runtime<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    encode_bertag(output, input)
}

#[inline(never)]
pub fn decode_bertag_runtime<'a>(input: &mut &'a [u8]) -> Result<&'a [u8], Error> {
    decode_bertag(input)
}

#[inline(never)]
pub fn encode_berlen_runtime<'a>(output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    encode_berlen(output, len)
}

#[inline(never)]
pub fn decode_berlen_runtime(input: &mut &[u8]) -> Result<usize, Error> {
    decode_berlen(input)
}

#[inline(never)]
pub fn encoded_berlen_runtime(len: usize) -> Result<usize, Error> {
    encoded_berlen(len)
}

#[inline(never)]
pub fn parse_hex_tag_runtime(tag: &str) -> Result<([u8; 4], usize), Error> {
    parse_hex_tag(tag)
}

#[inline(never)]
pub fn tag_eq_hex_9f02(tag_bytes: &[u8]) -> Result<bool, Error> {
    tag_eq_hex(tag_bytes, "9F02")
}

#[inline(never)]
pub fn decode_ber_tlv_entry_runtime<'a>(input: &mut &'a [u8]) -> Result<Option<BerTlvEntry<'a>>, Error> {
    decode_ber_tlv_entry(input)
}
