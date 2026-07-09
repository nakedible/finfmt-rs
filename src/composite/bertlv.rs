use core::str::FromStr;

use super::*;
use crate::primitive::bertlv::{encode_hex_upper_scratch, encode_unknown_tag_key_scratch, encode_unknown_tlv_from_key};

#[inline(always)]
fn decode_unknown_entry<K, V>(tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(K, V), Error>
where
    K: FromStr,
    V: FromStr,
{
    let key = encode_unknown_tag_key_scratch(scratch, tag)?;
    let key = key.parse::<K>().map_err(|_| {
        crate::utils::cold_path();
        Error::Invalid
    })?;
    let value = encode_hex_upper_scratch(scratch, value)?;
    let value = value.parse::<V>().map_err(|_| {
        crate::utils::cold_path();
        Error::Invalid
    })?;
    Ok((key, value))
}

#[inline(always)]
fn encode_unknown_entry<K: AsRef<str> + ?Sized, V: AsRef<str> + ?Sized>(output: &mut &mut [u8], key: &K, value: &V) -> Result<(), Error> {
    encode_unknown_tlv_from_key(output, key.as_ref(), value.as_ref())
}

impl<T, K, V> BerTlvExtras for T
where
    T: Extend<(K, V)>,
    for<'a> &'a T: IntoIterator<Item = (&'a K, &'a V)>,
    K: AsRef<str> + FromStr + PartialEq,
    V: AsRef<str> + FromStr,
{
    #[inline(always)]
    fn encode_unknowns(&self, output: &mut &mut [u8], _scratch: &mut &mut [u8]) -> Result<(), Error> {
        for (key, value) in self {
            encode_unknown_entry(output, key, value)?;
        }
        Ok(())
    }

    #[inline(always)]
    fn decode_unknown(&mut self, tag: &[u8], value: &[u8], scratch: &mut &mut [u8]) -> Result<(), Error> {
        let (key, value) = decode_unknown_entry::<K, V>(tag, value, scratch)?;
        for (existing_key, _) in &*self {
            if existing_key == &key {
                crate::utils::cold_path();
                return Err(Error::Invalid);
            }
        }
        self.extend(core::iter::once((key, value)));
        Ok(())
    }
}
