use core::marker::PhantomData;

use crate::primitive::nibble::{NibbleFormat, pack_expanded_nibbles, pack_nibbles, unpack_nibbles, unpack_padded_nibbles};
use crate::utils::cold_path;
use crate::{Error, Step};

pub struct PackNibbles<F: NibbleFormat, const ALIGN_RIGHT: bool = false, const PADDING: u8 = 0>(PhantomData<F>);

pub type PackNibblesRight<F, const PADDING: u8 = 0> = PackNibbles<F, true, PADDING>;
pub type PackNibblesLeft<F, const PADDING: u8 = 0> = PackNibbles<F, false, PADDING>;

impl<F: NibbleFormat, const ALIGN_RIGHT: bool, const PADDING: u8> Step for PackNibbles<F, ALIGN_RIGHT, PADDING> {
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
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        pack_nibbles(output, input, ALIGN_RIGHT, PADDING, &F::TABLE)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        match output_len {
            Some(output_len) => unpack_padded_nibbles(output, input, output_len, ALIGN_RIGHT, PADDING, &F::DIGITS).map(|buf| &*buf),
            None => unpack_nibbles(output, input, &F::DIGITS).map(|buf| &*buf),
        }
    }
}

pub struct UnpackNibbles<F: NibbleFormat>(PhantomData<F>);

impl<F: NibbleFormat> Step for UnpackNibbles<F> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_mul(2).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        if !input_len.is_multiple_of(2) {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(input_len / 2)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        unpack_nibbles(output, input, &F::DIGITS)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let output_len = output_len.unwrap_or(Self::decoded_max_len(input.len())?);
        if input.len() != output_len * 2 {
            cold_path();
            return Err(Error::Invalid);
        }
        pack_expanded_nibbles(output, input, &F::TABLE).map(|buf| &*buf)
    }
}
