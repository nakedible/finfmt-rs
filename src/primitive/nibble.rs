#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::utils::cold_path;

const fn table_from_nibble_digits(digits: &[u8]) -> [u8; 256] {
    let mut table = [0xFF; 256];
    let mut i = 0;
    while i < digits.len() {
        table[digits[i] as usize] = i as u8;
        i += 1;
    }
    table
}

/// Trait for nibble encoding formats.
///
/// Provides both the forward mapping (nibble value to character) in `DIGITS`
/// and the reverse mapping (character to nibble value) in `TABLE`.
pub trait NibbleFormat {
    /// Characters representing nibble values 0-15.
    const DIGITS: [u8; 16];
    /// Lookup table mapping characters to nibble values (0xFF for invalid).
    const TABLE: [u8; 256];
}

/// BCD with zone nibbles: 0-9 to '0'-'9', A-F to ':'-'?'.
pub struct Bcdz;
impl NibbleFormat for Bcdz {
    const DIGITS: [u8; 16] = *b"0123456789:;<=>?";
    const TABLE: [u8; 256] = table_from_nibble_digits(&Self::DIGITS);
}

/// Uppercase hexadecimal: 0-9 to '0'-'9', A-F to 'A'-'F'.
pub struct HexUpper;
impl NibbleFormat for HexUpper {
    const DIGITS: [u8; 16] = *b"0123456789ABCDEF";
    const TABLE: [u8; 256] = table_from_nibble_digits(&Self::DIGITS);
}

/// Lowercase hexadecimal: 0-9 to '0'-'9', A-F to 'a'-'f'.
pub struct HexLower;
impl NibbleFormat for HexLower {
    const DIGITS: [u8; 16] = *b"0123456789abcdef";
    const TABLE: [u8; 256] = table_from_nibble_digits(&Self::DIGITS);
}

/// EBCDIC hexadecimal: 0-9 to F0-F9, A-F to C1-C6.
pub struct HexEbcdic;
impl NibbleFormat for HexEbcdic {
    const DIGITS: [u8; 16] = *b"\xF0\xF1\xF2\xF3\xF4\xF5\xF6\xF7\xF8\xF9\xC1\xC2\xC3\xC4\xC5\xC6";
    const TABLE: [u8; 256] = table_from_nibble_digits(&Self::DIGITS);
}

#[inline(always)]
fn pack_nibbles_exact(dst: &mut [u8], src: &[u8], lut: &[u8; 256]) {
    for (chunk, out) in src.chunks_exact(2).zip(dst.iter_mut()) {
        let hi = lut[chunk[0] as usize];
        let lo = lut[chunk[1] as usize];
        debug_assert!(hi < 16 && lo < 16, "Invalid nibble value");
        *out = (hi << 4) | lo;
    }
}

#[inline(always)]
fn unpack_nibbles_exact(dst: &mut [u8], src: &[u8], digits: &[u8; 16]) {
    for (byte, out) in src.iter().zip(dst.chunks_exact_mut(2)) {
        out[0] = digits[(*byte >> 4) as usize];
        out[1] = digits[(*byte & 0x0F) as usize];
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn unpack_single_nibble(output: &mut u8, input: u8, high: bool, digits: &[u8; 16]) {
    *output = digits[if high { (input >> 4) as usize } else { (input & 0x0F) as usize }];
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn pack_nibbles<'a>(
    output: &mut &'a mut [u8],
    input: impl AsRef<[u8]>,
    align_right: bool,
    padding: u8,
    digit_table: &[u8; 256],
) -> Result<&'a mut [u8], Error> {
    let input = input.as_ref();
    debug_assert!(padding < 16, "Invalid padding nibble for packing");
    let buf = output.split_off_mut(..input.len().div_ceil(2)).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    if input.len() % 2 == 0 {
        pack_nibbles_exact(buf, input, digit_table);
    } else if align_right {
        let ([first, rest @ ..], [firstout, buf_rest @ ..]) = (input, &mut *buf) else {
            cold_path();
            return Err(Error::BufferOverflow);
        };
        let digit = digit_table[*first as usize];
        debug_assert!(digit < 16, "Invalid nibble value");
        *firstout = padding << 4 | digit;
        pack_nibbles_exact(buf_rest, rest, digit_table);
    } else {
        let ([rest @ .., last], [buf_rest @ .., lastout]) = (input, &mut *buf) else {
            cold_path();
            return Err(Error::BufferOverflow);
        };
        let digit = digit_table[*last as usize];
        debug_assert!(digit < 16, "Invalid nibble value");
        pack_nibbles_exact(buf_rest, rest, digit_table);
        *lastout = digit << 4 | padding;
    }
    Ok(buf)
}

/// Validates that every byte in `input` maps to a valid nibble under `digit_table`.
///
/// A byte is valid when `digit_table[byte] <= 0x0F` (i.e., the top nibble of the
/// table entry is zero). `table_from_nibble_digits` stores `0xFF` for invalid
/// bytes, so the check collapses to `OR`-ing the high nibble across the input.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn validate_nibbles(input: &[u8], digit_table: &[u8; 256]) -> Result<(), Error> {
    let mut invalid = 0u8;
    for &byte in input {
        invalid |= digit_table[byte as usize] >> 4;
    }
    if invalid != 0 {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn pack_expanded_nibbles<'a>(
    output: &mut &'a mut [u8],
    input: impl AsRef<[u8]>,
    digit_table: &[u8; 256],
) -> Result<&'a mut [u8], Error> {
    let input = input.as_ref();
    if !input.len().is_multiple_of(2) {
        cold_path();
        return Err(Error::Invalid);
    }
    validate_nibbles(input, digit_table)?;
    pack_nibbles(output, input, false, 0, digit_table)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn unpack_nibbles<'a>(output: &mut &'a mut [u8], input: impl AsRef<[u8]>, digits: &[u8; 16]) -> Result<&'a mut [u8], Error> {
    let input = input.as_ref();
    let buf = output.split_off_mut(..input.len() * 2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    unpack_nibbles_exact(buf, input, digits);
    Ok(buf)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn unpack_padded_nibbles<'a>(
    output: &mut &'a mut [u8],
    input: impl AsRef<[u8]>,
    output_len: usize,
    align_right: bool,
    padding: u8,
    digits: &[u8; 16],
) -> Result<&'a mut [u8], Error> {
    let input = input.as_ref();
    debug_assert!(padding < 16, "Invalid padding nibble for unpacking");
    if input.len() != output_len.div_ceil(2) {
        cold_path();
        return Err(Error::Invalid);
    }
    let buf = output.split_off_mut(..output_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    if output_len.is_multiple_of(2) {
        unpack_nibbles_exact(buf, input, digits);
    } else if align_right {
        let ([first, rest @ ..], [firstout, buf_rest @ ..]) = (input, &mut *buf) else {
            cold_path();
            return Err(Error::Invalid);
        };
        if first >> 4 != padding {
            cold_path();
            return Err(Error::Invalid);
        }
        unpack_single_nibble(firstout, *first, false, digits);
        unpack_nibbles_exact(buf_rest, rest, digits);
    } else {
        let ([rest @ .., last], [buf_rest @ .., lastout]) = (input, &mut *buf) else {
            cold_path();
            return Err(Error::Invalid);
        };
        if last & 0x0F != padding {
            cold_path();
            return Err(Error::Invalid);
        }
        unpack_nibbles_exact(buf_rest, rest, digits);
        unpack_single_nibble(lastout, *last, true, digits);
    }
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack(input: &[u8], align_right: bool, padding: u8, table: &[u8; 256]) -> Vec<u8> {
        let mut output = [0u8; 64];
        let initial_len = output.len();
        let mut outptr = &mut output[..];
        let result = pack_nibbles(&mut outptr, input, align_right, padding, table).unwrap();
        assert_eq!(outptr.len(), initial_len - result.len(), "cursor advancement");
        result.to_vec()
    }

    fn unpack(input: &[u8], digits: &[u8; 16]) -> Vec<u8> {
        let mut output = [0u8; 128];
        let initial_len = output.len();
        let mut outptr = &mut output[..];
        let result = unpack_nibbles(&mut outptr, input, digits).unwrap();
        assert_eq!(outptr.len(), initial_len - result.len(), "cursor advancement");
        result.to_vec()
    }

    fn pack_err(input: &[u8], buf_len: usize) -> Result<(), Error> {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..buf_len];
        pack_nibbles(&mut outptr, input, true, 0, &Bcdz::TABLE).map(|_| ())
    }

    fn unpack_err(input: &[u8], buf_len: usize) -> Result<(), Error> {
        let mut output = [0u8; 128];
        let mut outptr = &mut output[..buf_len];
        unpack_nibbles(&mut outptr, input, &Bcdz::DIGITS).map(|_| ())
    }

    fn pack_expanded(input: &[u8], table: &[u8; 256]) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..];
        pack_expanded_nibbles(&mut outptr, input, table).map(|result| result.to_vec())
    }

    fn pack_expanded_err(input: &[u8], buf_len: usize, table: &[u8; 256]) -> Result<(), Error> {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..buf_len];
        pack_expanded_nibbles(&mut outptr, input, table).map(|_| ())
    }

    #[test]
    fn test_pack() {
        // Empty, single digit (odd), even length
        assert_eq!(pack(b"", true, 0, &Bcdz::TABLE), b"");
        assert_eq!(pack(b"5", true, 0, &Bcdz::TABLE), b"\x05");
        assert_eq!(pack(b"5", false, 0, &Bcdz::TABLE), b"\x50");
        assert_eq!(pack(b"1234", true, 0, &Bcdz::TABLE), b"\x12\x34");
        assert_eq!(pack(b"1234", false, 0, &Bcdz::TABLE), b"\x12\x34");
        // Odd length > 1: alignment and padding
        assert_eq!(pack(b"123", true, 0, &Bcdz::TABLE), b"\x01\x23");
        assert_eq!(pack(b"123", false, 0, &Bcdz::TABLE), b"\x12\x30");
        assert_eq!(pack(b"123", true, 0xF, &Bcdz::TABLE), b"\xF1\x23");
        assert_eq!(pack(b"F", true, 0xF, &HexUpper::TABLE), b"\xFF");
        // All tables
        assert_eq!(pack(b"ABCDEF", true, 0, &HexUpper::TABLE), b"\xAB\xCD\xEF");
        assert_eq!(pack(b"abcdef", true, 0, &HexLower::TABLE), b"\xab\xcd\xef");
        assert_eq!(pack(b"\xF1\xF2\xF3\xF4", true, 0, &HexEbcdic::TABLE), b"\x12\x34");
        assert_eq!(pack(b":;<=>?", true, 0, &Bcdz::TABLE), b"\xAB\xCD\xEF");
    }

    #[test]
    fn test_unpack() {
        // Empty, single byte, edge values
        assert_eq!(unpack(b"", &Bcdz::DIGITS), b"");
        assert_eq!(unpack(b"\x12", &Bcdz::DIGITS), b"12");
        assert_eq!(unpack(b"\x00", &Bcdz::DIGITS), b"00");
        assert_eq!(unpack(b"\xFF", &Bcdz::DIGITS), b"??");
        assert_eq!(unpack(b"\xFF", &HexUpper::DIGITS), b"FF");
        assert_eq!(unpack(b"\xFF", &HexLower::DIGITS), b"ff");
        // Multi-byte with all tables
        assert_eq!(unpack(b"\xAB\xCD\xEF", &Bcdz::DIGITS), b":;<=>?");
        assert_eq!(unpack(b"\xAB\xCD\xEF", &HexUpper::DIGITS), b"ABCDEF");
        assert_eq!(unpack(b"\xAB\xCD\xEF", &HexLower::DIGITS), b"abcdef");
        assert_eq!(unpack(b"\x12\x34", &HexEbcdic::DIGITS), b"\xF1\xF2\xF3\xF4");
    }

    #[test]
    fn test_unpack_padded_and_single_nibble() {
        let mut output = [0u8; 64];
        let mut outptr = &mut output[..];
        assert_eq!(
            unpack_padded_nibbles(&mut outptr, b"\x12\x34", 4, true, 0, &Bcdz::DIGITS).unwrap(),
            b"1234"
        );
        let mut outptr = &mut output[..];
        assert_eq!(
            unpack_padded_nibbles(&mut outptr, b"\x01\x23", 3, true, 0, &Bcdz::DIGITS).unwrap(),
            b"123"
        );
        let mut outptr = &mut output[..];
        assert_eq!(
            unpack_padded_nibbles(&mut outptr, b"\x12\x30", 3, false, 0, &Bcdz::DIGITS).unwrap(),
            b"123"
        );
        let mut out = b' ';
        unpack_single_nibble(&mut out, 0xF1, false, &Bcdz::DIGITS);
        assert_eq!(out, b'1');
        unpack_single_nibble(&mut out, 0x1F, true, &Bcdz::DIGITS);
        assert_eq!(out, b'1');
    }

    #[test]
    fn test_pack_expanded() {
        assert_eq!(pack_expanded(b"", &HexUpper::TABLE), Ok(b"".to_vec()));
        assert_eq!(pack_expanded(b"12", &HexUpper::TABLE), Ok(b"\x12".to_vec()));
        assert_eq!(pack_expanded(b"1234", &HexUpper::TABLE), Ok(b"\x12\x34".to_vec()));
        assert_eq!(pack_expanded(b"1", &HexUpper::TABLE), Err(Error::Invalid));
        assert_eq!(pack_expanded(b"1G", &HexUpper::TABLE), Err(Error::Invalid));
        assert_eq!(pack_expanded(b"\xF1\xF2", &HexEbcdic::TABLE), Ok(b"\x12".to_vec()));
    }

    #[test]
    fn test_buffer_overflow() {
        // Pack: buffer too small, zero buffer
        assert_eq!(pack_err(b"123", 1), Err(Error::BufferOverflow)); // needs 2
        assert_eq!(pack_err(b"1", 0), Err(Error::BufferOverflow)); // needs 1
        assert_eq!(pack_expanded_err(b"12", 0, &HexUpper::TABLE), Err(Error::BufferOverflow)); // needs 1
        // Unpack: buffer too small, zero buffer
        assert_eq!(unpack_err(b"\x12\x34", 3), Err(Error::BufferOverflow)); // needs 4
        assert_eq!(unpack_err(b"\x12", 0), Err(Error::BufferOverflow)); // needs 2
    }

    #[test]
    fn test_lookup_tables() {
        // Verify each table is inverse of its digit array
        fn check<F: NibbleFormat>() {
            for (i, &digit) in F::DIGITS.iter().enumerate() {
                assert_eq!(F::TABLE[digit as usize], i as u8);
            }
        }
        check::<Bcdz>();
        check::<HexUpper>();
        check::<HexLower>();
        check::<HexEbcdic>();
        // Invalid chars return 0xFF
        assert_eq!(Bcdz::TABLE[b'A' as usize], 0xFF);
        assert_eq!(HexUpper::TABLE[b'a' as usize], 0xFF);
        assert_eq!(HexLower::TABLE[b'A' as usize], 0xFF);
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::*;

    fn assert_roundtrip(input: &str, table: &[u8; 256], digits: &[u8; 16]) {
        let even = if input.len() % 2 == 1 {
            format!("0{}", input)
        } else {
            input.to_string()
        };
        let mut packed = [0u8; 64];
        let mut pack_ptr = &mut packed[..];
        let packed_result = pack_nibbles(&mut pack_ptr, even.as_bytes(), true, 0, table).unwrap();
        let mut unpacked = [0u8; 128];
        let mut unpack_ptr = &mut unpacked[..];
        let unpacked_result = unpack_nibbles(&mut unpack_ptr, packed_result, digits).unwrap();
        assert_eq!(unpacked_result, even.as_bytes());
    }

    proptest! {
        #[test]
        fn pack_output_length(input in proptest::collection::vec(b'0'..=b'9', 0..100)) {
            let mut output = [0u8; 64];
            let mut outptr = &mut output[..];
            let result = pack_nibbles(&mut outptr, &input, true, 0, &Bcdz::TABLE).unwrap();
            prop_assert_eq!(result.len(), input.len().div_ceil(2));
        }

        #[test]
        fn unpack_output_length(input in proptest::collection::vec(any::<u8>(), 0..50)) {
            let mut output = [0u8; 128];
            let mut outptr = &mut output[..];
            let result = unpack_nibbles(&mut outptr, &input, &Bcdz::DIGITS).unwrap();
            prop_assert_eq!(result.len(), input.len() * 2);
        }

        #[test]
        fn roundtrip_bcdz(input in "[0-9]{0,50}") {
            assert_roundtrip(&input, &Bcdz::TABLE, &Bcdz::DIGITS);
        }

        #[test]
        fn roundtrip_hex_upper(input in "[0-9A-F]{0,50}") {
            assert_roundtrip(&input, &HexUpper::TABLE, &HexUpper::DIGITS);
        }

        #[test]
        fn roundtrip_hex_lower(input in "[0-9a-f]{0,50}") {
            assert_roundtrip(&input, &HexLower::TABLE, &HexLower::DIGITS);
        }

        #[test]
        fn roundtrip_bytes(input in proptest::collection::vec(any::<u8>(), 0..50)) {
            let mut unpacked = [0u8; 128];
            let mut unpack_ptr = &mut unpacked[..];
            let unpacked_result = unpack_nibbles(&mut unpack_ptr, &input, &HexUpper::DIGITS).unwrap();
            let mut repacked = [0u8; 64];
            let mut repack_ptr = &mut repacked[..];
            let repacked_result = pack_nibbles(&mut repack_ptr, unpacked_result, true, 0, &HexUpper::TABLE).unwrap();
            prop_assert_eq!(repacked_result, input.as_slice());
        }

        #[test]
        fn odd_padding_position(input in "[0-9]{1,21}", padding in 0u8..16) {
            let odd = if input.len() % 2 == 0 { &input[..input.len()-1] } else { &input[..] };
            // Right align: padding in high nibble of first byte
            let mut out = [0u8; 64];
            let mut ptr = &mut out[..];
            let result = pack_nibbles(&mut ptr, odd.as_bytes(), true, padding, &Bcdz::TABLE).unwrap();
            prop_assert_eq!(result[0] >> 4, padding);
            // Left align: padding in low nibble of last byte
            let mut out = [0u8; 64];
            let mut ptr = &mut out[..];
            let result = pack_nibbles(&mut ptr, odd.as_bytes(), false, padding, &Bcdz::TABLE).unwrap();
            prop_assert_eq!(result[result.len()-1] & 0x0F, padding);
        }
    }
}
