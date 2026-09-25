/// Fallible outcomes for wire-format encoding and decoding.
///
/// The variants are intentionally policy-oriented:
/// - `UnexpectedEof`: input ended before the requested wire data was available
/// - `BufferOverflow`: output or scratch space was too small for the result
/// - `InvalidValueLength`: when encoding, the supplied value is too long or too
///   short for the field
/// - `Invalid`: the provided data was malformed or otherwise not accepted
/// - `Internal`: an internal invariant failed, or a format definition/composition
///   is unsupported or inconsistent
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub enum Error {
    /// Input ended before enough wire bytes were available for the requested read.
    UnexpectedEof,
    /// Output or scratch space was too small for the encoded or decoded result.
    BufferOverflow,
    /// When encoding, the supplied value is too long or too short for the field:
    /// adding or removing characters or bytes would make it valid. This points at
    /// the configuration or value to fix, or at a format that should truncate.
    /// Decoding never returns it; a decoded value of the wrong length means the
    /// incoming message was encoded wrong, which is `Invalid`.
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
            Self::InvalidValueLength => f.write_str("value length not accepted by the field"),
            Self::Invalid => f.write_str("invalid data"),
            Self::Internal => f.write_str("internal error or invalid format composition"),
        }
    }
}

impl std::error::Error for Error {}

/// One step in a [`CompositeError`] path.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub enum PathSegment {
    /// A named field.
    Field(&'static str),
    /// A position in a list, counted from zero.
    Index(usize),
}

impl core::fmt::Display for PathSegment {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Field(name) => f.write_str(name),
            Self::Index(index) => write!(f, "[{index}]"),
        }
    }
}

/// Composite encode/decode error with path context.
///
/// The underlying failure kind is kept in [`Error`], while the composite layer
/// records up to four nested path segments from outermost to innermost. For
/// deeper paths, it retains the three outermost segments and the innermost one.
#[derive(Debug, PartialEq, Eq, Copy, Clone, Ord, PartialOrd, Hash)]
pub struct CompositeError {
    /// Underlying error kind.
    pub kind: Error,
    path_len: u8,
    path: [PathSegment; 4],
    truncated: bool,
}

impl CompositeError {
    pub const MAX_DEPTH: usize = 4;

    #[inline(always)]
    pub const fn new(kind: Error) -> Self {
        Self {
            kind,
            path_len: 0,
            path: [PathSegment::Field(""); 4],
            truncated: false,
        }
    }

    /// Prepend an enclosing field name.
    #[inline(always)]
    pub fn with_field(self, field: &'static str) -> Self {
        self.with_segment(PathSegment::Field(field))
    }

    /// Prepend an enclosing list position.
    #[inline(always)]
    pub fn with_index(self, index: usize) -> Self {
        self.with_segment(PathSegment::Index(index))
    }

    #[inline(always)]
    fn with_segment(mut self, segment: PathSegment) -> Self {
        let len = self.path_len as usize;
        let keep = if len < Self::MAX_DEPTH { len } else { Self::MAX_DEPTH - 2 };
        let mut idx = keep;
        while idx > 0 {
            self.path[idx] = self.path[idx - 1];
            idx -= 1;
        }
        self.path[0] = segment;
        if len < Self::MAX_DEPTH {
            self.path_len += 1;
        } else {
            self.truncated = true;
        }
        self
    }

    /// Whether intermediate path segments were dropped because the fixed path buffer filled up.
    #[inline(always)]
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Stored path segments; when truncated, omitted segments precede the last entry.
    #[inline(always)]
    pub fn path(&self) -> &[PathSegment] {
        self.path.get(..usize::from(self.path_len)).unwrap_or(&self.path)
    }
}

impl From<Error> for CompositeError {
    #[inline(always)]
    fn from(value: Error) -> Self {
        Self::new(value)
    }
}

/// Displays as `outer.list[2].field: <kind>`.
impl core::fmt::Display for CompositeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let path = self.path();
        for (idx, segment) in path.iter().enumerate() {
            if idx != 0 && self.truncated && idx + 1 == path.len() {
                f.write_str(".<truncated>")?;
            }
            if idx != 0 && matches!(segment, PathSegment::Field(_)) {
                f.write_str(".")?;
            }
            segment.fmt(f)?;
        }
        if !path.is_empty() {
            f.write_str(": ")?;
        }
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
    use super::{CompositeError, Error, PathSegment};

    #[test]
    fn test_error_display_messages() {
        assert_eq!(Error::UnexpectedEof.to_string(), "unexpected end of input");
        assert_eq!(Error::BufferOverflow.to_string(), "buffer too small for result");
        assert_eq!(Error::InvalidValueLength.to_string(), "value length not accepted by the field");
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
            assert_eq!(error.path(), path.iter().map(|&name| PathSegment::Field(name)).collect::<Vec<_>>());
            assert_eq!(error.is_truncated(), depth > CompositeError::MAX_DEPTH);
            assert_eq!(error.to_string(), display);
        }
        assert_eq!(
            CompositeError::from(Error::InvalidValueLength)
                .with_field("inner")
                .with_field("outer")
                .to_string(),
            "outer.inner: value length not accepted by the field"
        );
    }

    #[test]
    fn test_index_segments() {
        let error = CompositeError::from(Error::Invalid)
            .with_field("amount")
            .with_index(2)
            .with_field("items");
        assert_eq!(
            error.path(),
            [PathSegment::Field("items"), PathSegment::Index(2), PathSegment::Field("amount")]
        );
        assert_eq!(error.to_string(), "items[2].amount: invalid data");
        assert_eq!(CompositeError::from(Error::Invalid).with_index(0).to_string(), "[0]: invalid data");
        let deep = ["a", "b", "c"]
            .iter()
            .rev()
            .fold(CompositeError::from(Error::Invalid).with_index(7), |e, f| e.with_field(f));
        assert_eq!(deep.to_string(), "a.b.c[7]: invalid data");
        assert_eq!(deep.with_field("x").to_string(), "x.a.b.<truncated>[7]: invalid data");
        assert_eq!(size_of::<CompositeError>(), 72);
    }
}

#[cfg(test)]
mod proptests {
    use proptest::prelude::*;

    use super::{CompositeError, Error, PathSegment};

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
            prop_assert_eq!(error.path(), expected.iter().map(|&name| PathSegment::Field(name)).collect::<Vec<_>>());
            prop_assert_eq!(error.is_truncated(), fields.len() > CompositeError::MAX_DEPTH);
            prop_assert_eq!(error.kind, Error::Invalid);
            let mut display = expected;
            if error.is_truncated() {
                display.insert(3, "<truncated>");
            }
            let prefix = if display.is_empty() { String::new() } else { format!("{}: ", display.join(".")) };
            prop_assert_eq!(error.to_string(), format!("{prefix}invalid data"));
        }
    }
}
