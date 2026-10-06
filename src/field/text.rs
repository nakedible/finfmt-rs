use crate::primitive::bytes::copy_bytes;
use crate::primitive::text::{decode_padded, decode_padded_even, decode_padded_exact, encode_padded};
use crate::utils::cold_path;
use crate::{Error, Step};

pub struct Identity;

impl Step for Identity {
    const ENCODE_IN_PLACE: bool = true;
    const ENCODE_UNCHANGED: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len)
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        copy_bytes(output, input)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], _len: Option<usize>) -> Result<&'a [u8], Error> {
        Ok(input)
    }

    #[inline(always)]
    fn encode_in_place(_buf: &mut [u8], _input_len: usize) -> Result<(), Error> {
        Ok(())
    }
}

/// Width padding: pad the value with `CHAR` on the right to `PAD_TO` value units;
/// longer values pass through.
///
/// Decoding depends on where the field counts its length (see [`crate::Count`]).
/// When the count is taken before this step, it states the value's exact
/// length: the rest must be `CHAR` (`Invalid` otherwise), and a value that
/// really ends in `CHAR` keeps it. Otherwise decoding strips `CHAR` from the
/// right but keeps at least `MIN_LEN` units, so `PadLeft<4, b'0', 1>` decodes
/// `"0000"` as `"0"`.
pub struct PadRight<const PAD_TO: usize, const CHAR: u8 = b' ', const MIN_LEN: usize = 0>;

impl<const PAD_TO: usize, const CHAR: u8, const MIN_LEN: usize> Step for PadRight<PAD_TO, CHAR, MIN_LEN> {
    const ENCODE_APPENDING: bool = true;

    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len.max(PAD_TO))
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_padded(output, input, PAD_TO, false, CHAR)
    }

    #[inline(always)]
    fn encode_in_place(buf: &mut [u8], input_len: usize) -> Result<(), Error> {
        buf.get_mut(input_len..).unwrap_or_default().fill(CHAR);
        Ok(())
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        match len {
            Some(len) => decode_padded_exact(input, len, false, CHAR),
            None => Ok(decode_padded(input, MIN_LEN, false, CHAR)),
        }
    }
}

/// Width padding on the left; otherwise as [`PadRight`].
pub struct PadLeft<const PAD_TO: usize, const CHAR: u8 = b' ', const MIN_LEN: usize = 0>;

impl<const PAD_TO: usize, const CHAR: u8, const MIN_LEN: usize> Step for PadLeft<PAD_TO, CHAR, MIN_LEN> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        Ok(input_len.max(PAD_TO))
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_padded(output, input, PAD_TO, true, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        match len {
            Some(len) => decode_padded_exact(input, len, true, CHAR),
            None => Ok(decode_padded(input, MIN_LEN, true, CHAR)),
        }
    }
}

/// Pad to an even length with at most one `CHAR` on the right, typically so an
/// odd digit count packs into whole bytes. Decoding removes that one character:
/// exactly when the length is known, otherwise if it is present.
pub struct PadRightEven<const CHAR: u8 = b' '>;

impl<const CHAR: u8> Step for PadRightEven<CHAR> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_add(input_len % 2).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_padded(output, input, Self::encoded_len(input.len())?, false, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        decode_padded_even(input, len, false, CHAR)
    }
}

/// Pad to an even length with at most one `CHAR` on the left; decoding mirrors
/// [`PadRightEven`].
pub struct PadLeftEven<const CHAR: u8 = b' '>;

impl<const CHAR: u8> Step for PadLeftEven<CHAR> {
    #[inline(always)]
    fn encoded_len(input_len: usize) -> Result<usize, Error> {
        input_len.checked_add(input_len % 2).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })
    }

    #[inline(always)]
    fn encode<'a>(output: &mut &'a mut [u8], _scratch: &mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
        encode_padded(output, input, Self::encoded_len(input.len())?, true, CHAR)
    }

    #[inline(always)]
    fn decode<'a>(input: &'a [u8], _scratch: &mut &'a mut [u8], len: Option<usize>) -> Result<&'a [u8], Error> {
        decode_padded_even(input, len, true, CHAR)
    }
}
