use crate::primitive::bytes::{copy_bytes, reserve_bytes};
use crate::primitive::ebcdic::{
    ASCII_TO_EBCDIC_037, EBCDIC_037_TO_ASCII, decode_ebcdic_1142, encode_ebcdic_1142, translate_bytes, translate_bytes_inplace,
};
use crate::utils::cold_path;
use crate::{Error, Step};

/// Permissive conversion between ASCII bytes and their CP037 counterparts.
/// Supported characters, including ASCII controls, round-trip exactly. Encoding
/// replaces each non-ASCII byte with CP037 SUB (0x3F); decoding replaces each
/// CP037 byte outside the ASCII repertoire with ASCII SUB (0x1A).
/// This is byte conversion, not UTF-8 character decoding.
///
/// Strict callers validate before conversion: use an `Ascii` field check on
/// encode and `DecodeCheck<Ebcdic037, Ebcdic037Ascii<MIN, MAX>>` on decode.
/// Validation after conversion cannot distinguish replacements from genuine SUB.
pub struct Ebcdic037;
/// Strict conversion between UTF-8 text and IBM1142 wire bytes. Encoding
/// rejects invalid UTF-8 and characters outside IBM1142; pair it with
/// `Ebcdic1142Text` to also check the length, which counts Unicode scalars.
pub struct Ebcdic1142;

impl Step for Ebcdic037 {
    const ENCODE_IN_PLACE: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        let buf = copy_bytes(output, input)?;
        translate_bytes_inplace(buf, &ASCII_TO_EBCDIC_037);
        Ok(buf)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        if let Some(semantic_len) = semantic_len
            && input.len() != semantic_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        let buf = reserve_bytes(scratch, input.len())?;
        translate_bytes(buf, input, &EBCDIC_037_TO_ASCII)?;
        Ok(buf)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8]) -> Result<(), Error> {
        translate_bytes_inplace(buf, &ASCII_TO_EBCDIC_037);
        Ok(())
    }
}

impl Step for Ebcdic1142 {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_mul(3).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_ebcdic_1142(output, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        if let Some(semantic_len) = semantic_len
            && input.len() != semantic_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        decode_ebcdic_1142(scratch, input).map(|buf| &*buf)
    }
}
