use core::marker::PhantomData;

use crate::primitive::bytes::{copy_bytes, reserve_bytes};
use crate::primitive::codepage::{CodePage, decode_ascii_subset, decode_text, encode_ascii_subset, encode_text};
use crate::{Error, Step};

/// Permissive conversion between ASCII bytes and the ASCII characters of code
/// page `P`, for every field not expected to hold characters outside ASCII.
/// Supported characters, including ASCII controls, round-trip exactly.
/// Encoding replaces every other byte with the page's SUB; decoding replaces
/// every page byte outside the ASCII repertoire with ASCII SUB (0x1A).
/// This is byte conversion, not UTF-8 character decoding.
///
/// Strict callers validate before conversion: use an `Ascii` field check on
/// encode and `DecodeCheck<AsciiSubset<P>, AsciiSubsetBytes<P, MIN, MAX>>` on
/// decode. Validation after conversion cannot distinguish replacements from
/// genuine SUB.
pub struct AsciiSubset<P: CodePage>(PhantomData<P>);

/// Strict conversion between UTF-8 text and code page `P`. Encoding rejects
/// invalid UTF-8 and characters outside the page, and decoding rejects
/// unmapped bytes; pair it with `CharsetText` to also check the length, which
/// counts Unicode scalars.
pub struct Charset<P: CodePage>(PhantomData<P>);

impl<P: CodePage> Step for AsciiSubset<P> {
    const ENCODE_IN_PLACE: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        let buf = copy_bytes(output, input)?;
        encode_ascii_subset::<P>(buf);
        Ok(buf)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _len: Option<usize>) -> Result<&'a [u8], Error> {
        let buf = reserve_bytes(scratch, input.len())?;
        decode_ascii_subset::<P>(buf, input)?;
        Ok(buf)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8], _input_len: usize) -> Result<(), Error> {
        encode_ascii_subset::<P>(buf);
        Ok(())
    }
}

impl<P: CodePage> Step for Charset<P> {
    const INPUT_IN_CHARS: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_text::<P>(output, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], scratch: &mut &'a mut [u8], _len: Option<usize>) -> Result<&'a [u8], Error> {
        decode_text::<P>(scratch, input).map(|buf| &*buf)
    }
}
