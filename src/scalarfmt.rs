use crate::Error;
use crate::primitive::decimal::{MAX_INTEGER_TEXT_LEN, format_i64, format_u64, parse_i64, parse_u64};
use crate::utils::{cold_path, length_as_invalid};

/// Core encode/decode trait for financial message field types.
///
/// Analogous to serde's `Serialize`/`Deserialize`, but for binary wire formats.
/// Each field type (e.g., `BcdN<4>`, `An<10>`) implements this trait to define
/// how values are encoded to and decoded from wire format.
///
/// # Buffer Protocol
///
/// Both encode and decode use the slice advancement pattern:
/// - Buffers are `&mut &[u8]` (input) or `&mut &mut [u8]` (output, decode scratch)
/// - Functions consume/produce bytes and advance the slice
/// - After the call, the slice reflects remaining capacity/data
///
/// The wire side is always `[u8]`. The user side can be `[u8]` or `str`:
/// - `encode`/`decode`: work with `&[u8]` user data
/// - `encode_str`/`decode_str`: convenience wrappers for `&str` user data
///
/// # Scratch Buffer
///
/// Encoding receives scratch as a workspace for this call only: the callee may
/// use all of it, and nothing it writes there outlives the call. Decoding
/// receives scratch as an arena cursor: decoded values may borrow from it, so
/// the callee advances it past every byte it keeps.
///
/// # Numeric Encoding
///
/// A typed number that does not fit the field is `Invalid`: it is out of range,
/// and there are no characters to remove. `InvalidValueLength` is reserved for
/// values whose length the caller can change.
///
/// The default implementations for numeric types convert through decimal strings
/// using the crate's fixed-buffer formatter. This matches the common
/// case where wire formats encode numbers as decimal digits (ASCII, EBCDIC, BCD).
///
/// Field types that use binary integer encoding (e.g., 2-byte big-endian) should
/// override the numeric methods to avoid the decimal round-trip.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a scalar field format",
    label = "this type does not implement `ScalarFmt`",
    note = "field formats are scalar formats or composite formats; both implement `FieldEncode`/`FieldDecode`"
)]
pub trait ScalarFmt {
    /// The format consumes all remaining input, as a [`crate::Rest`] length
    /// does.
    const TAKES_REST: bool = false;

    /// Calculate the encoded wire length in bytes for a given user input.
    ///
    /// Returns the exact number of bytes that `encode` would write for this input.
    /// Used to pre-calculate lengths for length-prefixed fields.
    ///
    /// Also validates the input, returning an error if encoding would fail.
    fn encoded_len(input: &[u8]) -> Result<usize, Error>;

    /// Encode user bytes to wire format.
    ///
    /// Writes the encoded bytes to `output` and advances it past the written portion.
    /// Uses `scratch` for intermediate transformations if needed.
    fn encode(output: &mut &mut [u8], scratch: &mut [u8], input: &[u8]) -> Result<(), Error>;

    /// Decode wire format to user bytes.
    ///
    /// Reads bytes from `input`, advances it past the consumed portion, and returns
    /// the decoded bytes. The returned slice borrows from either `input` or `scratch`
    /// depending on whether transformation was needed.
    fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a [u8], Error>;

    /// Calculate the encoded wire length for a string input.
    ///
    /// Default delegates to `encoded_len` via `as_bytes()`.
    #[inline(always)]
    fn encoded_len_str(input: &str) -> Result<usize, Error> {
        Self::encoded_len(input.as_bytes())
    }

    /// Encode a string value to wire format.
    ///
    /// Default delegates to `encode` via `as_bytes()`.
    #[inline(always)]
    fn encode_str(output: &mut &mut [u8], scratch: &mut [u8], input: &str) -> Result<(), Error> {
        Self::encode(output, scratch, input.as_bytes())
    }

    /// Decode wire format to a string value.
    ///
    /// Default delegates to `decode` and converts to `&str`.
    #[inline(always)]
    fn decode_str<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<&'a str, Error> {
        let bytes = Self::decode(input, scratch)?;
        core::str::from_utf8(bytes).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    /// Calculate the encoded length in bytes for an unsigned 64-bit integer.
    ///
    /// Default implementation converts to decimal string and delegates to `encoded_len`.
    ///
    /// Override for binary integer encodings.
    #[inline(always)]
    fn encoded_len_u64(input: u64) -> Result<usize, Error> {
        let mut digits = [0u8; MAX_INTEGER_TEXT_LEN];
        Self::encoded_len(format_u64(&mut digits, input)).map_err(length_as_invalid)
    }

    /// Calculate the encoded length in bytes for a `usize`.
    ///
    /// Default implementation delegates to `encoded_len_u64`.
    #[inline(always)]
    fn encoded_len_usize(input: usize) -> Result<usize, Error> {
        Self::encoded_len_u64(input as u64)
    }

    /// Calculate the encoded length in bytes for a signed 64-bit integer.
    ///
    /// Default implementation converts to decimal string and delegates to `encoded_len`.
    ///
    /// Override for binary integer encodings or special sign handling.
    #[inline(always)]
    fn encoded_len_i64(input: i64) -> Result<usize, Error> {
        let mut digits = [0u8; MAX_INTEGER_TEXT_LEN];
        Self::encoded_len(format_i64(&mut digits, input)).map_err(length_as_invalid)
    }

    /// Encode an unsigned 64-bit integer to wire format.
    ///
    /// Default implementation converts to decimal ASCII in a fixed stack buffer and
    /// delegates to `encode`.
    ///
    /// Override for binary integer encodings.
    #[inline(always)]
    fn encode_u64(output: &mut &mut [u8], scratch: &mut [u8], input: u64) -> Result<(), Error> {
        let mut digits = [0u8; MAX_INTEGER_TEXT_LEN];
        Self::encode(output, scratch, format_u64(&mut digits, input)).map_err(length_as_invalid)
    }

    /// Encode a `usize` to wire format.
    ///
    /// Default implementation delegates to `encode_u64`.
    #[inline(always)]
    fn encode_usize(output: &mut &mut [u8], scratch: &mut [u8], input: usize) -> Result<(), Error> {
        Self::encode_u64(output, scratch, input as u64)
    }

    /// Decode wire format to an unsigned 64-bit integer.
    ///
    /// Default implementation decodes via `decode` and parses ASCII digits.
    ///
    /// Override for binary integer encodings.
    #[inline(always)]
    fn decode_u64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<u64, Error> {
        parse_u64(Self::decode(input, scratch)?)
    }

    /// Decode wire format to a `usize`.
    ///
    /// Default implementation delegates to `decode_u64` and checks that the value fits.
    #[inline(always)]
    fn decode_usize<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<usize, Error> {
        usize::try_from(Self::decode_u64(input, scratch)?).map_err(|_| {
            cold_path();
            Error::Invalid
        })
    }

    /// Encode a signed 64-bit integer to wire format.
    ///
    /// Default implementation converts to decimal ASCII in a fixed stack buffer and
    /// delegates to `encode`.
    ///
    /// Override for binary integer encodings or special sign handling (e.g., C/D prefix).
    #[inline(always)]
    fn encode_i64(output: &mut &mut [u8], scratch: &mut [u8], input: i64) -> Result<(), Error> {
        let mut digits = [0u8; MAX_INTEGER_TEXT_LEN];
        Self::encode(output, scratch, format_i64(&mut digits, input)).map_err(length_as_invalid)
    }

    /// Decode wire format to a signed 64-bit integer.
    ///
    /// Default implementation decodes via `decode` and parses ASCII digits with optional `-` prefix.
    ///
    /// Override for binary integer encodings or special sign handling (e.g., C/D prefix).
    #[inline(always)]
    fn decode_i64<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<i64, Error> {
        parse_i64(Self::decode(input, scratch)?)
    }
}
