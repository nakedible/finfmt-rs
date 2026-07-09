#[cfg(all(not(debug_assertions), feature = "no-panic"))]
use no_panic::no_panic;

use crate::Error;
use crate::primitive::bytes::all_bytes_eq;
use crate::primitive::ebcdic::{ASCII_TO_EBCDIC_037, EBCDIC_037_TO_ASCII, translate_bytes, translate_bytes_inplace};
use crate::primitive::int::decode_signed_magnitude_i64;
use crate::primitive::nibble::{Bcdz, NibbleFormat, pack_nibbles, unpack_nibbles};
use crate::primitive::validation::{parse_scaled_decimal, parse_signed_decimal, split_signed_input, validate_numeric};
use crate::utils::cold_path;

pub const MAX_DECIMAL_LEN: usize = 20;

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
fn write_pair(buf: &mut [u8; MAX_DECIMAL_LEN], pos: &mut usize, value: usize) {
    let idx = value * 2;
    *pos -= 2;
    buf[*pos..*pos + 2].copy_from_slice(&DEC_DIGITS_LUT[idx..idx + 2]);
}

#[inline(always)]
fn write_quad(buf: &mut [u8; MAX_DECIMAL_LEN], pos: &mut usize, value: usize) {
    let hi = (value / 100) * 2;
    let lo = (value % 100) * 2;
    *pos -= 4;
    buf[*pos..*pos + 2].copy_from_slice(&DEC_DIGITS_LUT[hi..hi + 2]);
    buf[*pos + 2..*pos + 4].copy_from_slice(&DEC_DIGITS_LUT[lo..lo + 2]);
}

#[inline(always)]
fn pair4_u64(buf: &mut [u8; MAX_DECIMAL_LEN], mut value: u64) -> usize {
    let mut pos = MAX_DECIMAL_LEN;

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
pub fn format_u64(output: &mut [u8; MAX_DECIMAL_LEN], value: u64) -> &[u8] {
    let start = pair4_u64(output, value);
    &output[start..]
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn format_i64(output: &mut [u8; MAX_DECIMAL_LEN], value: i64) -> &[u8] {
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

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_negative_prefix(input: &mut &[u8], neg: u8) -> Result<bool, Error> {
    if !matches!(input.first(), Some(&sign) if sign == neg) {
        return Ok(false);
    }
    let _ = input.split_off(..1).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    Ok(true)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn prepend_minus<'a>(output: &mut &'a mut [u8], digits: &[u8]) -> Result<&'a mut [u8], Error> {
    let buf = output.split_off_mut(..digits.len() + 1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    buf[0] = b'-';
    buf[1..].copy_from_slice(digits);
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
    let (negative, _int_digits, frac_digits, total_digits, first_nonzero) = parse_scaled_decimal(input, scale, signed)?;
    let digits_len = match first_nonzero {
        Some(first_nonzero) => total_digits - first_nonzero,
        None => 1,
    };
    if digits_len > max_digits {
        cold_path();
        return Err(Error::InvalidValueLength);
    }

    let out_len = digits_len + usize::from(negative && first_nonzero.is_some());
    let buf = output.split_off_mut(..out_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    if first_nonzero.is_none() {
        buf[0] = b'0';
        return Ok(&mut buf[..1]);
    }

    let mut out = 0usize;
    if negative {
        buf[0] = b'-';
        out = 1;
    }
    let skip = first_nonzero.unwrap_or(0);
    let mut digit_index = 0usize;
    for &byte in input {
        if byte != b'.' && byte != b'-' {
            if digit_index >= skip {
                buf[out] = byte;
                out += 1;
            }
            digit_index += 1;
        }
    }
    let mut i = 0usize;
    while i < scale - frac_digits {
        if digit_index >= skip {
            buf[out] = b'0';
            out += 1;
        }
        digit_index += 1;
        i += 1;
    }
    Ok(buf)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_implied<'a>(output: &mut &'a mut [u8], input: &[u8], scale: usize) -> Result<&'a mut [u8], Error> {
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
    validate_numeric(digits, 1, usize::MAX)?;

    let mut first_nonzero = digits.len();
    let mut i = 0usize;
    while i < digits.len() {
        if digits[i] != b'0' {
            first_nonzero = i;
            break;
        }
        i += 1;
    }
    if first_nonzero == digits.len() {
        let buf = output.split_off_mut(..1).ok_or_else(|| {
            cold_path();
            Error::BufferOverflow
        })?;
        buf[0] = b'0';
        return Ok(buf);
    }

    let digits = &digits[first_nonzero..];
    let out_len = if scale == 0 {
        digits.len() + usize::from(negative)
    } else if digits.len() > scale {
        let int_len = digits.len() - scale;
        let mut frac_len = scale;
        while frac_len > 0 && digits[int_len + frac_len - 1] == b'0' {
            frac_len -= 1;
        }
        usize::from(negative) + if frac_len == 0 { int_len } else { int_len + 1 + frac_len }
    } else {
        let prefix_zeros = scale - digits.len();
        let mut frac_len = digits.len();
        while frac_len > 0 && digits[frac_len - 1] == b'0' {
            frac_len -= 1;
        }
        usize::from(negative) + if frac_len == 0 { 1 } else { 2 + prefix_zeros + frac_len }
    };
    let buf = output.split_off_mut(..out_len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;

    let mut out = 0usize;
    if negative {
        buf[out] = b'-';
        out += 1;
    }

    if scale == 0 {
        buf[out..out + digits.len()].copy_from_slice(digits);
        out += digits.len();
        return Ok(&mut buf[..out]);
    }

    if digits.len() > scale {
        let int_len = digits.len() - scale;
        buf[out..out + int_len].copy_from_slice(&digits[..int_len]);
        out += int_len;
        let frac = &digits[int_len..];
        let mut frac_len = frac.len();
        while frac_len > 0 && frac[frac_len - 1] == b'0' {
            frac_len -= 1;
        }
        if frac_len == 0 {
            return Ok(&mut buf[..out]);
        }
        buf[out] = b'.';
        out += 1;
        buf[out..out + frac_len].copy_from_slice(&frac[..frac_len]);
        out += frac_len;
        return Ok(&mut buf[..out]);
    }

    let prefix_zeros = scale - digits.len();
    let mut frac_len = digits.len();
    while frac_len > 0 && digits[frac_len - 1] == b'0' {
        frac_len -= 1;
    }
    if frac_len == 0 {
        buf[out] = b'0';
        out += 1;
        return Ok(&mut buf[..out]);
    }
    buf[out] = b'0';
    buf[out + 1] = b'.';
    out += 2;
    let mut i = 0usize;
    while i < prefix_zeros {
        buf[out] = b'0';
        out += 1;
        i += 1;
    }
    buf[out..out + frac_len].copy_from_slice(&digits[..frac_len]);
    out += frac_len;
    Ok(&mut buf[..out])
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_overpunch_digit(negative: bool, digit: u8) -> u8 {
    (if negative { 0xD0 } else { 0xC0 }) + digit
}

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
    let (negative, digit) = match input >> 4 {
        0xA | 0xC | 0xE | 0xF => (false, input & 0x0F),
        0xB | 0xD => (true, input & 0x0F),
        _ => {
            cold_path();
            return Err(Error::Invalid);
        }
    };
    if digit > 9 {
        cold_path();
        return Err(Error::Invalid);
    }
    Ok((negative, digit))
}

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

#[inline(always)]
fn encode_decimal_ebcdic_signed_digits(output: &mut [u8], digits: &[u8], negative: bool) {
    let pad = output.len() - digits.len();
    output[..pad].fill(0xF0);
    if digits.len() > 1 {
        let end = output.len() - 1;
        output[pad..end].copy_from_slice(&digits[..digits.len() - 1]);
        translate_bytes_inplace(&mut output[pad..end], &ASCII_TO_EBCDIC_037);
    }
    output[output.len() - 1] = encode_overpunch_digit(negative, digits[digits.len() - 1] - b'0');
}

#[inline(always)]
fn encode_decimal_packed_digits(output: &mut [u8], digits: &[u8], negative: bool, signed: bool) -> Result<(), Error> {
    let max_digits = packed_decimal_max_digits(output.len())?;
    let pad = max_digits - digits.len();
    let prefix_bytes = pad / 2;
    let sign = encode_packed_sign(negative, signed);
    output[..prefix_bytes].fill(0);
    if digits.len().is_multiple_of(2) {
        let (first, rest) = digits.split_first().ok_or_else(|| {
            cold_path();
            Error::Invalid
        })?;
        let tail = &mut output[prefix_bytes..];
        tail[0] = first.wrapping_sub(b'0');
        let mut rest_out = &mut tail[1..];
        let _ = pack_nibbles(&mut rest_out, rest, false, sign, &Bcdz::TABLE)?;
    } else {
        let mut tail = &mut output[prefix_bytes..];
        let _ = pack_nibbles(&mut tail, digits, false, sign, &Bcdz::TABLE)?;
    }
    Ok(())
}

// TODO: write an optimized `parse_u64` that consumes ASCII digits 4 at a time
// (pair4-style, mirroring `format_u64` / `pair4_u64`). Today we delegate to
// `str::from_utf8` + `u64::parse` from std, which is general-purpose and does
// redundant per-byte checks on input we've already validated as digits.
#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn parse_u64(bytes: &[u8]) -> Result<u64, Error> {
    let digits = match bytes.iter().position(|&byte| byte != b'0') {
        Some(first_nonzero) => &bytes[first_nonzero..],
        None => b"0",
    };
    if digits.len() > MAX_DECIMAL_LEN {
        cold_path();
        return Err(Error::Invalid);
    }
    let value = core::str::from_utf8(digits).map_err(|_| {
        cold_path();
        Error::Invalid
    })?;
    value.parse::<u64>().map_err(|_| {
        cold_path();
        Error::Invalid
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
    let mut digits_buf = [0u8; MAX_DECIMAL_LEN];
    let digits = format_u64(&mut digits_buf, value as u64);
    if digits.len() > len {
        cold_path();
        return Err(Error::Invalid);
    }
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let pad = len - digits.len();
    buf[..pad].fill(b'0');
    buf[pad..].copy_from_slice(digits);
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_ebcdic_fixed(output: &mut &mut [u8], value: usize, len: usize) -> Result<(), Error> {
    let mut digits_buf = [0u8; MAX_DECIMAL_LEN];
    let digits = format_u64(&mut digits_buf, value as u64);
    if digits.len() > len {
        cold_path();
        return Err(Error::Invalid);
    }
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let pad = len - digits.len();
    buf[..pad].fill(0xF0);
    buf[pad..].copy_from_slice(digits);
    translate_bytes_inplace(&mut buf[pad..], &ASCII_TO_EBCDIC_037);
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_ebcdic_blankable_fixed(output: &mut &mut [u8], value: usize, len: usize) -> Result<(), Error> {
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
    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    validate_numeric(bytes, len, len)?;
    parse_usize(bytes)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ebcdic_fixed(input: &mut &[u8], len: usize) -> Result<usize, Error> {
    let bytes = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    let digits = match bytes.iter().position(|&byte| byte != 0xF0) {
        Some(first_nonzero) => &bytes[first_nonzero..],
        None => return Ok(0),
    };
    if digits.len() > MAX_DECIMAL_LEN {
        cold_path();
        return Err(Error::Invalid);
    }
    let mut ascii = [0u8; MAX_DECIMAL_LEN];
    translate_bytes(&mut ascii[..digits.len()], digits, &EBCDIC_037_TO_ASCII);
    validate_numeric(&ascii[..digits.len()], digits.len(), digits.len())?;
    parse_usize(&ascii[..digits.len()])
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ebcdic_blankable_fixed(input: &mut &[u8], len: usize) -> Result<usize, Error> {
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
pub fn encode_decimal_ebcdic_signed_fixed(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    let (negative, digits) = parse_signed_decimal(input, len)?;
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    encode_decimal_ebcdic_signed_digits(buf, digits, negative);
    Ok(())
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_ebcdic_signed_fixed<'a>(input: &mut &[u8], output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    let input = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    if len == 0 {
        cold_path();
        return Err(Error::Invalid);
    }

    let buf = output.split_off_mut(..len + 1).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;

    let body_len = len - 1;
    let (negative, last_digit) = decode_overpunch_digit(input[len - 1])?;
    let out = usize::from(negative);
    if body_len != 0 {
        translate_bytes(&mut buf[out..out + body_len], &input[..body_len], &EBCDIC_037_TO_ASCII);
        validate_numeric(&buf[out..out + body_len], body_len, body_len)?;
    }
    let first_nonzero = buf[out..out + body_len].iter().position(|&byte| byte != b'0');
    if first_nonzero.is_none() && last_digit == 0 {
        buf[0] = b'0';
        return Ok(&mut buf[..1]);
    }
    let copied = if let Some(first_nonzero) = first_nonzero {
        buf.copy_within(out + first_nonzero..out + body_len, out);
        body_len - first_nonzero
    } else {
        0
    };
    if negative {
        buf[0] = b'-';
    }
    let last_pos = out + copied;
    buf[last_pos] = b'0' + last_digit;
    let total_len = last_pos + 1;
    Ok(&mut buf[..total_len])
}

#[inline(always)]
fn unpack_decimal_packed<'a>(input: &[u8], output: &mut &'a mut [u8]) -> Result<&'a mut [u8], Error> {
    let buf = output.split_off_mut(..input.len() * 2).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    let mut out = &mut *buf;
    unpack_nibbles(&mut out, input, &Bcdz::DIGITS)
}

#[inline(always)]
fn decode_decimal_packed_common<'a>(input: &[u8], output: &mut &'a mut [u8], signed: bool) -> Result<&'a mut [u8], Error> {
    if input.is_empty() {
        cold_path();
        return Err(Error::Invalid);
    }
    let buf = unpack_decimal_packed(input, output)?;
    let negative = decode_packed_sign(input[input.len() - 1] & 0x0F)?;
    if negative && !signed {
        cold_path();
        return Err(Error::Invalid);
    }
    let digits_len = buf.len() - 1;
    validate_numeric(&buf[..digits_len], digits_len, digits_len)?;
    let first_nonzero = buf[..digits_len].iter().position(|&byte| byte != b'0');
    if first_nonzero.is_none() {
        buf[0] = b'0';
        return Ok(&mut buf[..1]);
    }
    let offset = usize::from(negative);
    let first_nonzero = first_nonzero.unwrap_or(0);
    let end = digits_len;
    buf.copy_within(first_nonzero..end, offset);
    if negative {
        buf[0] = b'-';
    }
    Ok(&mut buf[..offset + end - first_nonzero])
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_packed_fixed(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    validate_numeric(input, 1, packed_decimal_max_digits(len)?)?;
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    encode_decimal_packed_digits(buf, input, false, false)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn encode_decimal_packed_signed_fixed(output: &mut &mut [u8], input: &[u8], len: usize) -> Result<(), Error> {
    let (negative, digits) = parse_signed_decimal(input, packed_decimal_max_digits(len)?)?;
    let buf = output.split_off_mut(..len).ok_or_else(|| {
        cold_path();
        Error::BufferOverflow
    })?;
    encode_decimal_packed_digits(buf, digits, negative, true)
}

#[inline(always)]
#[cfg_attr(all(not(debug_assertions), feature = "no-panic"), no_panic)]
pub fn decode_decimal_packed_fixed<'a>(input: &mut &[u8], output: &mut &'a mut [u8], len: usize) -> Result<&'a mut [u8], Error> {
    let input = input.split_off(..len).ok_or_else(|| {
        cold_path();
        Error::UnexpectedEof
    })?;
    decode_decimal_packed_common(input, output, false)
}

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
        MAX_DECIMAL_LEN, decode_decimal_ascii_fixed, decode_decimal_ebcdic_blankable_fixed, decode_decimal_ebcdic_fixed,
        decode_decimal_ebcdic_signed_fixed, decode_decimal_implied, decode_decimal_packed_fixed, decode_decimal_packed_signed_fixed,
        decode_negative_prefix, decode_sign, encode_decimal_ascii_fixed, encode_decimal_ebcdic_blankable_fixed,
        encode_decimal_ebcdic_fixed, encode_decimal_ebcdic_signed_fixed, encode_decimal_implied, encode_decimal_packed_fixed,
        encode_decimal_packed_signed_fixed, encode_negative_prefix, encode_sign, format_i64, format_u64, prepend_minus,
    };
    use crate::Error;
    use crate::primitive::validation::validate_decimal_implied;

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
        decode_decimal_ebcdic_blankable_fixed(&mut input, N)
    }

    fn encode_signed_ebcdic_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_decimal_ebcdic_signed_fixed(out, input, N))
    }

    fn decode_signed_ebcdic_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_DECIMAL_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_decimal_ebcdic_signed_fixed(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_packed_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_decimal_packed_fixed(out, input, N))
    }

    fn decode_packed_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_DECIMAL_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_decimal_packed_fixed(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_signed_packed_ascii<const N: usize>(input: &[u8]) -> Result<[u8; N], Error> {
        encode::<N>(|out| encode_decimal_packed_signed_fixed(out, input, N))
    }

    fn decode_signed_packed_ascii<const N: usize>(input: &[u8]) -> Result<Vec<u8>, Error> {
        let mut input = input;
        let mut scratch = [0u8; MAX_DECIMAL_LEN];
        let mut scratch_ptr = scratch.as_mut_slice();
        Ok(decode_decimal_packed_signed_fixed(&mut input, &mut scratch_ptr, N)?.to_vec())
    }

    fn encode_implied_ascii(input: &[u8], scale: usize, max_digits: usize, signed: bool) -> Result<Vec<u8>, Error> {
        let len = validate_decimal_implied(input, scale, max_digits, signed)?;
        let mut output = [0u8; MAX_DECIMAL_LEN + 1];
        let mut out_ptr = output.as_mut_slice();
        let encoded = encode_decimal_implied(&mut out_ptr, input, scale, max_digits, signed)?;
        assert_eq!(encoded.len(), len);
        Ok(encoded.to_vec())
    }

    fn decode_implied_ascii(input: &[u8], scale: usize) -> Result<Vec<u8>, Error> {
        let mut output = [0u8; MAX_DECIMAL_LEN + 2];
        let mut out_ptr = output.as_mut_slice();
        Ok(decode_decimal_implied(&mut out_ptr, input, scale)?.to_vec())
    }

    #[test]
    fn test_format_decimal() {
        let mut buf = [0u8; MAX_DECIMAL_LEN];
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
            encode::<2>(|out| encode_decimal_ebcdic_blankable_fixed(out, 0, 2)),
            Ok([0x40, 0x40])
        );
        assert_eq!(
            encode::<2>(|out| encode_decimal_ebcdic_blankable_fixed(out, 42, 2)),
            Ok([0xF4, 0xF2])
        );
        assert_eq!(decode_blankable_ebcdic::<2>(&[0x40, 0x40]), Ok(0));
        assert_eq!(decode_blankable_ebcdic::<2>(&[0xF4, 0xF2]), Ok(42));
        assert_eq!(decode_blankable_ebcdic::<2>(&[0x40, 0xF2]), Err(Error::Invalid));
        assert_eq!(
            encode::<1>(|out| encode_decimal_ebcdic_blankable_fixed(out, 0, 2)),
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
        assert_eq!(decode_negative_prefix(&mut input, b'-'), Ok(true));
        assert_eq!(input, b"123");
        let mut input = &b"123"[..];
        assert_eq!(decode_negative_prefix(&mut input, b'-'), Ok(false));
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
            decode_decimal_ebcdic_signed_fixed(&mut short_input, &mut short_ptr, 2),
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
        assert_eq!(decode_implied_ascii(b"+5", 2), Err(Error::Invalid));
        assert_eq!(decode_implied_ascii(b"12A", 2), Err(Error::Invalid));
    }
}

#[cfg(test)]
mod proptests {
    use proptest::{prop_assert_eq, proptest};

    use super::{MAX_DECIMAL_LEN, decode_decimal_implied, encode_decimal_implied, format_i64, format_u64};

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
        fn format_u64_matches_std(value: u64) {
            let mut buf = [0u8; MAX_DECIMAL_LEN];
            let expected = value.to_string();
            prop_assert_eq!(format_u64(&mut buf, value), expected.as_bytes());
        }

        #[test]
        fn format_i64_matches_std(value: i64) {
            let mut buf = [0u8; MAX_DECIMAL_LEN];
            let expected = value.to_string();
            prop_assert_eq!(format_i64(&mut buf, value), expected.as_bytes());
        }

        #[test]
        fn implied_decimal_roundtrips(value: u64, negative in proptest::bool::ANY, scale in 0usize..=6usize) {
            let negative = negative && value != 0;
            let decoded = canonical_scaled(value, negative, scale);
            let mut encoded = [0u8; MAX_DECIMAL_LEN + 1];
            let mut encoded_ptr = encoded.as_mut_slice();
            let wire = encode_decimal_implied(&mut encoded_ptr, &decoded, scale, MAX_DECIMAL_LEN, true).map(|buf| buf.to_vec());
            let mut output = [0u8; MAX_DECIMAL_LEN + 2];
            let mut output_ptr = output.as_mut_slice();
            prop_assert_eq!(wire.as_ref().map(|buf| decode_decimal_implied(&mut output_ptr, buf, scale).map(|out| out.to_vec())), Ok(Ok(decoded)));
        }
    }
}
