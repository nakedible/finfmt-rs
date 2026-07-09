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

#[inline(always)]
pub(crate) fn copy_into<'a>(output: &mut &'a mut [u8], input: &[u8]) -> Result<&'a mut [u8], Error> {
    let ret = output.split_off_mut(..input.len()).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    ret.copy_from_slice(input);
    Ok(ret)
}
