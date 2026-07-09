use core::marker::PhantomData;

use super::Check;
use crate::Error;
use crate::utils::{cold_path, take_scratch};

pub trait Step {
    const INPLACE: bool = false;

    fn encoded_len(input_len: usize) -> Result<usize, Error>;
    fn decoded_max_len(input_len: usize) -> Result<usize, Error>;

    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error>;

    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error>;

    fn encode_inplace(_buf: &mut [u8]) -> Result<(), Error> {
        cold_path();
        Err(Error::Internal)
    }
}

pub struct Chain<A, B>(PhantomData<(A, B)>);
pub struct ByteCheck<S, C>(PhantomData<(S, C)>);

impl<First: Step, Rest: Step> Step for Chain<First, Rest> {
    const INPLACE: bool = First::INPLACE && Rest::INPLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Rest::encoded_len(First::encoded_len(input_len)?)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        First::decoded_max_len(Rest::decoded_max_len(input_len)?)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        let mid_len = First::encoded_len(input.len())?;
        let mid_buf = take_scratch(scratch, mid_len)?;
        let mut mid_out = mid_buf;
        let mid = First::encode(&mut mid_out, scratch, input)?;
        Rest::encode(output, scratch, mid)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let rest_output_len = match output_len {
            Some(output_len) => Some(First::encoded_len(output_len)?),
            None => None,
        };
        let mid_cap = rest_output_len.unwrap_or(Rest::decoded_max_len(input.len())?);
        let mid_buf = take_scratch(scratch, mid_cap)?;
        let mut mid_out = mid_buf;
        let mid = Rest::decode(input, &mut mid_out, scratch, rest_output_len)?;
        First::decode(mid, output, scratch, output_len)
    }

    #[inline(always)]
    fn encode_inplace(buf: &mut [u8]) -> Result<(), Error> {
        First::encode_inplace(buf)?;
        Rest::encode_inplace(buf)
    }
}

impl<S: Step, C: Check> Step for ByteCheck<S, C> {
    const INPLACE: bool = S::INPLACE;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        S::encoded_len(input_len)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        S::decoded_max_len(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        S::encode(output, scratch, input)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        output: &mut &'a mut [u8],
        scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        C::validate(input)?;
        S::decode(input, output, scratch, output_len)
    }

    #[inline(always)]
    fn encode_inplace(buf: &mut [u8]) -> Result<(), Error> {
        S::encode_inplace(buf)
    }
}
