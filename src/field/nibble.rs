use core::marker::PhantomData;

use crate::primitive::nibble::{NibbleAlphabet, pack_nibbles, pack_nibbles_checked, unpack_nibbles, unpack_padded_nibbles};
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
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        pack_nibbles::<F>(output, input, ALIGN_RIGHT, PADDING)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], count: Option<usize>) -> Result<&'a [u8], Error> {
        match count {
            Some(count) => unpack_padded_nibbles::<F>(scratch, input, count, ALIGN_RIGHT, PADDING).map(|buf| &*buf),
            None => unpack_nibbles::<F>(scratch, input).map(|buf| &*buf),
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
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        unpack_nibbles::<F>(output, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _count: Option<usize>) -> Result<&'a [u8], Error> {
        pack_nibbles_checked::<F>(scratch, input).map(|buf| &*buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitive::nibble::UpperHexDigits;

    #[test]
    fn unpack_decode_checks_actual_shape() {
        for (input, requested, expected) in [
            (&b"AB"[..], None, Ok(&[0xAB][..])),
            (b"AB", Some(1), Ok(&[0xAB][..])),
            (b"A", None, Err(Error::Invalid)),
            (b"A", Some(0), Err(Error::Invalid)),
            (b"AG", Some(1), Err(Error::Invalid)),
        ] {
            let mut output = [0; 2];
            assert_eq!(
                UnpackNibbles::<UpperHexDigits>::decode(input, &mut &mut output[..], requested),
                expected
            );
        }
    }
}
