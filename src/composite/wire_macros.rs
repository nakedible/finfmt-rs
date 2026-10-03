/// Make structs their own wire formats, described by `#[wire(...)]`
/// attributes.
///
/// Each struct in the block implements [`FieldEncode<Self>`](crate::FieldEncode)
/// and [`FieldDecode<'de, Self>`](crate::FieldDecode), so it encodes and decodes
/// on its own or as another record's field. The struct is emitted as written,
/// without its `#[wire]` attributes, so derives and serde attributes keep
/// working. The attributes are what a derive would read, and nothing else in
/// the block is wire syntax.
///
/// Every struct states its layout:
///
/// - `#[wire(concat)]`: fields back to back, in declaration order. An
///   `Option` field may be omitted at the end of the record: it decodes as
///   `None` at the end of the input, and only `Option` fields may follow it.
///
/// Fields take at most one `#[wire(...)]` attribute:
///
/// - `fmt = F`: the field's format. Without it, the field's type must be a
///   format itself, such as another wire struct; for an `Option<T>` field,
///   `T` must.
///
/// A struct may have one lifetime parameter, for fields borrowed from the
/// input or scratch.
///
/// Fields are parsed one per macro step, so a record of more than about 100
/// fields needs a higher `#![recursion_limit]`. Doc comments are free, and so
/// are attributes after `#[wire(...)]`; any other attribute before it costs a
/// step.
///
/// ```
/// use finfmt::{Ascii, AsciiLength, Field, Fixed, Numeric};
///
/// type N4 = Field<Numeric<4, 4>, Fixed<4>>;
/// type N6 = Field<Numeric<6, 6>, Fixed<6>>;
///
/// finfmt::wire_type! {
///     #[derive(Debug, PartialEq)]
///     #[wire(concat)]
///     pub struct Header<'a> {
///         #[wire(fmt = N4)]
///         pub mti: &'a str,
///         pub terminal: Terminal,
///         #[wire(fmt = N6)]
///         pub stan: Option<u64>,
///     }
///
///     #[derive(Debug, PartialEq)]
///     #[wire(concat)]
///     pub struct Terminal {
///         #[wire(fmt = Field<Ascii<1, 8>, AsciiLength<1>>)]
///         pub id: String,
///     }
/// }
///
/// let header = Header { mti: "0100", terminal: Terminal { id: "T1".into() }, stan: None };
/// let mut output = [0; 32];
/// let used = finfmt::encode::<Header, _>(&mut output, &mut [0; 64], &header).unwrap();
/// assert_eq!(&output[..used], b"01002T1");
/// let mut scratch = [0; 64];
/// let decoded: Header = finfmt::decode::<Header, _>(&output[..used], &mut scratch).unwrap();
/// assert_eq!(decoded, header);
/// ```
///
/// A struct without a layout, a required field after an `Option` one in a
/// concat record, and an unknown argument are compile errors:
///
/// ```compile_fail
/// finfmt::wire_type! {
///     struct NoLayout {}
/// }
/// ```
///
/// ```compile_fail
/// type N2 = finfmt::Field<finfmt::Numeric<2, 2>, finfmt::Fixed<2>>;
/// finfmt::wire_type! {
///     #[wire(concat)]
///     struct Gap {
///         #[wire(fmt = N2)]
///         first: Option<String>,
///         #[wire(fmt = N2)]
///         second: String,
///     }
/// }
/// ```
///
/// ```compile_fail
/// type N2 = finfmt::Field<finfmt::Numeric<2, 2>, finfmt::Fixed<2>>;
/// finfmt::wire_type! {
///     #[wire(concat)]
///     struct Unknown {
///         #[wire(fmt = N2, bits = 3)]
///         code: String,
///     }
/// }
/// ```
#[macro_export]
macro_rules! wire_type {
    ($($(#[$($attr:tt)*])* $vis:vis $kw:ident $name:ident $(<$lt:lifetime>)? { $($body:tt)* })*) => {
        $($crate::__finfmt_wire_item! { [] []; $(#[$($attr)*])*; $vis $kw $name [$($lt)?] { $($body)* } })*
    };
}

/// Split an item's attributes into kept ones and `#[wire]` arguments, then
/// dispatch on the layout.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_item {
    ([$($kept:tt)*] [$($wire:tt)*]; #[wire($($args:tt)*)] $(#[$($rest:tt)*])*; $($item:tt)*) => {
        $crate::__finfmt_wire_item! { [$($kept)*] [$($wire)* $($args)* ,]; $(#[$($rest)*])*; $($item)* }
    };
    ([$($kept:tt)*] $wire:tt; #[$($attr:tt)*] $(#[$($rest:tt)*])*; $($item:tt)*) => {
        $crate::__finfmt_wire_item! { [$($kept)* #[$($attr)*]] $wire; $(#[$($rest)*])*; $($item)* }
    };
    ($kept:tt [concat ,]; ; $vis:vis struct $name:ident $lt:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { (concat $kept $vis $name $lt []) []; $($body)* }
    };
    ($kept:tt []; ; $vis:vis $kw:ident $name:ident $($rest:tt)*) => {
        compile_error!(concat!("wire_type!: `", stringify!($name), "` needs a layout attribute, such as #[wire(concat)]"));
    };
    ($kept:tt $wire:tt; ; $vis:vis enum $name:ident $($rest:tt)*) => {
        compile_error!(concat!("wire_type!: enums are not supported yet (`", stringify!($name), "`)"));
    };
    ($kept:tt [$($wire:tt)*]; ; $vis:vis $kw:ident $name:ident $($rest:tt)*) => {
        compile_error!(concat!("wire_type!: unsupported layout for `", stringify!($name), "`: ", stringify!($($wire)*)));
    };
}

/// Parse one field per step into `{ [kept attributes] (vis) name (type) kind
/// (default format) [wire arguments] }`. Doc comments are taken in the same
/// step; other attributes before the `#[wire]` one move into the field's kept
/// list one per step.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_fields {
    (($layout:ident $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [];) => {
        $crate::__finfmt_wire_emit! { $layout $kept $vis $name $lt [$($done)*] }
    };
    (($layout:ident $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field (Option<$inner>) opt ($inner) [$($args)*]
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:ident $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : $ty:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field ($ty) req ($ty) [$($args)*]
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:ident $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* $fvis:vis $field:ident : Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])*] ($fvis) $field (Option<$inner>) opt ($inner) []
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:ident $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* $fvis:vis $field:ident : $ty:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* { [$($fkept)* $(#[doc = $doc])*] ($fvis) $field ($ty) req ($ty) [] }]) [];
            $($($rest)*)?
        }
    };
    ($state:tt [$($fkept:tt)*]; #[$($attr:tt)*] $($rest:tt)*) => {
        $crate::__finfmt_wire_fields! { $state [$($fkept)* #[$($attr)*]]; $($rest)* }
    };
}

/// Emit the struct without its `#[wire]` attributes, and its format impls.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_emit {
    (concat [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident ($ty:ty) $kind:ident ($default:ty) $args:tt })*]) => {
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = assert!(
            $crate::composite::optional_fields_trail(&[$($crate::__finfmt_wire_is_optional!($kind)),*]),
            concat!("wire_type!: in the concat record `", stringify!($name), "`, only Option fields may follow an Option field")
        );

        impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
            #[inline(always)]
            #[allow(unused_assignments)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                let _ = (&output, &scratch, value);
                #[allow(unused_mut, unused_variables)]
                let mut omitted = false;
                $($crate::__finfmt_wire_concat_encode!(value, output, scratch, omitted; $kind $field $crate::__finfmt_wire_fmt!($args $default));)*
                Ok(())
            }
        }

        impl<$($lt,)? '__finfmt_de> $crate::composite::FieldDecode<'__finfmt_de, $crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?])>
            for $name $(<$lt>)?
        {
            #[inline(always)]
            fn decode_field(
                input: &mut &'__finfmt_de [u8],
                scratch: &mut &'__finfmt_de mut [u8],
            ) -> Result<$crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?]), $crate::CompositeError> {
                let _ = (&input, &scratch);
                $($crate::__finfmt_wire_concat_decode!(input, scratch; $kind $field $crate::__finfmt_wire_fmt!($args $default));)*
                Ok($name { $($field),* })
            }
        }
    };
}

/// The struct's type with its lifetime, if any, set to `$de`.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_decoded {
    ($name:ident [$de:lifetime] [$lt:lifetime]) => { $name<$de> };
    ($name:ident [$de:lifetime] []) => { $name };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_is_optional {
    (opt) => {
        true
    };
    (req) => {
        false
    };
}

/// A field's format: `fmt = F`, or the default.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_fmt {
    ([fmt = $fmt:ty $(,)?] $default:ty) => { $fmt };
    ([] $default:ty) => { $default };
    ([$($args:tt)*] $default:ty) => {
        compile_error!(concat!("wire_type!: unsupported #[wire] field arguments: ", stringify!($($args)*)))
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_concat_encode {
    ($value:ident, $output:ident, $scratch:ident, $omitted:ident; req $field:ident $fmt:ty) => {
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
    };
    ($value:ident, $output:ident, $scratch:ident, $omitted:ident; opt $field:ident $fmt:ty) => {
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
            }
            (false, Some(inner)) => <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            (false, None) => $omitted = true,
            (true, None) => {}
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_concat_decode {
    ($input:ident, $scratch:ident; req $field:ident $fmt:ty) => {
        let $field = <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    ($input:ident, $scratch:ident; opt $field:ident $fmt:ty) => {
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
    };
}

#[cfg(test)]
mod tests {
    use serde::Serialize;

    use crate::composite::FieldEncode;
    use crate::{Ascii, AsciiLength, CompositeError, Error, Field, Fixed, Numeric, PathSegment};

    type N2 = Field<Numeric<2, 2>, Fixed<2>>;
    type N4 = Field<Numeric<4, 4>, Fixed<4>>;
    type Var = Field<Ascii<0, 9>, AsciiLength<1>>;

    fn encode<F: FieldEncode<T>, T>(value: &T) -> Result<std::vec::Vec<u8>, CompositeError> {
        let mut output = [0; 64];
        let used = crate::encode::<F, T>(&mut output, &mut [0; 128], value)?;
        Ok(output[..used].to_vec())
    }

    crate::wire_type! {
        /// A record with a borrowed field, a nested record and optional tail.
        #[derive(Debug, Clone, PartialEq, Serialize)]
        #[wire(concat)]
        struct Record<'a> {
            /// The kind.
            #[wire(fmt = N4)]
            #[serde(rename = "type")]
            kind: &'a str,
            inner: Inner,
            #[wire(fmt = Var)]
            name: String,
            #[wire(fmt = N2)]
            first: Option<u64>,
            second: Option<Inner>,
        }

        #[derive(Debug, Clone, PartialEq, Serialize)]
        #[wire(concat)]
        pub(crate) struct Inner {
            #[wire(fmt = N2)]
            pub(crate) code: String,
        }
    }

    // The same layout through the old macro, for comparison.
    #[derive(Debug, Clone, PartialEq)]
    struct OldRecord<'a> {
        kind: &'a str,
        inner: Inner,
        name: String,
        first: Option<u64>,
        second: Option<Inner>,
    }

    crate::concat_format! {
        struct OldRecordFmt for<'a> OldRecord<'a> {
            kind: N4,
            inner: Inner,
            name: Var,
            first: Option<N2>,
            second: Option<Inner>,
        }
    }

    // Each macro's record nests inside the other's.
    #[derive(Debug, PartialEq)]
    struct Outer {
        record: Inner,
    }

    crate::concat_format! {
        struct OuterFmt for Outer {
            record: Inner,
        }
    }

    crate::wire_type! {
        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct Wrapped {
            #[wire(fmt = OuterFmt)]
            outer: Outer,
        }
    }

    fn record(first: Option<u64>, second: Option<&str>) -> Record<'static> {
        Record {
            kind: "0100",
            inner: Inner { code: "07".into() },
            name: "ABC".into(),
            first,
            second: second.map(|code| Inner { code: code.into() }),
        }
    }

    #[test]
    fn concat_records_match_concat_format() {
        for (first, second, wire) in [
            (None, None, &b"0100073ABC"[..]),
            (Some(15), None, b"0100073ABC15"),
            (Some(15), Some("42"), b"0100073ABC1542"),
        ] {
            let value = record(first, second);
            let old = OldRecord {
                kind: value.kind,
                inner: value.inner.clone(),
                name: value.name.clone(),
                first: value.first,
                second: value.second.clone(),
            };
            assert_eq!(encode::<Record, _>(&value).as_deref(), Ok(wire));
            assert_eq!(encode::<OldRecordFmt, _>(&old).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            let decoded: Record = crate::decode::<Record, _>(wire, &mut scratch).unwrap();
            assert_eq!(decoded, value);
            // The borrowed field points into the input.
            assert!(wire.as_ptr_range().contains(&decoded.kind.as_ptr()));
        }
        // A present field after an omitted one, and the error paths.
        let gap = encode::<Record, _>(&record(None, Some("42"))).unwrap_err();
        assert_eq!(gap.kind, Error::Invalid);
        assert_eq!(gap.path(), &[PathSegment::Field("second")]);
        let mut scratch = [0; 64];
        let bad = crate::decode::<Record, Record>(b"0100X73ABC", &mut scratch).unwrap_err();
        let mut scratch = [0; 64];
        let old_bad = crate::decode::<OldRecordFmt, OldRecord>(b"0100X73ABC", &mut scratch).unwrap_err();
        assert_eq!(bad, old_bad);
        assert_eq!(bad.path(), &[PathSegment::Field("inner"), PathSegment::Field("code")]);
    }

    #[test]
    fn records_nest_across_macros_and_keep_other_attributes() {
        let value = Wrapped {
            outer: Outer {
                record: Inner { code: "12".into() },
            },
        };
        assert_eq!(encode::<Wrapped, _>(&value).as_deref(), Ok(&b"12"[..]));
        let mut scratch = [0; 8];
        assert_eq!(crate::decode::<Wrapped, Wrapped>(b"12", &mut scratch), Ok(value));
        // Serde attributes stay on the struct; `#[wire]` ones are gone.
        let json = serde_json::to_string(&record(None, None)).unwrap();
        assert!(json.starts_with(r#"{"type":"0100","inner":{"code":"07"}"#), "{json}");
    }
}
