use crate::Error;

#[cold]
pub(crate) const fn cold_path() {}

/// Borrow the first `len` bytes of an encode workspace, returning them and the rest.
#[inline(always)]
pub(crate) fn split_scratch(scratch: &mut [u8], len: usize) -> Result<(&mut [u8], &mut [u8]), Error> {
    scratch.split_at_mut_checked(len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })
}

/// Report a value length error as `Invalid`, for errors that are not about the
/// length of a value the caller is encoding: decoded values and typed numbers.
#[inline(always)]
pub(crate) fn length_as_invalid(error: Error) -> Error {
    if error == Error::InvalidValueLength {
        Error::Invalid
    } else {
        error
    }
}

/// Report a length prefix that cannot hold the value's length as a value
/// length error: shortening the value is what fixes it.
#[inline(always)]
pub(crate) fn prefix_overflow(error: Error) -> Error {
    if error == Error::Invalid {
        Error::InvalidValueLength
    } else {
        error
    }
}
