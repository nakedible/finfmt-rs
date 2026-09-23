use core::marker::PhantomData;

use crate::primitive::nibble::{NibbleAlphabet, pack_expanded_nibbles, pack_nibbles, unpack_nibbles, unpack_padded_nibbles};
use crate::utils::cold_path;
use crate::{Error, Step};

pub struct PackNibbles<F: NibbleAlphabet, const ALIGN_RIGHT: bool = false, const PADDING: u8 = 0>(PhantomData<F>);

pub type PackNibblesRight<F, const PADDING: u8 = 0> = PackNibbles<F, true, PADDING>;
pub type PackNibblesLeft<F, const PADDING: u8 = 0> = PackNibbles<F, false, PADDING>;

impl<F: NibbleAlphabet, const ALIGN_RIGHT: bool, const PADDING: u8> Step for PackNibbles<F, ALIGN_RIGHT, PADDING> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len.div_ceil(2))
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_mul(2).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        pack_nibbles(output, input, ALIGN_RIGHT, PADDING, &F::NIBBLES)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        match semantic_len {
            Some(semantic_len) => unpack_padded_nibbles(scratch, input, semantic_len, ALIGN_RIGHT, PADDING, &F::DIGITS).map(|buf| &*buf),
            None => unpack_nibbles(scratch, input, &F::DIGITS).map(|buf| &*buf),
        }
    }
}

pub struct UnpackNibbles<F: NibbleAlphabet>(PhantomData<F>);

impl<F: NibbleAlphabet> Step for UnpackNibbles<F> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_mul(2).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len / 2)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        unpack_nibbles(output, input, &F::DIGITS)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], semantic_len: Option<usize>) -> Result<&'a [u8], Error> {
        if let Some(semantic_len) = semantic_len
            && semantic_len != input.len() / 2
        {
            cold_path();
            return Err(Error::Invalid);
        }
        pack_expanded_nibbles(scratch, input, &F::NIBBLES).map(|buf| &*buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::nibble::HexUpper;

    #[test]
    fn unpack_decode_checks_actual_shape_and_requested_length() {
        for (input, requested, expected) in [
            (&b"AB"[..], None, Ok(&[0xAB][..])),
            (b"AB", Some(1), Ok(&[0xAB][..])),
            (b"AB", Some(2), Err(Error::Invalid)),
            (b"A", None, Err(Error::Invalid)),
            (b"A", Some(0), Err(Error::Invalid)),
            (b"AG", Some(1), Err(Error::Invalid)),
        ] {
            let mut output = [0; 2];
            assert_eq!(UnpackNibbles::<HexUpper>::decode(input, &mut &mut output[..], requested), expected);
        }
    }
}
