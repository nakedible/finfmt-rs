use crate::Error;

#[cold]
pub(crate) const fn cold_path() {}

#[inline(always)]
pub(crate) fn take_scratch<'a>(scratch: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    scratch.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })
}

/// Borrow the first `len` bytes of an encode workspace, returning them and the rest.
#[inline(always)]
pub(crate) fn split_scratch(scratch: &mut [u8], len: usize) -> Result<(&mut [u8], &mut [u8]), Error> {
    scratch.split_at_mut_checked(len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })
}
