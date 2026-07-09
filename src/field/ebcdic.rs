use crate::primitive::ebcdic::{
    ASCII_TO_EBCDIC_037, EBCDIC_037_TO_ASCII, ebcdic_1142_to_utf8, translate_bytes, translate_bytes_inplace, utf8_to_ebcdic_1142,
};
use crate::utils::{cold_path, copy_into};
use crate::{Error, Step};

pub struct Ebcdic037;
pub struct Ebcdic1142;

impl Step for Ebcdic037 {
    const INPLACE: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        let buf = copy_into(output, input)?;
        translate_bytes_inplace(buf, &ASCII_TO_EBCDIC_037);
        Ok(buf)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        if let Some(output_len) = output_len
            && input.len() != output_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        let buf = output.split_off_mut(..input.len()).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })?;
        translate_bytes(buf, input, &EBCDIC_037_TO_ASCII);
        Ok(buf)
    }

    #[inline(always)]
    fn encode_inplace(buf: &mut [u8]) -> Result<(), Error> {
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
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        utf8_to_ebcdic_1142(output, input)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        if let Some(output_len) = output_len
            && input.len() != output_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        ebcdic_1142_to_utf8(output, input).map(|buf| &*buf)
    }
}
