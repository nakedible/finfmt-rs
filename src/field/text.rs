use crate::primitive::text::{decode_bytes, encode_bytes};
use crate::utils::{cold_path, copy_into};
use crate::{Error, Step};

pub struct Identity;

impl Step for Identity {
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
        copy_into(output, input)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        _output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        if let Some(output_len) = output_len
            && input.len() != output_len
        {
            cold_path();
            return Err(Error::Invalid);
        }
        Ok(input)
    }

    #[inline(always)]
    fn encode_inplace(_buf: &mut [u8]) -> Result<(), Error> {
        Ok(())
    }
}

pub struct PadRight<const PAD_TO: usize, const CHAR: u8 = b' ', const MIN_LEN: usize = 0>;

impl<const PAD_TO: usize, const CHAR: u8, const MIN_LEN: usize> Step for PadRight<PAD_TO, CHAR, MIN_LEN> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len.max(PAD_TO))
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_bytes(output, input, PAD_TO, usize::MAX, false, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        _output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let mut input_ref = input;
        decode_bytes(&mut input_ref, output_len.unwrap_or(MIN_LEN).max(MIN_LEN), input.len(), false, CHAR)
    }
}

pub struct PadLeft<const PAD_TO: usize, const CHAR: u8 = b' ', const MIN_LEN: usize = 0>;

impl<const PAD_TO: usize, const CHAR: u8, const MIN_LEN: usize> Step for PadLeft<PAD_TO, CHAR, MIN_LEN> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len.max(PAD_TO))
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_bytes(output, input, PAD_TO, usize::MAX, true, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        _output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let mut input_ref = input;
        decode_bytes(&mut input_ref, output_len.unwrap_or(MIN_LEN).max(MIN_LEN), input.len(), true, CHAR)
    }
}

pub struct PadRightEven<const CHAR: u8 = b' '>;

impl<const CHAR: u8> Step for PadRightEven<CHAR> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len + input_len % 2)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_bytes(output, input, input.len() + input.len() % 2, usize::MAX, false, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        _output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let mut input_ref = input;
        decode_bytes(&mut input_ref, output_len.unwrap_or(0), input.len(), false, CHAR)
    }
}

pub struct PadLeftEven<const CHAR: u8 = b' '>;

impl<const CHAR: u8> Step for PadLeftEven<CHAR> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len + input_len % 2)
    }

    #[inline(always)]
    fn decoded_max_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_bytes(output, input, input.len() + input.len() % 2, usize::MAX, true, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(
        input: &'a [u8],
        _output: &mut &'a mut [u8],
        _scratch: &mut &'a mut [u8],
        output_len: Option<usize>,
    ) -> Result<&'a [u8], Error> {
        let mut input_ref = input;
        decode_bytes(&mut input_ref, output_len.unwrap_or(0), input.len(), true, CHAR)
    }
}
