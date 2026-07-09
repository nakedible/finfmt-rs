/// Fallible outcomes for wire-format encoding and decoding.
///
/// The variants are intentionally policy-oriented:
/// - `UnexpectedEof`: input ended before the requested wire data was available
/// - `BufferOverflow`: output or scratch space was too small for the result
/// - `InvalidValueLength`: the semantic value length was outside the accepted bounds
/// - `Invalid`: the provided data was malformed or otherwise not accepted
/// - `Internal`: an internal invariant failed, or a format definition/composition
///   is unsupported or inconsistent
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub enum Error {
    /// Input ended before enough wire bytes were available for the requested read.
    UnexpectedEof,
    /// Output or scratch space was too small for the encoded or decoded result.
    BufferOverflow,
    /// The semantic input or output length was outside the accepted bounds.
    InvalidValueLength,
    /// The provided data was malformed or otherwise invalid for the format.
    Invalid,
    /// An internal invariant failed, or the format definition/composition is invalid.
    Internal,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnexpectedEof => f.write_str("unexpected end of input"),
            Self::BufferOverflow => f.write_str("buffer too small for result"),
            Self::InvalidValueLength => f.write_str("semantic value length out of bounds"),
            Self::Invalid => f.write_str("invalid data"),
            Self::Internal => f.write_str("internal error or invalid format composition"),
        }
    }
}

impl std::error::Error for Error {}

/// Composite encode/decode error with field path context.
///
/// The underlying failure kind is kept in [`Error`], while the composite layer
/// records up to four nested field names from outermost to innermost.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub struct StructError {
    /// Underlying error kind.
    pub kind: Error,
    path_len: u8,
    path: [&'static str; 4],
    /// Whether deeper path entries were dropped because the fixed path buffer filled up.
    pub truncated: bool,
}

impl StructError {
    pub const MAX_DEPTH: usize = 4;

    #[inline(always)]
    pub const fn new(kind: Error) -> Self {
        Self {
            kind,
            path_len: 0,
            path: [""; 4],
            truncated: false,
        }
    }

    #[inline(always)]
    pub fn with_field(mut self, field: &'static str) -> Self {
        let len = self.path_len as usize;
        let keep = len.min(Self::MAX_DEPTH.saturating_sub(1));
        let mut idx = keep;
        while idx > 0 {
            self.path[idx] = self.path[idx - 1];
            idx -= 1;
        }
        self.path[0] = field;
        if len < Self::MAX_DEPTH {
            self.path_len += 1;
        } else {
            self.truncated = true;
        }
        self
    }

    #[inline(always)]
    pub fn path(&self) -> &[&'static str] {
        &self.path[..self.path_len as usize]
    }
}

impl From<Error> for StructError {
    #[inline(always)]
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

impl core::fmt::Display for StructError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.path_len == 0 {
            return self.kind.fmt(f);
        }

        let mut idx = 0usize;
        while idx < self.path_len as usize {
            if idx != 0 {
                f.write_str(".")?;
            }
            f.write_str(self.path[idx])?;
            idx += 1;
        }

        if self.truncated {
            f.write_str(".<truncated>")?;
        }

        f.write_str(": ")?;
        self.kind.fmt(f)
    }
}

impl std::error::Error for StructError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{Error, StructError};

    #[test]
    fn test_error_display_messages() {
        assert_eq!(Error::UnexpectedEof.to_string(), "unexpected end of input");
        assert_eq!(Error::BufferOverflow.to_string(), "buffer too small for result");
        assert_eq!(Error::InvalidValueLength.to_string(), "semantic value length out of bounds");
        assert_eq!(Error::Invalid.to_string(), "invalid data");
        assert_eq!(Error::Internal.to_string(), "internal error or invalid format composition");
    }

    #[test]
    fn test_struct_error_display_messages() {
        assert_eq!(StructError::from(Error::Invalid).to_string(), "invalid data");
        assert_eq!(
            StructError::from(Error::Invalid).with_field("field").to_string(),
            "field: invalid data"
        );
        assert_eq!(
            StructError::from(Error::InvalidValueLength)
                .with_field("inner")
                .with_field("outer")
                .to_string(),
            "outer.inner: semantic value length out of bounds"
        );

        let err = StructError::from(Error::Invalid);
        let err = err
            .with_field("fifth")
            .with_field("fourth")
            .with_field("third")
            .with_field("second")
            .with_field("first");
        assert_eq!(err.to_string(), "first.second.third.fourth.<truncated>: invalid data");
    }
}
