#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::all_bytes_eq;
use crate::primitive::ebcdic::{EBCDIC_037_TO_ASCII, translate_bytes};
use crate::primitive::int::decode_signed_magnitude_i64;
use crate::primitive::nibble::{Bcdz, NibbleAlphabet, pack_nibbles, unpack_padded_nibbles};
use crate::primitive::validation::validate_numeric;
use crate::utils::cold_path;

/// Maximum text length of u64 or i64, including the i64 minus sign.
pub const MAX_INTEGER_TEXT_LEN: usize = 20;

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn split_signed_input(input: &[u8]) -> Result<(bool, &[u8]), Error> {
    let Some((&first, rest)) = input.split_first() else {
        cold_path();
        return Err(Error::Invalid);
    };
    let (negative, digits) = match first {
        b'-' => (true, rest),
        b'+' => {
            cold_path();
            return Err(Error::Invalid);
        }
        _ => (false, input),
    };
    if digits.is_empty() {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok((negative, digits))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_signed_decimal(input: &[u8], max_digits: usize) -> Result<(bool, &[u8]), Error> {
    let (negative, digits) = split_signed_input(input)?;
    validate_numeric(digits, 1, max_digits)?;
    Ok((negative, digits))
}

/// Return the normalized sign, output length, and significant source suffix,
/// which may still contain the decimal point.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
fn analyze_scaled_decimal(input: &[u8], scale: usize, max_digits: usize, signed: bool) -> Result<(bool, usize, &[u8]), Error> {
    let (negative, input) = split_signed_input(input)?;
    if negative && !signed {
        cold_path();
        return Err(Error::Invalid);
    }
    let mut int_digits = 0usize;
    let mut frac_digits = 0usize;
    let mut seen_dot = false;
    let mut first_nonzero = None;
    let mut digit_index = 0usize;
    let mut significant = &b""[..];
    let mut remaining = input;
    while let Some((&byte, rest)) = remaining.split_first() {
        match byte {
            b'0'..=b'9' => {
                if byte != b'0' && first_nonzero.is_none() {
                    first_nonzero = Some(digit_index);
                    significant = remaining;
                }
                if seen_dot {
                    frac_digits += 1;
                } else {
                    int_digits += 1;
                }
                digit_index += 1;
            }
            b'.' if !seen_dot => seen_dot = true,
            _ => {
                cold_path();
                return Err(Error::Invalid);
            }
        }
        remaining = rest;
    }
    if int_digits == 0 || (seen_dot && frac_digits == 0) || frac_digits > scale {
        cold_path();
        return Err(Error::Invalid);
    }
    let total_digits = int_digits.checked_add(scale).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    let digits_len = first_nonzero.map_or(1, |first| total_digits - first);
    if digits_len > max_digits {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    let negative = negative && first_nonzero.is_some();
    let out_len = digits_len.checked_add(usize::from(negative)).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    Ok((negative, out_len, significant))
}

/// Validate decimal text and return its implied-decimal encoded byte count.
/// The count includes any normalized minus sign and digits implied by `scale`.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encoded_decimal_implied_len(input: &[u8], scale: usize, max_digits: usize, signed: bool) -> Result<usize, Error> {
    analyze_scaled_decimal(input, scale, max_digits, signed).map(|(_, out_len, _)| out_len)
}

const DEC_DIGITS_LUT: &[u8; 200] = b"\
00010203040506070809\
10111213141516171819\
20212223242526272829\
30313233343536373839\
40414243444546474849\
50515253545556575859\
60616263646566676869\
70717273747576777879\
80818283848586878889\
90919293949596979899";

#[inline(always)]
fn write_pair(buf: &mut [u8; MAX_INTEGER_TEXT_LEN], pos: &mut usize, value: usize) {
    let idx = value * 2;
    *pos -= 2;
    buf[*pos..*pos + 2].copy_from_slice(&DEC_DIGITS_LUT[idx..idx + 2]);
}

#[inline(always)]
fn write_quad(buf: &mut [u8; MAX_INTEGER_TEXT_LEN], pos: &mut usize, value: usize) {
    let hi = (value / 100) * 2;
    let lo = (value % 100) * 2;
    *pos -= 4;
    buf[*pos..*pos + 2].copy_from_slice(&DEC_DIGITS_LUT[hi..hi + 2]);
    buf[*pos + 2..*pos + 4].copy_from_slice(&DEC_DIGITS_LUT[lo..lo + 2]);
}

#[inline(always)]
fn pair4_u64(buf: &mut [u8; MAX_INTEGER_TEXT_LEN], mut value: u64) -> usize {
    let mut pos = MAX_INTEGER_TEXT_LEN;

    while value >= 10_000 {
        let rem = (value % 10_000) as usize;
        value /= 10_000;
        write_quad(buf, &mut pos, rem);
    }

    if value >= 100 {
        write_pair(buf, &mut pos, (value % 100) as usize);
        value /= 100;
    }

    if value < 10 {
        pos -= 1;
        buf[pos] = b'0' + value as u8;
    } else {
        write_pair(buf, &mut pos, value as usize);
    }

    pos
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn format_u64(output: &mut [u8; MAX_INTEGER_TEXT_LEN], value: u64) -> &[u8] {
    let start = pair4_u64(output, value);
    &output[start..]
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn format_i64(output: &mut [u8; MAX_INTEGER_TEXT_LEN], value: i64) -> &[u8] {
    let negative = value < 0;
    let mut pos = pair4_u64(output, value.unsigned_abs());
    if negative {
        pos = pos.saturating_sub(1);
        output[pos] = b'-';
    }
    &output[pos..]
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_sign(input: &mut &[u8], pos: u8, neg: u8) -> Result<bool, Error> {
    let sign = input.split_off(..1).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?[0];
    match sign {
        s if s == pos => Ok(false),
        s if s == neg => Ok(true),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_sign(output: &mut &mut [u8], negative: bool, pos: u8, neg: u8) -> Result<(), Error> {
    let sign = output.split_off_mut(..1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    sign[0] = if negative { neg } else { pos };
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_negative_prefix(output: &mut &mut [u8], negative: bool, neg: u8) -> Result<(), Error> {
    if !negative {
        return Ok(());
    }
    let sign = output.split_off_mut(..1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    sign[0] = neg;
    Ok(())
}

/// Consume a leading negative marker if present, returning whether it was found.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_negative_prefix(input: &mut &[u8], neg: u8) -> bool {
    match *input {
        [first, rest @ ..] if *first == neg => {
            *input = rest;
            true
        }
        _ => false,
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn prepend_minus<'a>(output: &mut &'a mut [u8], digits: &[u8]) -> Result<&'a mut [u8], Error> {
    let len = digits.len().checked_add(1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let (sign, out) = buf.split_at_mut(1);
    sign[0] = b'-';
    out.copy_from_slice(digits);
    Ok(buf)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_implied<'a>(
    output: &mut &'a mut [u8],
    input: &[u8],
    scale: usize,
    max_digits: usize,
    signed: bool,
) -> Result<&'a mut [u8], Error> {
    let (negative, out_len, significant) = analyze_scaled_decimal(input, scale, max_digits, signed)?;
    let buf = output.split_off_mut(..out_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let mut out = buf.iter_mut();
    for sign in out.by_ref().take(usize::from(negative)) {
        *sign = b'-';
    }
    let digits = significant.iter().copied().filter(|&byte| byte != b'.');
    for (byte, out) in digits.zip(out.by_ref()) {
        *out = byte;
    }
    out.into_slice().fill(b'0');
    Ok(buf)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_implied<'a>(output: &mut &'a mut [u8], input: &[u8], scale: usize) -> Result<&'a mut [u8], Error> {
    let (negative, digits) = split_signed_input(input)?;
    validate_numeric(digits, 1, usize::MAX)?;
    decode_decimal_implied_digits(output, digits, negative, scale)
}

/// Place the implied decimal point in nonempty, prevalidated ASCII digits.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn decode_decimal_implied_digits<'a>(
    output: &mut &'a mut [u8],
    mut digits: &[u8],
    negative: bool,
    scale: usize,
) -> Result<&'a mut [u8], Error> {
    debug_assert!(validate_numeric(digits, 1, usize::MAX).is_ok());
    while let Some(rest) = digits.strip_prefix(b"0") {
        digits = rest;
    }
    if digits.is_empty() {
        return crate::primitive::bytes::copy_bytes(output, b"0");
    }
    let (integer, mut fraction) = digits.split_at(digits.len().saturating_sub(scale));
    while let Some(rest) = fraction.strip_suffix(b"0") {
        fraction = rest;
    }
    let integer = if integer.is_empty() { b"0" } else { integer };
    let prefix_zeros = scale.saturating_sub(digits.len());
    let fraction_len = prefix_zeros.checked_add(fraction.len()).ok_or_else(|| {
        cold_path();
        Error::Internal
    })?;
    let out_len = integer
        .len()
        .checked_add(usize::from(negative))
        .and_then(|len| len.checked_add(usize::from(fraction_len != 0)))
        .and_then(|len| len.checked_add(fraction_len))
        .ok_or_else(|| {
            cold_path();
            Error::Internal
        })?;
    let buf = output.split_off_mut(..out_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let mut out = buf.iter_mut();
    for sign in out.by_ref().take(usize::from(negative)) {
        *sign = b'-';
    }
    for (&digit, out) in integer.iter().zip(out.by_ref()) {
        *out = digit;
    }
    for dot in out.by_ref().take(usize::from(fraction_len != 0)) {
        *dot = b'.';
    }
    for zero in out.by_ref().take(prefix_zeros) {
        *zero = b'0';
    }
    for (&digit, out) in fraction.iter().zip(out) {
        *out = digit;
    }
    Ok(buf)
}

/// Encode a prevalidated decimal digit (0-9) with its EBCDIC sign zone.
/// Debug builds assert the digit domain; release does not validate it.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_overpunch_digit(negative: bool, digit: u8) -> u8 {
    debug_assert!(digit <= 9, "overpunch requires a decimal digit");
    (encode_packed_sign(negative, true) << 4) | digit
}

/// Encode D for negative values, otherwise C for signed or F for unsigned values.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_packed_sign(negative: bool, signed: bool) -> u8 {
    if negative {
        0x0D
    } else if signed {
        0x0C
    } else {
        0x0F
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_overpunch_digit(input: u8) -> Result<(bool, u8), Error> {
    let negative = decode_packed_sign(input >> 4)?;
    let digit = input & 0x0F;
    if digit > 9 {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok((negative, digit))
}

/// Decode a packed-decimal sign or EBCDIC overpunch zone: A/C/E/F are
/// positive and B/D are negative.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_packed_sign(input: u8) -> Result<bool, Error> {
    match input {
        0x0A | 0x0C | 0x0E | 0x0F => Ok(false),
        0x0B | 0x0D => Ok(true),
        _ => {
            cold_path();
            Err(Error::Invalid)
        }
    }
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn packed_decimal_max_digits(bytes_len: usize) -> Result<usize, Error> {
    bytes_len.checked_mul(2).and_then(|v| v.checked_sub(1)).ok_or_else(|| {
        cold_path();
        Error::Invalid
    })
}

/// Encode prevalidated ASCII digits, checking they fit the zoned output width.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_ebcdic_zoned_digits(output: &mut &mut [u8], digits: &[u8], negative: bool, len: usize) -> Result<(), Error> {
    let output = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    debug_assert!(validate_numeric(digits, 1, usize::MAX).is_ok());
    if digits.len() > output.len() {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    if let ([body @ .., last], [out @ .., last_out]) = (digits, output) {
        let pad = out.len().saturating_sub(body.len());
        let (padding, target) = out.split_at_mut(pad);
        padding.fill(0xF0);
        for (out, &digit) in target.iter_mut().zip(body) {
            *out = digit.wrapping_add(0xC0);
        }
        *last_out = encode_overpunch_digit(negative, last.wrapping_sub(b'0'));
    }
    Ok(())
}

/// Encode prevalidated ASCII digits, checking they fit the packed output width.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub(crate) fn encode_decimal_packed_digits(
    output: &mut &mut [u8],
    digits: &[u8],
    negative: bool,
    signed: bool,
    len: usize,
) -> Result<(), Error> {
    let output = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    debug_assert!(validate_numeric(digits, 1, usize::MAX).is_ok());
    if digits.len() > output.len().saturating_mul(2).saturating_sub(1) {
        cold_path();
        return Err(Error::InvalidValueLength);
    }
    let used_bytes = digits.len() / 2 + 1;
    let prefix_bytes = output.len().saturating_sub(used_bytes);
    let (prefix, tail) = output.split_at_mut(prefix_bytes);
    prefix.fill(0);
    let sign = encode_packed_sign(negative, signed);
    if digits.len().is_multiple_of(2) {
        let ([first, rest @ ..], [first_out, tail @ ..]) = (digits, tail) else {
            cold_path();
            return Err(Error::Invalid);
        };
        *first_out = first.wrapping_sub(b'0');
        pack_nibbles(&mut &mut *tail, rest, false, sign, &Bcdz::NIBBLES)?;
    } else {
        pack_nibbles(&mut &mut *tail, digits, false, sign, &Bcdz::NIBBLES)?;
    }
    Ok(())
}

/// Parse nonempty ASCII decimal digits, accepting leading zeroes. Signs, non-digits
/// and arithmetic overflow return `Invalid`. This is a checked parsing boundary.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_u64(bytes: &[u8]) -> Result<u64, Error> {
    parse_decimal_digits(bytes, b'0')
}

#[inline(always)]
fn parse_decimal_digits(bytes: &[u8], zero: u8) -> Result<u64, Error> {
    if bytes.is_empty() {
        cold_path();
        return Err(Error::Invalid);
    }
    bytes.iter().try_fold(0u64, |value, &byte| {
        let digit = byte.wrapping_sub(zero);
        if digit > 9 {
            cold_path();
            return Err(Error::Invalid);
        }
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u64::from(digit)))
            .ok_or_else(|| {
                cold_path();
                Error::Invalid
            })
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_usize(bytes: &[u8]) -> Result<usize, Error> {
    usize::try_from(parse_u64(bytes)?).map_err(|_| {
        cold_path();
        Error::Invalid
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_i64(bytes: &[u8]) -> Result<i64, Error> {
    let (negative, digits) = split_signed_input(bytes)?;
    let magnitude = parse_u64(digits)?;
    decode_signed_magnitude_i64(negative, magnitude)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_ascii_fixed(output: &mut &mut [u8], value: usize, len: usize) -> Result<(), Error> {
    encode_decimal_fixed(output, value, len, b'0')
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_ebcdic_fixed(output: &mut &mut [u8], value: usize, len: usize) -> Result<(), Error> {
    encode_decimal_fixed(output, value, len, 0xF0)
}

#[inline(always)]
fn encode_decimal_fixed(output: &mut &mut [u8], mut value: usize, len: usize, zero: u8) -> Result<(), Error> {
    let limit = u32::try_from(len).ok().and_then(|len| 10usize.checked_pow(len));
    if len == 0 || limit.is_some_and(|limit| value >= limit) {
        cold_path();
        return Err(Error::Invalid);
    }
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    for byte in buf.iter_mut().rev() {
        *byte = zero + (value % 10) as u8;
        value /= 10;
    }
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_ebcdic_blank_zero_fixed(output: &mut &mut [u8], value: usize, len: usize) -> Result<(), Error> {
    if value == 0 {
        let buf = output.split_off_mut(..len).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })?;
        buf.fill(0x40);
        return Ok(());
    }
    encode_decimal_ebcdic_fixed(output, value, len)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ascii_fixed(input: &mut &[u8], len: usize) -> Result<usize, Error> {
    decode_decimal_fixed(input, len, b'0')
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ebcdic_fixed(input: &mut &[u8], len: usize) -> Result<usize, Error> {
    decode_decimal_fixed(input, len, 0xF0)
}

#[inline(always)]
fn decode_decimal_fixed(input: &mut &[u8], len: usize, zero: u8) -> Result<usize, Error> {
    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    usize::try_from(parse_decimal_digits(bytes, zero)?).map_err(|_| {
        cold_path();
        Error::Invalid
    })
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ebcdic_blank_zero_fixed(input: &mut &[u8], len: usize) -> Result<usize, Error> {
    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    if all_bytes_eq(bytes, 0x40) {
        return Ok(0);
    }
    let mut nested = bytes;
    decode_decimal_ebcdic_fixed(&mut nested, len)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_ebcdic_zoned_decimal(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    let (negative, digits) = parse_signed_decimal(input, len)?;
    encode_ebcdic_zoned_digits(output, digits, negative, len)
}

/// Reserve `len + 1` scratch bytes and return the canonical signed digits within
/// that area. The returned slice may start after the beginning of the reservation.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_ebcdic_zoned_decimal<'a>(input: &mut &[u8], output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    let input = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    let Some((&last, body)) = input.split_last() else {
        cold_path();
        return Err(Error::Invalid);
    };
    let buf = output.split_off_mut(..input.len() + 1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let (negative, last_digit) = decode_overpunch_digit(last)?;
    let [_, digits @ .., last_out] = &mut *buf else {
        cold_path();
        return Err(Error::BufferOverflow);
    };
    translate_bytes(digits, body, &EBCDIC_037_TO_ASCII)?;
    validate_numeric(&*digits, body.len(), body.len())?;
    *last_out = b'0' + last_digit;
    Ok(canonical_signed_digits(buf, negative))
}

// The first byte is reserved for an optional minus; the rest are ASCII digits.
#[inline(always)]
fn canonical_signed_digits(mut buf: &mut [u8], negative: bool) -> &mut [u8] {
    debug_assert!(buf.len() >= 2);
    while matches!(buf, [_, b'0', _, ..]) {
        buf = &mut buf[1..];
    }
    if negative && !matches!(buf, [_, b'0']) {
        if let [sign, ..] = buf {
            *sign = b'-';
        }
        buf
    } else {
        match buf {
            [_, digits @ ..] => digits,
            [] => buf,
        }
    }
}

#[inline(always)]
fn decode_decimal_packed_common<'a>(input: &[u8], output: &mut &'a mut [u8], signed: bool) -> Result<&'a mut [u8], Error> {
    let Some(&last) = input.last() else {
        cold_path();
        return Err(Error::Invalid);
    };
    let buf = output.split_off_mut(..input.len() * 2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let sign = last & 0x0F;
    let negative = decode_packed_sign(sign)?;
    if negative && !signed {
        cold_path();
        return Err(Error::Invalid);
    }
    let [_, digits @ ..] = &mut *buf else {
        cold_path();
        return Err(Error::BufferOverflow);
    };
    let len = digits.len();
    unpack_padded_nibbles(&mut &mut *digits, input, len, false, sign, &Bcdz::DIGITS)?;
    validate_numeric(&*digits, len, len)?;
    Ok(canonical_signed_digits(buf, negative))
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_packed_fixed(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    validate_numeric(input, 1, packed_decimal_max_digits(len)?)?;
    encode_decimal_packed_digits(output, input, false, false, len)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_packed_signed_fixed(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    let (negative, digits) = parse_signed_decimal(input, packed_decimal_max_digits(len)?)?;
    encode_decimal_packed_digits(output, digits, negative, true, len)
}

/// Reserve `2 * len` scratch bytes and return canonical unsigned digits within
/// that area. The returned slice may start after the beginning of the reservation.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_packed_fixed<'a>(input: &mut &[u8], output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    let input = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    decode_decimal_packed_common(input, output, false)
}

/// Reserve `2 * len` scratch bytes and return canonical signed digits within
/// that area. The returned slice may start after the beginning of the reservation.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_packed_signed_fixed<'a>(input: &mut &[u8], output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    let input = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    decode_decimal_packed_common(input, output, true)
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_INTEGER_TEXT_LEN, decode_decimal_ascii_fixed, decode_decimal_ebcdic_blank_zero_fixed, decode_decimal_ebcdic_fixed,
        decode_decimal_implied, decode_decimal_packed_fixed, decode_decimal_packed_signed_fixed, decode_ebcdic_zoned_decimal,
        decode_negative_prefix, decode_sign, encode_decimal_ascii_fixed, encode_decimal_ebcdic_blank_zero_fixed,
        encode_decimal_ebcdic_fixed, encode_decimal_implied, encode_decimal_packed_fixed, encode_decimal_packed_signed_fixed,
        encode_ebcdic_zoned_decimal, encode_negative_prefix, encode_sign, encoded_decimal_implied_len, format_i64, format_u64,
        parse_signed_decimal, prepend_minus, split_signed_input,
    };
    use crate::Error;

    fn encode<const N: usize>(f: impl FnOnce(&mut &mut [u8]) -> Result<(), Error>) -> Result<[u8; N], Error> {
        let mut out = [0u8; N];
        let mut out_ptr = out.as_mut_slice();
        f(&mut out_ptr)?;
        Ok(out)
    }

    fn decode_ascii<const N: usize>(input: &[u8]) -> Result<usize, Error> {
        let mut input = input;
        decode_decimal_ascii_fixed(&mut input, N)
    }

    fn decode_ebcdic<const N: usize>(input: &[u8]) -> Result<usize, Error> {
        let mut input = input;
        decode_decimal_ebcdic_fixed(&mut input, N)
    }

    fn decode_blankable_ebcdic<const N: usize>(input: &[u8]) -> Result<usize, Error> {
        let mut input = input;
        decode_decimal_ebcdic_blank_zero_fixed(&mut input, N)
    }

    fn encode_signed_ebcdic_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_ebcdic_zoned_decimal(out, input, N))
    }

    fn decode_signed_ebcdic_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_INTEGER_TEXT_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_ebcdic_zoned_decimal(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_packed_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_decimal_packed_fixed(out, input, N))
    }

    fn decode_packed_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_INTEGER_TEXT_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_decimal_packed_fixed(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_signed_packed_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_decimal_packed_signed_fixed(out, input, N))
    }

    fn decode_signed_packed_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_INTEGER_TEXT_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_decimal_packed_signed_fixed(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_implied_ascii(input: &[u8], scale: usize, max_digits: usize, signed: bool) -> Result<Vec<u8>, Error> {
        let len = encoded_decimal_implied_len(input, scale, max_digits, signed)?;
        let mut output = [0u8; MAX_INTEGER_TEXT_LEN + 1];
        let mut out_ptr = output.as_mut_slice();
        let encoded = encode_decimal_implied(&mut out_ptr, input, scale, max_digits, signed)?;
        assert_eq!(encoded.len(), len);
        Ok(encoded.to_vec())
    }

    fn decode_implied_ascii(input: &[u8], scale: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; MAX_INTEGER_TEXT_LEN + 2];
        let mut out_ptr = output.as_mut_slice();
        Ok(decode_decimal_implied(&mut out_ptr, input, scale)?.to_vec())
    }

    #[test]
    fn test_validate_signed_decimal_and_implied() {
        assert_eq!(split_signed_input(b"12"), Ok((false, &b"12"[..])));
        assert_eq!(split_signed_input(b"-12"), Ok((true, &b"12"[..])));
        assert_eq!(split_signed_input(b"+12"), Err(Error::Invalid));
        assert_eq!(split_signed_input(b"-"), Err(Error::Invalid));

        assert_eq!(parse_signed_decimal(b"12", 2), Ok((false, &b"12"[..])));
        assert_eq!(parse_signed_decimal(b"-12", 2), Ok((true, &b"12"[..])));
        assert_eq!(parse_signed_decimal(b"-123", 2), Err(Error::InvalidValueLength));
        assert_eq!(parse_signed_decimal(b"+12", 2), Err(Error::Invalid));

        assert_eq!(encoded_decimal_implied_len(b"123.45", 2, 5, false), Ok(5));
        assert_eq!(encoded_decimal_implied_len(b"-0.05", 2, 5, true), Ok(2));
        assert_eq!(encoded_decimal_implied_len(b"1234.56", 2, 5, false), Err(Error::InvalidValueLength));
        assert_eq!(encoded_decimal_implied_len(b".5", 2, 5, false), Err(Error::Invalid));
        assert_eq!(encoded_decimal_implied_len(b"1.", 2, 5, false), Err(Error::Invalid));
        assert_eq!(encoded_decimal_implied_len(b"+1", 2, 5, true), Err(Error::Invalid));
    }

    #[test]
    fn test_integer_parsers() {
        use super::{parse_i64, parse_u64, parse_usize};
        for (digits, value) in [("0", 0), ("000000000000000000000001", 1), ("18446744073709551615", u64::MAX)] {
            assert_eq!(parse_u64(digits.as_bytes()), Ok(value));
        }
        for digits in ["", "+1", "0+1", "-1", "1 ", "1.0", "18446744073709551616"] {
            assert_eq!(parse_u64(digits.as_bytes()), Err(Error::Invalid));
        }
        assert_eq!(parse_i64(b"-9223372036854775808"), Ok(i64::MIN));
        assert_eq!(parse_i64(b"9223372036854775807"), Ok(i64::MAX));
        assert_eq!(parse_i64(b"-0"), Ok(0));
        for digits in ["-", "+1", "-+1", "9223372036854775808", "-9223372036854775809"] {
            assert_eq!(parse_i64(digits.as_bytes()), Err(Error::Invalid));
        }
        assert_eq!(parse_usize(usize::MAX.to_string().as_bytes()), Ok(usize::MAX));
        assert_eq!(
            parse_usize((u128::from(usize::MAX as u64) + 1).to_string().as_bytes()),
            Err(Error::Invalid)
        );
        for zero in [b'0', 0xF0] {
            let value = usize::MAX;
            let mut storage = [0xAA; 32];
            let mut out = storage.as_mut_slice();
            super::encode_decimal_fixed(&mut out, value, 24, zero).unwrap();
            assert_eq!(out, &[0xAA; 8]);
            let mut input = storage.as_slice();
            assert_eq!(super::decode_decimal_fixed(&mut input, 24, zero), Ok(value));
            assert_eq!(input, &[0xAA; 8]);
            assert_eq!(
                super::decode_decimal_fixed(&mut &[zero.wrapping_sub(1)][..], 1, zero),
                Err(Error::Invalid)
            );
        }
    }

    pub(super) fn check_fixed_decimal(value: usize, width: usize, capacity: usize) {
        let digits = value.to_string();
        for zero in [b'0', 0xF0] {
            let mut storage = [0xAA; 66];
            let mut out = &mut storage[..capacity];
            let result = super::encode_decimal_fixed(&mut out, value, width, zero);
            let expected = if digits.len() > width {
                Err(Error::Invalid)
            } else if width > capacity {
                Err(Error::BufferOverflow)
            } else {
                Ok(())
            };
            assert_eq!(result, expected, "value={value}, width={width}, capacity={capacity}");
            let written = if result.is_ok() { width } else { 0 };
            assert_eq!(out.len(), capacity - written);
            assert!(storage[written..].iter().all(|&byte| byte == 0xAA));
            if result.is_ok() {
                let expected: Vec<_> = format!("{value:0width$}").bytes().map(|digit| digit - b'0' + zero).collect();
                assert_eq!(&storage[..width], expected);
                let mut wire = &storage[..width];
                assert_eq!(super::decode_decimal_fixed(&mut wire, width, zero), Ok(value));
                assert!(wire.is_empty());
            }
        }
    }

    #[test]
    fn fixed_decimal_boundaries() {
        let mut values = vec![0, 1, usize::MAX];
        for exponent in 1..=20 {
            if let Some(power) = 10usize.checked_pow(exponent) {
                values.extend([power - 1, power, power + 1]);
            }
        }
        for value in values {
            for width in 0usize..=32 {
                for capacity in [0, width.saturating_sub(1), width, 34] {
                    check_fixed_decimal(value, width, capacity);
                }
            }
        }
        for width in [usize::MAX, usize::MAX - 1] {
            assert_eq!(encode_decimal_ascii_fixed(&mut &mut [][..], 0, width), Err(Error::BufferOverflow));
            assert_eq!(
                encode_decimal_ebcdic_fixed(&mut &mut [][..], usize::MAX, width),
                Err(Error::BufferOverflow)
            );
        }
    }

    #[test]
    fn test_decimal_signs_and_digits() {
        use super::{decode_overpunch_digit, decode_packed_sign, encode_overpunch_digit, encode_packed_sign};
        for byte in 0u8..=255 {
            let sign = match byte >> 4 {
                0xA | 0xC | 0xE | 0xF => Ok(false),
                0xB | 0xD => Ok(true),
                _ => Err(Error::Invalid),
            };
            assert_eq!(decode_packed_sign(byte >> 4), sign);
            let expected = if byte & 0xF <= 9 {
                sign.map(|negative| (negative, byte & 0xF))
            } else {
                Err(Error::Invalid)
            };
            assert_eq!(decode_overpunch_digit(byte), expected);
        }
        for digit in 0..=9 {
            for negative in [false, true] {
                assert_eq!(
                    decode_overpunch_digit(encode_overpunch_digit(negative, digit)),
                    Ok((negative, digit))
                );
            }
        }
        assert_eq!(encode_packed_sign(false, true), 0xC);
        assert_eq!(encode_packed_sign(true, true), 0xD);
        assert_eq!(encode_packed_sign(false, false), 0xF);
    }

    #[test]
    fn test_packed_and_zoned_widths_and_signs() {
        for sign in 0u8..=15 {
            let expected = match sign {
                0xA | 0xC | 0xE | 0xF => Ok(b"7".as_slice()),
                0xB | 0xD => Ok(b"-7".as_slice()),
                _ => Err(Error::Invalid),
            };
            assert_eq!(decode_signed_packed_ascii::<1>(&[0x70 | sign]).as_deref(), expected.as_deref());
            assert_eq!(decode_signed_ebcdic_ascii::<1>(&[(sign << 4) | 7]).as_deref(), expected.as_deref());
        }
        for digits in ["0", "00", "7", "12", "123", "1234", "00012", "-0", "-000", "-12", "-1234"] {
            let wire = encode_signed_packed_ascii::<3>(digits.as_bytes()).unwrap();
            let expected = digits.parse::<i64>().unwrap().to_string().into_bytes();
            assert_eq!(decode_signed_packed_ascii::<3>(&wire), Ok(expected.clone()));
            let wire = encode_signed_ebcdic_ascii::<5>(digits.as_bytes()).unwrap();
            assert_eq!(decode_signed_ebcdic_ascii::<5>(&wire), Ok(expected));
        }
        for wire in [[0xFA, 0x1C], [0x1A, 0x2C], [0x12, 0xFC]] {
            assert_eq!(decode_signed_packed_ascii::<2>(&wire), Err(Error::Invalid));
        }
    }

    #[test]
    fn test_format_decimal() {
        let mut buf = [0u8; MAX_INTEGER_TEXT_LEN];
        for &(value, expected) in &[
            (0_u64, "0"),
            (7, "7"),
            (99, "99"),
            (100, "100"),
            (9_999, "9999"),
            (10_000, "10000"),
            (12_345_678, "12345678"),
            (u64::MAX, "18446744073709551615"),
        ] {
            assert_eq!(format_u64(&mut buf, value), expected.as_bytes());
        }

        for &(value, expected) in &[
            (0_i64, "0"),
            (7, "7"),
            (-7, "-7"),
            (99, "99"),
            (-99, "-99"),
            (10_000, "10000"),
            (-10_000, "-10000"),
            (i64::MAX, "9223372036854775807"),
            (i64::MIN, "-9223372036854775808"),
        ] {
            assert_eq!(format_i64(&mut buf, value), expected.as_bytes());
        }
    }

    #[test]
    fn test_fixed_width_decimal_codecs() {
        for &(value, ascii, ebcdic) in &[(0usize, *b"0", [0xF0]), (9, *b"9", [0xF9])] {
            assert_eq!(encode::<1>(|out| encode_decimal_ascii_fixed(out, value, 1)), Ok(ascii));
            assert_eq!(encode::<1>(|out| encode_decimal_ebcdic_fixed(out, value, 1)), Ok(ebcdic));
            assert_eq!(decode_ascii::<1>(&ascii), Ok(value));
            assert_eq!(decode_ebcdic::<1>(&ebcdic), Ok(value));
        }

        assert_eq!(encode::<4>(|out| encode_decimal_ascii_fixed(out, 42, 4)), Ok(*b"0042"));
        assert_eq!(
            encode::<4>(|out| encode_decimal_ebcdic_fixed(out, 42, 4)),
            Ok([0xF0, 0xF0, 0xF4, 0xF2])
        );
        assert_eq!(encode::<2>(|out| encode_decimal_ascii_fixed(out, 99, 2)), Ok(*b"99"));
        assert_eq!(encode::<2>(|out| encode_decimal_ebcdic_fixed(out, 99, 2)), Ok([0xF9, 0xF9]));
        assert_eq!(decode_ascii::<4>(b"0042"), Ok(42));
        assert_eq!(decode_ebcdic::<4>(&[0xF0, 0xF0, 0xF4, 0xF2]), Ok(42));
        assert_eq!(decode_ascii::<2>(b"99"), Ok(99));
        assert_eq!(decode_ebcdic::<2>(&[0xF9, 0xF9]), Ok(99));

        assert_eq!(encode::<1>(|out| encode_decimal_ascii_fixed(out, 1, 2)), Err(Error::BufferOverflow));
        assert_eq!(
            encode::<1>(|out| encode_decimal_ebcdic_fixed(out, 1, 2)),
            Err(Error::BufferOverflow)
        );
        assert_eq!(encode::<2>(|out| encode_decimal_ascii_fixed(out, 100, 2)), Err(Error::Invalid));
        assert_eq!(encode::<2>(|out| encode_decimal_ebcdic_fixed(out, 100, 2)), Err(Error::Invalid));
        assert_eq!(decode_ascii::<2>(b"A0"), Err(Error::Invalid));
        assert_eq!(decode_ebcdic::<2>(&[0xF0, b'0']), Err(Error::Invalid));
        assert_eq!(decode_ascii::<2>(b"1"), Err(Error::UnexpectedEof));
        assert_eq!(decode_ebcdic::<2>(&[0xF1]), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_blankable_fixed_width_ebcdic_decimal_codecs() {
        assert_eq!(
            encode::<2>(|out| encode_decimal_ebcdic_blank_zero_fixed(out, 0, 2)),
            Ok([0x40, 0x40])
        );
        assert_eq!(
            encode::<2>(|out| encode_decimal_ebcdic_blank_zero_fixed(out, 42, 2)),
            Ok([0xF4, 0xF2])
        );
        assert_eq!(decode_blankable_ebcdic::<2>(&[0x40, 0x40]), Ok(0));
        assert_eq!(decode_blankable_ebcdic::<2>(&[0xF4, 0xF2]), Ok(42));
        assert_eq!(decode_blankable_ebcdic::<2>(&[0x40, 0xF2]), Err(Error::Invalid));
        assert_eq!(
            encode::<1>(|out| encode_decimal_ebcdic_blank_zero_fixed(out, 0, 2)),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_blankable_ebcdic::<2>(&[0x40]), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_sign_prefix_helpers() {
        assert_eq!(encode::<1>(|out| encode_sign(out, false, b'C', b'D')), Ok(*b"C"));
        assert_eq!(encode::<1>(|out| encode_sign(out, true, b'C', b'D')), Ok(*b"D"));
        assert_eq!(encode::<0>(|out| encode_sign(out, false, b'C', b'D')), Err(Error::BufferOverflow));
        let mut output = [0xAAu8; 1];
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(encode_negative_prefix(&mut out_ptr, false, b'-'), Ok(()));
        assert_eq!(out_ptr.len(), 1);
        assert_eq!(output, [0xAA]);
        let mut output = [0u8; 1];
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(encode_negative_prefix(&mut out_ptr, true, b'-'), Ok(()));
        assert_eq!(out_ptr.len(), 0);
        assert_eq!(output, *b"-");
        let mut input = &b"C123"[..];
        assert_eq!(decode_sign(&mut input, b'C', b'D'), Ok(false));
        assert_eq!(input, b"123");
        let mut input = &b"-123"[..];
        assert!(decode_negative_prefix(&mut input, b'-'));
        assert_eq!(input, b"123");
        let mut input = &b"123"[..];
        assert!(!decode_negative_prefix(&mut input, b'-'));
        assert_eq!(input, b"123");
    }

    #[test]
    fn test_prepend_minus() {
        let mut output = [0u8; 4];
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(prepend_minus(&mut out_ptr, b"12").map(|v| v.to_vec()), Ok(b"-12".to_vec()));
        let mut output = [0u8; 2];
        let mut out_ptr = output.as_mut_slice();
        assert_eq!(prepend_minus(&mut out_ptr, b"12").map(|v| v.to_vec()), Err(Error::BufferOverflow));
    }

    #[test]
    fn test_fixed_width_signed_ebcdic_ascii_codecs() {
        assert_eq!(encode_signed_ebcdic_ascii::<2>(b"-7"), Ok([0xF0, 0xD7]));
        assert_eq!(encode_signed_ebcdic_ascii::<3>(b"12"), Ok([0xF0, 0xF1, 0xC2]));
        assert_eq!(encode_signed_ebcdic_ascii::<1>(b"0"), Ok([0xC0]));
        assert_eq!(decode_signed_ebcdic_ascii::<2>(b"\xF0\xD7"), Ok(b"-7".to_vec()));
        assert_eq!(decode_signed_ebcdic_ascii::<3>(b"\xF0\xF1\xC2"), Ok(b"12".to_vec()));
        assert_eq!(decode_signed_ebcdic_ascii::<3>(b"\xF0\xF0\xC0"), Ok(b"0".to_vec()));
        assert_eq!(decode_signed_ebcdic_ascii::<2>(b"\xF1\xB2"), Ok(b"-12".to_vec()));
        assert_eq!(encode_signed_ebcdic_ascii::<2>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_signed_ebcdic_ascii::<2>(b"123"), Err(Error::InvalidValueLength));
        let mut short_input = &b"\xF0\xC0"[..];
        let mut short = [0u8; 2];
        let mut short_ptr = short.as_mut_slice();
        assert_eq!(
            decode_ebcdic_zoned_decimal(&mut short_input, &mut short_ptr, 2),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_signed_ebcdic_ascii::<2>(b"\xC1\xC2"), Err(Error::Invalid));
        assert_eq!(decode_signed_ebcdic_ascii::<2>(b"\xF1"), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_fixed_width_packed_ascii_codecs() {
        assert_eq!(encode_packed_ascii::<2>(b"12"), Ok([0x01, 0x2F]));
        assert_eq!(encode_packed_ascii::<2>(b"123"), Ok([0x12, 0x3F]));
        assert_eq!(encode_packed_ascii::<1>(b"0"), Ok([0x0F]));
        assert_eq!(decode_packed_ascii::<2>(b"\x01\x2C"), Ok(b"12".to_vec()));
        assert_eq!(decode_packed_ascii::<2>(b"\x01\x2F"), Ok(b"12".to_vec()));
        assert_eq!(decode_packed_ascii::<2>(b"\x00\x0C"), Ok(b"0".to_vec()));
        assert_eq!(decode_packed_ascii::<0>(b""), Err(Error::Invalid));
        assert_eq!(encode_packed_ascii::<2>(b"-7"), Err(Error::Invalid));
        assert_eq!(encode_packed_ascii::<2>(b"1234"), Err(Error::InvalidValueLength));
        let mut short_input = &b"\x01\x2C"[..];
        let mut short = [0u8; 2];
        let mut short_ptr = short.as_mut_slice();
        assert_eq!(
            decode_decimal_packed_fixed(&mut short_input, &mut short_ptr, 2),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_packed_ascii::<2>(b"\x01\x2D"), Err(Error::Invalid));
        assert_eq!(decode_packed_ascii::<2>(b"\x1A\x2C"), Err(Error::Invalid));
        assert_eq!(decode_packed_ascii::<2>(b"\x12"), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_fixed_width_signed_packed_ascii_codecs() {
        assert_eq!(encode_signed_packed_ascii::<2>(b"-7"), Ok([0x00, 0x7D]));
        assert_eq!(encode_signed_packed_ascii::<2>(b"12"), Ok([0x01, 0x2C]));
        assert_eq!(encode_signed_packed_ascii::<1>(b"0"), Ok([0x0C]));
        assert_eq!(decode_signed_packed_ascii::<2>(b"\x00\x7D"), Ok(b"-7".to_vec()));
        assert_eq!(decode_signed_packed_ascii::<2>(b"\x00\x0D"), Ok(b"0".to_vec()));
        assert_eq!(decode_signed_packed_ascii::<2>(b"\x01\x2B"), Ok(b"-12".to_vec()));
        assert_eq!(decode_signed_packed_ascii::<0>(b""), Err(Error::Invalid));
        assert_eq!(encode_signed_packed_ascii::<2>(b"+7"), Err(Error::Invalid));
        assert_eq!(encode_signed_packed_ascii::<2>(b"1234"), Err(Error::InvalidValueLength));
        let mut short_input = &b"\x01\x2C"[..];
        let mut short = [0u8; 3];
        let mut short_ptr = short.as_mut_slice();
        assert_eq!(
            decode_decimal_packed_signed_fixed(&mut short_input, &mut short_ptr, 2),
            Err(Error::BufferOverflow)
        );
        assert_eq!(decode_signed_packed_ascii::<2>(b"\x1A\x2C"), Err(Error::Invalid));
        assert_eq!(decode_signed_packed_ascii::<2>(b"\x12"), Err(Error::UnexpectedEof));
    }

    #[test]
    fn test_implied_decimal_capacity_and_scale_limits() {
        for (input, wire, canonical, scale) in [
            (&b"-0.00"[..], &b"0"[..], &b"0"[..], 2),
            (b"0001.200", b"1200", b"1.2", 3),
            (b"-0.00100", b"-100", b"-0.001", 5),
            (b"-0012", b"-12", b"-12", 0),
        ] {
            assert_eq!(encoded_decimal_implied_len(input, scale, 20, true), Ok(wire.len()));
            let mut encoded = [0xAA; 24];
            let mut out = &mut encoded[..wire.len() + 1];
            assert_eq!(encode_decimal_implied(&mut out, input, scale, 20, true).map(|s| &*s), Ok(wire));
            assert_eq!(out, [0xAA]);
            let mut out = &mut encoded[..wire.len() - 1];
            assert_eq!(encode_decimal_implied(&mut out, input, scale, 20, true), Err(Error::BufferOverflow));
            let mut decoded = [0xAA; 24];
            let mut out = &mut decoded[..canonical.len() + 1];
            assert_eq!(decode_decimal_implied(&mut out, wire, scale).map(|s| &*s), Ok(canonical));
            assert_eq!(out, [0xAA]);
            let mut out = &mut decoded[..canonical.len() - 1];
            assert_eq!(decode_decimal_implied(&mut out, wire, scale), Err(Error::BufferOverflow));
        }
        let mut output = [0; 4];
        for input in [&b"1"[..], b"-1"] {
            assert_eq!(
                encoded_decimal_implied_len(input, usize::MAX, usize::MAX, true),
                Err(Error::Internal)
            );
            assert_eq!(
                encode_decimal_implied(&mut output.as_mut_slice(), input, usize::MAX, usize::MAX, true),
                Err(Error::Internal)
            );
            assert_eq!(
                decode_decimal_implied(&mut output.as_mut_slice(), input, usize::MAX),
                Err(Error::Internal)
            );
        }
        assert_eq!(
            encoded_decimal_implied_len(b"-1", usize::MAX - 1, usize::MAX, true),
            Err(Error::Internal)
        );
        assert_eq!(
            encode_decimal_implied(&mut output.as_mut_slice(), b"-1", usize::MAX - 1, usize::MAX, true),
            Err(Error::Internal)
        );
        assert_eq!(
            decode_decimal_implied(&mut output.as_mut_slice(), b"1", usize::MAX - 1),
            Err(Error::Internal)
        );
        assert_eq!(
            decode_decimal_implied(&mut output.as_mut_slice(), b"1", usize::MAX - 2),
            Err(Error::BufferOverflow)
        );
        assert_eq!(
            decode_decimal_implied(&mut output.as_mut_slice(), b"-0", usize::MAX).map(|s| &*s),
            Ok(&b"0"[..])
        );
    }

    #[test]
    fn test_implied_decimal_ascii_codecs() {
        assert_eq!(encode_implied_ascii(b"123.45", 2, 5, false), Ok(b"12345".to_vec()));
        assert_eq!(encode_implied_ascii(b"1", 2, 5, false), Ok(b"100".to_vec()));
        assert_eq!(encode_implied_ascii(b"001.20", 2, 5, false), Ok(b"120".to_vec()));
        assert_eq!(encode_implied_ascii(b"-0.05", 2, 5, true), Ok(b"-5".to_vec()));
        assert_eq!(encode_implied_ascii(b"-0", 2, 5, true), Ok(b"0".to_vec()));
        assert_eq!(encode_implied_ascii(b"1.234", 2, 5, false), Err(Error::Invalid));

        assert_eq!(decode_implied_ascii(b"12345", 2), Ok(b"123.45".to_vec()));
        assert_eq!(decode_implied_ascii(b"120", 2), Ok(b"1.2".to_vec()));
        assert_eq!(decode_implied_ascii(b"100", 2), Ok(b"1".to_vec()));
        assert_eq!(decode_implied_ascii(b"5", 2), Ok(b"0.05".to_vec()));
        assert_eq!(decode_implied_ascii(b"0", 2), Ok(b"0".to_vec()));
        assert_eq!(decode_implied_ascii(b"-5", 2), Ok(b"-0.05".to_vec()));
        assert_eq!(decode_implied_ascii(b"-", 2), Err(Error::Invalid));
        assert_eq!(decode_implied_ascii(b"+5", 2), Err(Error::Invalid));
        assert_eq!(decode_implied_ascii(b"12A", 2), Err(Error::Invalid));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::{prop_assert, prop_assert_eq, proptest};

    use super::{MAX_INTEGER_TEXT_LEN, decode_decimal_implied, encode_decimal_implied, format_i64, format_u64};

    fn canonical_scaled(value: u64, negative: bool, scale: usize) -> Vec<u8> {
        if value == 0 {
            return b"0".to_vec();
        }
        let digits = value.to_string();
        let digits = digits.as_bytes();
        let mut out = Vec::with_capacity(digits.len() + scale + 2);
        if negative {
            out.push(b'-');
        }
        if scale == 0 {
            out.extend_from_slice(digits);
            return out;
        }
        if digits.len() > scale {
            let int_len = digits.len() - scale;
            let mut frac_len = scale;
            while frac_len > 0 && digits[int_len + frac_len - 1] == b'0' {
                frac_len -= 1;
            }
            out.extend_from_slice(&digits[..int_len]);
            if frac_len > 0 {
                out.push(b'.');
                out.extend_from_slice(&digits[int_len..int_len + frac_len]);
            }
            return out;
        }
        let prefix_zeros = scale - digits.len();
        let mut frac_len = digits.len();
        while frac_len > 0 && digits[frac_len - 1] == b'0' {
            frac_len -= 1;
        }
        if frac_len == 0 {
            out.push(b'0');
            return out;
        }
        out.extend_from_slice(b"0.");
        out.resize(out.len() + prefix_zeros, b'0');
        out.extend_from_slice(&digits[..frac_len]);
        out
    }

    proptest! {
        #[test]
        fn fixed_decimal_matches_std(value: usize, width in 0usize..=32, capacity in 0usize..=34) {
            super::tests::check_fixed_decimal(value, width, capacity);
        }

        #[test]
        fn decimal_integer_parsers_roundtrip(unsigned: u64, signed: i64) {
            prop_assert_eq!(super::parse_u64(unsigned.to_string().as_bytes()), Ok(unsigned));
            prop_assert_eq!(super::parse_i64(signed.to_string().as_bytes()), Ok(signed));
        }

        #[test]
        fn signed_decimal_wire_roundtrips(value: i64, width in 20usize..=32) {
            let expected = value.to_string();
            let mut packed = [0xAA; 40];
            let mut zoned = [0xAA; 40];
            super::encode_decimal_packed_signed_fixed(&mut packed.as_mut_slice(), expected.as_bytes(), width).unwrap();
            super::encode_ebcdic_zoned_decimal(&mut zoned.as_mut_slice(), expected.as_bytes(), width).unwrap();
            for (is_packed, storage) in [(true, packed), (false, zoned)] {
                let mut input = storage.as_slice();
                let mut scratch = [0xAA; 80];
                let mut out = scratch.as_mut_slice();
                let decoded = if is_packed {
                    super::decode_decimal_packed_signed_fixed(&mut input, &mut out, width)
                } else {
                    super::decode_ebcdic_zoned_decimal(&mut input, &mut out, width)
                }.unwrap();
                prop_assert_eq!(&*decoded, expected.as_bytes());
                prop_assert_eq!(input, &storage[width..]);
                prop_assert_eq!(out.len(), 80 - if is_packed { 2 * width } else { width + 1 });
                prop_assert!(out.iter().all(|&byte| byte == 0xAA));
            }
        }

        #[test]
        fn format_u64_matches_std(value: u64) {
            let mut buf = [0u8; MAX_INTEGER_TEXT_LEN];
            let expected = value.to_string();
            prop_assert_eq!(format_u64(&mut buf, value), expected.as_bytes());
        }

        #[test]
        fn format_i64_matches_std(value: i64) {
            let mut buf = [0u8; MAX_INTEGER_TEXT_LEN];
            let expected = value.to_string();
            prop_assert_eq!(format_i64(&mut buf, value), expected.as_bytes());
        }

        #[test]
        fn implied_decimal_roundtrips(value: u64, negative in proptest::bool::ANY, scale in 0usize..=6usize) {
            let negative = negative && value != 0;
            let decoded = canonical_scaled(value, negative, scale);
            let mut encoded = [0u8; MAX_INTEGER_TEXT_LEN + 1];
            let mut encoded_ptr = encoded.as_mut_slice();
            let wire = encode_decimal_implied(&mut encoded_ptr, &decoded, scale, MAX_INTEGER_TEXT_LEN, true).map(|buf| buf.to_vec());
            let mut output = [0u8; MAX_INTEGER_TEXT_LEN + 2];
            let mut output_ptr = output.as_mut_slice();
            prop_assert_eq!(wire.as_ref().map(|buf| decode_decimal_implied(&mut output_ptr, buf, scale).map(|out| out.to_vec())), Ok(Ok(decoded)));
        }
    }
}
