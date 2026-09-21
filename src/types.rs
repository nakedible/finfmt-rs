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
/// records up to four nested field names from outermost to innermost. For deeper
/// paths, it retains the three outermost names and the innermost name.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub struct CompositeError {
    /// Underlying error kind.
    pub kind: Error,
    path_len: u8,
    path: [&'static str; 4],
    /// Whether intermediate path entries were dropped because the fixed path buffer filled up.
    pub truncated: bool,
}

impl CompositeError {
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
        let keep = if len < Self::MAX_DEPTH { len } else { Self::MAX_DEPTH - 2 };
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

    /// Stored field names; when truncated, omitted names precede the last entry.
    #[inline(always)]
    pub fn path(&self) -> &[&'static str] {
        &self.path[..self.path_len as usize]
    }
}

impl From<Error> for CompositeError {
    #[inline(always)]
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

impl core::fmt::Display for CompositeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        if self.path_len == 0 {
            return self.kind.fmt(f);
        }

        let mut idx = 0usize;
        while idx < self.path_len as usize {
            if idx != 0 {
                if self.truncated && idx + 1 == self.path_len as usize {
                    f.write_str(".<truncated>")?;
                }
                f.write_str(".")?;
            }
            f.write_str(self.path[idx])?;
            idx += 1;
        }

        f.write_str(": ")?;
        self.kind.fmt(f)
    }
}

impl std::error::Error for CompositeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.kind)
    }
}

#[cfg(test)]
mod tests {
    use super::{CompositeError, Error};

    #[test]
    fn test_error_display_messages() {
        assert_eq!(Error::UnexpectedEof.to_string(), "unexpected end of input");
        assert_eq!(Error::BufferOverflow.to_string(), "buffer too small for result");
        assert_eq!(Error::InvalidValueLength.to_string(), "semantic value length out of bounds");
        assert_eq!(Error::Invalid.to_string(), "invalid data");
        assert_eq!(Error::Internal.to_string(), "internal error or invalid format composition");
    }

    #[test]
    fn test_struct_error_paths() {
        let fields = ["a", "b", "c", "d", "e", "f"];
        let cases: &[(usize, &[&str], &str)] = &[
            (0, &[], "invalid data"),
            (1, &["a"], "a: invalid data"),
            (2, &["a", "b"], "a.b: invalid data"),
            (3, &["a", "b", "c"], "a.b.c: invalid data"),
            (4, &["a", "b", "c", "d"], "a.b.c.d: invalid data"),
            (5, &["a", "b", "c", "e"], "a.b.c.<truncated>.e: invalid data"),
            (6, &["a", "b", "c", "f"], "a.b.c.<truncated>.f: invalid data"),
        ];
        for &(depth, path, display) in cases {
            let error = fields[..depth]
                .iter()
                .rev()
                .fold(CompositeError::from(Error::Invalid), |error, field| error.with_field(field));
            assert_eq!(error.path(), path);
            assert_eq!(error.truncated, depth > CompositeError::MAX_DEPTH);
            assert_eq!(error.to_string(), display);
        }
        assert_eq!(
            CompositeError::from(Error::InvalidValueLength)
                .with_field("inner")
                .with_field("outer")
                .to_string(),
            "outer.inner: semantic value length out of bounds"
        );
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::{CompositeError, Error};

    proptest! {
        #[test]
        fn error_path_preserves_outer_context_and_leaf(
            fields in prop::collection::vec(prop::sample::select(vec!["outer", "inner", "leaf", "", "a.b"]), 0..65),
        ) {
            let error = fields.iter().rev().fold(CompositeError::new(Error::Invalid), |error, field| error.with_field(field));
            let mut expected = fields.clone();
            if fields.len() > CompositeError::MAX_DEPTH {
                expected = fields[..3].iter().copied().chain(fields.last().copied()).collect();
            }
            prop_assert_eq!(error.path(), expected.as_slice());
            prop_assert_eq!(error.truncated, fields.len() > CompositeError::MAX_DEPTH);
            prop_assert_eq!(error.kind, Error::Invalid);
            let mut display = expected;
            if error.truncated {
                display.insert(3, "<truncated>");
            }
            let prefix = if display.is_empty() { String::new() } else { format!("{}: ", display.join(".")) };
            prop_assert_eq!(error.to_string(), format!("{prefix}invalid data"));
        }
    }
}
