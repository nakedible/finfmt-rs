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
/// - `#[wire(delimited = b'|')]`: fields separated by a byte. The last field
///   takes the rest of the bounded input and may contain the separator;
///   earlier fields must not encode it, which debug builds assert. An `Option`
///   field may be anywhere, and an empty segment decodes as `None`, even one
///   encoded from a value that produced no bytes.
/// - `#[wire(bitmap = B)]`: a presence bitmap, then the fields it marks, in
///   field number order. `B` implements [`BitmapFormat`](crate::BitmapFormat),
///   naming the protocol's layout and word encoding once. Fields with a
///   `bit` are the body: an `Option` field is present when its bit is set, a
///   required field's bit must be set, and decoding rejects bits the record
///   does not declare. Fields without a `bit` come first and are the header,
///   written before the bitmap, such as the MTI; they cannot be `Option`. The
///   numbers are checked against the layout at compile time.
/// - `#[wire(ber_tlv)]`: BER-TLV entries, one per field with a `tag`. Encoding
///   writes them in declaration order; decoding accepts them in any order and
///   rejects a repeated known tag. An `Option` field is absent when its tag
///   is; a required field's tag must appear. `Some` of an empty value writes
///   the tag with a zero length. Unknown tags are rejected unless the record
///   has an `extras` field. `#[wire(ber_tlv(allow_zero_padding))]` also
///   accepts `00` bytes before, between and after entries; encoding never
///   writes them.
///
/// Fields take at most one `#[wire(...)]` attribute:
///
/// - `fmt = F`: the field's format. Without it, the field's type must be a
///   format itself, such as another wire struct; for an `Option<T>` field,
///   `T` must.
/// - `bit = N`: the field's number in a bitmap record.
/// - `tag = "9F02"`: the field's tag in a BER-TLV record, in uppercase hex; a
///   constant works too. Tags are checked at compile time: each must be one
///   valid tag, and none may repeat.
/// - `extras`: in a BER-TLV record, the field collecting unknown tags, such as
///   a `BTreeMap<String, String>` of uppercase hex keyed like `t9F03_unknown`.
///   At most one; it takes no `fmt` or `tag` and is written at its declared
///   position. Encoding rejects an entry that claims a declared tag, even one
///   whose field is absent.
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
/// concat record, a bitmap field number out of order, a repeated BER tag, and
/// an unknown argument are compile errors:
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
/// # type N2 = finfmt::Field<finfmt::Numeric<2, 2>, finfmt::Fixed<2>>;
/// # struct B;
/// # impl finfmt::BitmapFormat for B {
/// #     const LAYOUT: finfmt::BitmapLayout = finfmt::BitmapLayout::iso(1, 2);
/// #     type Word = finfmt::Identity;
/// # }
/// finfmt::wire_type! {
///     #[wire(bitmap = B)]
///     struct Descending {
///         #[wire(fmt = N2, bit = 3)]
///         third: String,
///         #[wire(fmt = N2, bit = 2)]
///         second: String,
///     }
/// }
/// ```
///
/// ```compile_fail
/// type Text = finfmt::Field<finfmt::Ascii<0, 9>, finfmt::Rest>;
/// finfmt::wire_type! {
///     #[wire(ber_tlv)]
///     struct Repeated {
///         #[wire(tag = "9F02", fmt = Text)]
///         first: String,
///         #[wire(tag = "9F02", fmt = Text)]
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
    ($kept:tt [delimited = $separator:expr ,]; ; $vis:vis struct $name:ident $lt:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([delimited $separator] $kept $vis $name $lt []) []; $($body)* }
    };
    ($kept:tt [bitmap = $format:ty ,]; ; $vis:vis struct $name:ident $lt:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([bitmap $format] $kept $vis $name $lt []) []; $($body)* }
    };
    ($kept:tt [ber_tlv ,]; ; $vis:vis struct $name:ident $lt:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([ber_tlv false] $kept $vis $name $lt []) []; $($body)* }
    };
    ($kept:tt [ber_tlv(allow_zero_padding) ,]; ; $vis:vis struct $name:ident $lt:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([ber_tlv true] $kept $vis $name $lt []) []; $($body)* }
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
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [];) => {
        $crate::__finfmt_wire_emit! { $layout $kept $vis $name $lt [$($done)*] }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field (Option<$inner>) opt ($inner) [$($args)*]
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : $ty:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field ($ty) req ($ty) [$($args)*]
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* $fvis:vis $field:ident : Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt [$($done)* {
                [$($fkept)* $(#[doc = $doc])*] ($fvis) $field (Option<$inner>) opt ($inner) []
            }]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt [$($done:tt)*]) [$($fkept:tt)*];
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
    ([delimited $separator:expr] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident ($ty:ty) $kind:ident ($default:ty) $args:tt })*]) => {
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
            #[inline(always)]
            #[allow(unused_assignments, unused_variables)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                let _ = (&output, &scratch, value);
                const SEPARATOR: u8 = $separator;
                let count = <[&str]>::len(&[$(stringify!($field)),*]);
                let mut position = 0;
                $(
                    position += 1;
                    $crate::__finfmt_wire_delimited_encode!(value, output, scratch, SEPARATOR, position < count;
                        $kind $field $crate::__finfmt_wire_fmt!($args $default));
                )*
                Ok(())
            }
        }

        impl<$($lt,)? '__finfmt_de> $crate::composite::FieldDecode<'__finfmt_de, $crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?])>
            for $name $(<$lt>)?
        {
            #[inline(always)]
            #[allow(unused_assignments, unused_variables)]
            fn decode_field(
                input: &mut &'__finfmt_de [u8],
                scratch: &mut &'__finfmt_de mut [u8],
            ) -> Result<$crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?]), $crate::CompositeError> {
                let _ = (&input, &scratch);
                const SEPARATOR: u8 = $separator;
                let count = <[&str]>::len(&[$(stringify!($field)),*]);
                let mut position = 0;
                $(
                    position += 1;
                    $crate::__finfmt_wire_delimited_decode!(input, scratch, SEPARATOR, position < count;
                        $kind $field $crate::__finfmt_wire_fmt!($args $default));
                )*
                Ok($name { $($field),* })
            }
        }
    };
    ([bitmap $format:ty] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = if let Err(message) = $crate::composite::check_bitmap_fields(
            <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
            &[$($crate::__finfmt_wire_bitmap_args! { {@bit; $kind $field} ($default); [] []; $($args)* }),*],
            &[$($crate::__finfmt_wire_is_optional!($kind)),*],
        ) {
            panic!("{}", message);
        };

        impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
            #[inline(always)]
            #[allow(unused_mut)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                let _ = (&output, &scratch, value);
                $($crate::__finfmt_wire_bitmap_args! { {@head_encode value, output, scratch; $kind $field} ($default); [] []; $($args)* })*
                let mut bitmap = $crate::bitmap::Bitmap::new();
                $($crate::__finfmt_wire_bitmap_args! { {@set bitmap, value; $kind $field} ($default); [] []; $($args)* })*
                $crate::bitmap::encode_bitmap::<<$format as $crate::bitmap::BitmapFormat>::Word>(
                    output,
                    &mut *scratch,
                    &bitmap,
                    <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
                )
                .map_err($crate::CompositeError::from)?;
                $($crate::__finfmt_wire_bitmap_args! { {@body_encode value, output, scratch; $kind $field} ($default); [] []; $($args)* })*
                Ok(())
            }
        }

        impl<$($lt,)? '__finfmt_de> $crate::composite::FieldDecode<'__finfmt_de, $crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?])>
            for $name $(<$lt>)?
        {
            #[inline(always)]
            #[allow(unused_mut)]
            fn decode_field(
                input: &mut &'__finfmt_de [u8],
                scratch: &mut &'__finfmt_de mut [u8],
            ) -> Result<$crate::__finfmt_wire_decoded!($name ['__finfmt_de] [$($lt)?]), $crate::CompositeError> {
                $($crate::__finfmt_wire_bitmap_args! { {@head_decode input, scratch; $kind $field} ($default); [] []; $($args)* })*
                let bitmap = $crate::bitmap::decode_bitmap::<<$format as $crate::bitmap::BitmapFormat>::Word>(
                    input,
                    &mut **scratch,
                    <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
                )
                .map_err($crate::CompositeError::from)?;
                // Fields this record does not declare are rejected before any is decoded.
                let mut unknown = bitmap;
                $($crate::__finfmt_wire_bitmap_args! { {@clear unknown; $kind $field} ($default); [] []; $($args)* })*
                if unknown != $crate::bitmap::Bitmap::new() {
                    $crate::__private::cold_path();
                    return Err($crate::Error::Invalid.into());
                }
                $($crate::__finfmt_wire_bitmap_args! { {@body_decode bitmap, input, scratch; $kind $field} ($default); [] []; $($args)* })*
                Ok($name { $($field),* })
            }
        }
    };
    ([ber_tlv $padding:tt] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = if let Err(message) = $crate::composite::check_ber_tags(&[
            $($crate::__finfmt_wire_ber_args! { {@tag; $kind $field} ($default); [] [] []; $($args)* }),*
        ]) {
            panic!("{}", message);
        };

        impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                let _ = (&output, &scratch, value);
                // Extras must not use a declared tag, even one whose field is absent.
                #[allow(dead_code)]
                const KNOWN_TAGS: &[&str] = &[$($crate::__finfmt_wire_ber_args! { {@known; $kind $field} ($default); [] [] []; $($args)* }),*];
                $($crate::__finfmt_wire_ber_args! {
                    {@encode value, output, scratch, KNOWN_TAGS; $kind $field} ($default); [] [] []; $($args)*
                })*
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
                $($crate::__finfmt_wire_ber_args! { {@init; $kind $field} ($default); [] [] []; $($args)* })*
                while let Some(entry) =
                    $crate::composite::decode_ber_tlv_collection_entry::<$padding>(input).map_err($crate::CompositeError::from)?
                {
                    let mut value_input = entry.value;
                    let mut matched = false;
                    let mut tag_hex = [0; $crate::primitive::bertlv::MAX_BER_TAG_HEX];
                    let tag_hex = $crate::primitive::bertlv::format_ber_tag_hex(&mut tag_hex, entry.tag);
                    $($crate::__finfmt_wire_ber_args! {
                        {@match tag_hex, value_input, scratch, matched; $kind $field} ($default); [] [] []; $($args)*
                    })*
                    $($crate::__finfmt_wire_ber_args! {
                        {@unknown entry, value_input, scratch, matched; $kind $field} ($default); [] [] []; $($args)*
                    })*
                    if !matched {
                        $crate::__private::cold_path();
                        return Err($crate::CompositeError::from($crate::Error::Invalid));
                    }
                }
                $($crate::__finfmt_wire_ber_args! { {@finish; $kind $field} ($default); [] [] []; $($args)* })*
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
    ([$(fmt = $fmt:ty,)? bit = $($args:tt)*] $default:ty) => {
        compile_error!("wire_type!: `bit` is for bitmap records")
    };
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

/// Encode one delimited field, then the separator unless it is the last. Only
/// the last field may contain the separator.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_delimited_encode {
    ($value:ident, $output:ident, $scratch:ident, $separator:ident, $more:expr; req $field:ident $fmt:ty) => {
        $crate::__finfmt_wire_delimited_encode!(@value &$value.$field, $output, $scratch, $separator, $more; $field $fmt);
        $crate::__finfmt_wire_delimited_encode!(@next $output, $separator, $more)
    };
    ($value:ident, $output:ident, $scratch:ident, $separator:ident, $more:expr; opt $field:ident $fmt:ty) => {
        if let Some(inner) = $value.$field.as_ref() {
            $crate::__finfmt_wire_delimited_encode!(@value inner, $output, $scratch, $separator, $more; $field $fmt);
        }
        $crate::__finfmt_wire_delimited_encode!(@next $output, $separator, $more)
    };
    (@value $value:expr, $output:ident, $scratch:ident, $separator:ident, $more:expr; $field:ident $fmt:ty) => {
        $crate::composite::encode_delimited_value::<_, $fmt>($output, $scratch, $value, if $more { Some($separator) } else { None })
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
    };
    (@next $output:ident, $separator:ident, $more:expr) => {
        if $more {
            $crate::composite::encode_delimiter($output, $separator)?;
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_delimited_decode {
    ($input:ident, $scratch:ident, $separator:ident, $more:expr; req $field:ident $fmt:ty) => {
        let segment = $crate::composite::decode_delimited_field($input, $separator, $more)?;
        let $field = $crate::composite::decode_delimited_value::<_, $fmt>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    ($input:ident, $scratch:ident, $separator:ident, $more:expr; opt $field:ident $fmt:ty) => {
        let segment = $crate::composite::decode_delimited_field($input, $separator, $more)?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $fmt>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
    };
}

/// Parse a bitmap field's `fmt` and `bit` arguments, in either order, then
/// generate the code for one phase of the record with
/// `__finfmt_wire_bitmap_field!`. Fields without a bit are the header.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_bitmap_args {
    ({$($phase:tt)*} ($default:ty); [$fmt:ty] [$($bit:literal)?];) => {
        $crate::__finfmt_wire_bitmap_field! { $($phase)* ($fmt) [$($bit)?] }
    };
    ({$($phase:tt)*} ($default:ty); [] [$($bit:literal)?];) => {
        $crate::__finfmt_wire_bitmap_field! { $($phase)* ($default) [$($bit)?] }
    };
    ($phase:tt $default:tt; [] $bit:tt; fmt = $fmt:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_bitmap_args! { $phase $default; [$fmt] $bit; $($($rest)*)? }
    };
    ($phase:tt $default:tt; $fmt:tt []; bit = $bit:literal $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_bitmap_args! { $phase $default; $fmt [$bit]; $($($rest)*)? }
    };
    ($phase:tt $default:tt; [$fmt:ty] $bit:tt; fmt = $($args:tt)*) => {
        compile_error!("wire_type!: a field takes one `fmt`")
    };
    ($phase:tt $default:tt; $fmt:tt [$bit:literal]; bit = $($args:tt)*) => {
        compile_error!("wire_type!: a field takes one `bit`")
    };
    ($phase:tt $default:tt; $fmt:tt $bit:tt; $($args:tt)+) => {
        compile_error!(concat!("wire_type!: unsupported #[wire] field arguments: ", stringify!($($args)+)))
    };
}

/// One phase of a bitmap record for one field, given its format and bit.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_bitmap_field {
    (@bit; $kind:ident $field:ident ($fmt:ty) [$bit:literal]) => { Some($bit) };
    (@bit; $kind:ident $field:ident ($fmt:ty) []) => { None };

    (@head_encode $value:ident, $output:ident, $scratch:ident; req $field:ident ($fmt:ty) []) => {
        $crate::__finfmt_wire_concat_encode!($value, $output, $scratch, unused; req $field $fmt);
    };
    (@head_encode $($rest:tt)*) => {};

    (@set $bitmap:ident, $value:ident; req $field:ident ($fmt:ty) [$bit:literal]) => {
        $bitmap.set($bit, true);
    };
    (@set $bitmap:ident, $value:ident; opt $field:ident ($fmt:ty) [$bit:literal]) => {
        if $value.$field.is_some() {
            $bitmap.set($bit, true);
        }
    };
    (@set $($rest:tt)*) => {};

    (@body_encode $value:ident, $output:ident, $scratch:ident; req $field:ident ($fmt:ty) [$bit:literal]) => {
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@body_encode $value:ident, $output:ident, $scratch:ident; opt $field:ident ($fmt:ty) [$bit:literal]) => {
        if let Some(inner) = $value.$field.as_ref() {
            <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        }
    };
    (@body_encode $($rest:tt)*) => {};

    (@head_decode $input:ident, $scratch:ident; req $field:ident ($fmt:ty) []) => {
        $crate::__finfmt_wire_concat_decode!($input, $scratch; req $field $fmt);
    };
    // Rejected by the record's compile-time check; bound to keep errors quiet.
    (@head_decode $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) []) => {
        let $field = None;
    };
    (@head_decode $($rest:tt)*) => {};

    (@clear $unknown:ident; $kind:ident $field:ident ($fmt:ty) [$bit:literal]) => {
        $unknown.set($bit, false);
    };
    (@clear $($rest:tt)*) => {};

    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; req $field:ident ($fmt:ty) [$bit:literal]) => {
        let $field = if $bitmap.get($bit) {
            <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
    };
    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) [$bit:literal]) => {
        let $field = if $bitmap.get($bit) {
            Some(
                <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
    };
    (@body_decode $($rest:tt)*) => {};
}

/// Parse a BER-TLV field's `fmt`, `tag` and `extras` arguments, in any
/// order, then generate one phase of the record with
/// `__finfmt_wire_ber_field!`.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_ber_args {
    ({$($phase:tt)*} ($default:ty); [] [$tag:expr] [];) => {
        $crate::__finfmt_wire_ber_field! { $($phase)* ($default) [$tag] }
    };
    ({$($phase:tt)*} ($default:ty); [$fmt:ty] [$tag:expr] [];) => {
        $crate::__finfmt_wire_ber_field! { $($phase)* ($fmt) [$tag] }
    };
    ({$($phase:tt)*} ($default:ty); [] [] [extras];) => {
        $crate::__finfmt_wire_ber_field! { $($phase)* extras }
    };
    ($phase:tt $default:tt; $fmt:tt [] [];) => {
        compile_error!("wire_type!: a BER-TLV field needs a `tag`, or `extras` for the unknown-tag collection")
    };
    ($phase:tt $default:tt; $fmt:tt $tag:tt [extras];) => {
        compile_error!("wire_type!: an `extras` field takes no `fmt` or `tag`")
    };
    ($phase:tt $default:tt; [] $tag:tt $extras:tt; fmt = $fmt:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_ber_args! { $phase $default; [$fmt] $tag $extras; $($($rest)*)? }
    };
    ($phase:tt $default:tt; $fmt:tt [] $extras:tt; tag = $tag:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_ber_args! { $phase $default; $fmt [$tag] $extras; $($($rest)*)? }
    };
    ($phase:tt $default:tt; $fmt:tt $tag:tt []; extras $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_ber_args! { $phase $default; $fmt $tag [extras]; $($($rest)*)? }
    };
    ($phase:tt $default:tt; $fmt:tt $tag:tt $extras:tt; $($args:tt)+) => {
        compile_error!(concat!("wire_type!: unsupported or repeated #[wire] field arguments: ", stringify!($($args)+)))
    };
}

/// One phase of a BER-TLV record for one field: a tagged field given its
/// format and tag, or the `extras` collection.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_ber_field {
    (@tag; $kind:ident $field:ident ($fmt:ty) [$tag:expr]) => {
        Some($tag)
    };
    (@tag; $kind:ident $field:ident extras) => {
        None
    };
    (@known; $kind:ident $field:ident ($fmt:ty) [$tag:expr]) => {
        $tag
    };
    // No unknown tag is empty, so the extras slot matches nothing.
    (@known; $kind:ident $field:ident extras) => {
        ""
    };

    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; req $field:ident ($fmt:ty) [$tag:expr]) => {
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            <$fmt as $crate::composite::FieldEncode<_>>::encode_field(value_out, scratch, &$value.$field)
        })?;
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; opt $field:ident ($fmt:ty) [$tag:expr]) => {
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$fmt as $crate::composite::FieldEncode<_>>::encode_field(value_out, scratch, inner)
            })?;
        }
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; req $field:ident extras) => {
        $crate::composite::BerTlvExtras::encode_unknowns(&$value.$field, $output, $scratch, $known)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; opt $field:ident extras) => {
        compile_error!("wire_type!: an `extras` field cannot be an Option");
    };

    (@init; $kind:ident $field:ident ($fmt:ty) [$tag:expr]) => {
        let mut $field = None;
    };
    (@init; $kind:ident $field:ident extras) => {
        let mut $field = ::core::default::Default::default();
    };

    (@match $tag_hex:ident, $value_input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident ($fmt:ty) [$tag:expr]) => {
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_hex,
                $tag,
                &mut $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field(value_input, scratch),
            )?;
        }
    };
    (@match $($rest:tt)*) => {};

    (@unknown $entry:ident, $value_input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident extras) => {
        if !$matched {
            $crate::composite::BerTlvExtras::decode_unknown(&mut $field, $entry.tag, $value_input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
            $matched = true;
        }
    };
    (@unknown $($rest:tt)*) => {};

    (@finish; req $field:ident ($fmt:ty) [$tag:expr]) => {
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
            }
        };
    };
    (@finish $($rest:tt)*) => {};
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

    type Text = Field<Ascii<0, 20>, crate::Rest>;

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(delimited = b'|')]
        struct Delimited<'a> {
            #[wire(fmt = Text)]
            first: &'a str,
            #[wire(fmt = N2)]
            middle: Option<u64>,
            inner: Inner,
            #[wire(fmt = Text)]
            last: Option<String>,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OldDelimited<'a> {
        first: &'a str,
        middle: Option<u64>,
        inner: Inner,
        last: Option<String>,
    }

    crate::delimited_format! {
        struct OldDelimitedFmt for<'a> OldDelimited<'a>, b'|' {
            first: Text,
            middle: Option<N2>,
            inner: Inner,
            last: Option<Text>,
        }
    }

    #[test]
    fn delimited_records_match_delimited_format() {
        for (middle, last, wire) in [(None, None, &b"AB||07|"[..]), (Some(15), Some("X|Y"), b"AB|15|07|X|Y")] {
            let value = Delimited {
                first: "AB",
                middle,
                inner: Inner { code: "07".into() },
                last: last.map(Into::into),
            };
            let old = OldDelimited {
                first: "AB",
                middle,
                inner: value.inner.clone(),
                last: value.last.clone(),
            };
            assert_eq!(encode::<Delimited, _>(&value).as_deref(), Ok(wire));
            assert_eq!(encode::<OldDelimitedFmt, _>(&old).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<Delimited, Delimited>(wire, &mut scratch), Ok(value));
        }
        // Errors match, segment by segment.
        for wire in [&b"AB|1"[..], b"AB|15|7X|", b"AB|15|07"] {
            let mut scratch = [0; 64];
            let new = crate::decode::<Delimited, Delimited>(wire, &mut scratch).unwrap_err();
            let mut scratch = [0; 64];
            let old = crate::decode::<OldDelimitedFmt, OldDelimited>(wire, &mut scratch).unwrap_err();
            assert_eq!(new, old);
        }
        // Only the last field may contain the separator.
        let inside = std::panic::catch_unwind(|| {
            encode::<Delimited, _>(&Delimited {
                first: "A|B",
                middle: None,
                inner: Inner { code: "07".into() },
                last: None,
            })
        });
        assert_eq!(inside.is_err(), cfg!(debug_assertions));
    }

    struct HexBitmap;
    impl crate::bitmap::BitmapFormat for HexBitmap {
        const LAYOUT: crate::bitmap::BitmapLayout = crate::bitmap::BitmapLayout::iso(1, 2);
        type Word = crate::UnpackNibbles<crate::primitive::nibble::UpperHexDigits>;
    }

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(bitmap = HexBitmap)]
        struct Message<'a> {
            #[wire(fmt = N4)]
            mti: &'a str,
            #[wire(fmt = Var, bit = 2)]
            pan: Option<String>,
            #[wire(bit = 3)]
            inner: Inner,
            /// A secondary-bitmap field.
            #[wire(bit = 70, fmt = N2)]
            code: Option<u64>,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OldMessage<'a> {
        mti: &'a str,
        pan: Option<String>,
        inner: Inner,
        code: Option<u64>,
    }

    crate::bitmap_format! {
        struct OldMessageFmt for<'a> OldMessage<'a>, <HexBitmap as crate::bitmap::BitmapFormat>::LAYOUT,
            <HexBitmap as crate::bitmap::BitmapFormat>::Word {
            head: { mti: N4, }
            2 => pan: Option<Var>,
            3 => inner: Inner,
            70 => code: Option<N2>,
        }
    }

    #[test]
    fn bitmap_records_match_bitmap_format() {
        for (pan, code) in [(None, None), (Some("4111"), Some(42))] {
            let value = Message {
                mti: "0100",
                pan: pan.map(Into::into),
                inner: Inner { code: "07".into() },
                code,
            };
            let old = OldMessage {
                mti: value.mti,
                pan: value.pan.clone(),
                inner: value.inner.clone(),
                code,
            };
            let wire = encode::<Message, _>(&value).unwrap();
            assert_eq!(encode::<OldMessageFmt, _>(&old), Ok(wire.clone()));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<Message, Message>(&wire, &mut scratch), Ok(value));
        }
        // Unknown and missing fields are rejected alike.
        // Field 4 is unknown; field 3 is required.
        for wire in [&b"0100100000000000000007"[..], b"010040000000000000000"] {
            let mut scratch = [0; 64];
            let new = crate::decode::<Message, Message>(wire, &mut scratch).unwrap_err();
            let mut scratch = [0; 64];
            let old = crate::decode::<OldMessageFmt, OldMessage>(wire, &mut scratch).unwrap_err();
            assert_eq!(new, old);
            assert_eq!(new.kind, Error::Invalid);
        }
    }

    type Hex4 = Field<crate::UpperHexEven<0, 16>, crate::Rest, crate::PackNibbles<crate::primitive::nibble::UpperHexDigits>>;
    const AMOUNT: &str = "9F02";

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(ber_tlv)]
        struct Emv {
            #[wire(tag = AMOUNT, fmt = Hex4)]
            amount: String,
            #[wire(extras)]
            extras: std::collections::BTreeMap<String, String>,
            #[wire(fmt = Hex4, tag = "5F2A")]
            currency: Option<String>,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(ber_tlv(allow_zero_padding))]
        struct Strict<'a> {
            #[wire(tag = "9F02", fmt = Field<Ascii<0, 8>, crate::Rest>)]
            amount: &'a str,
            #[wire(tag = "9F36", fmt = Hex4)]
            counter: Option<String>,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OldEmv {
        amount: String,
        extras: std::collections::BTreeMap<String, String>,
        currency: Option<String>,
    }

    crate::ber_tlv_format! {
        struct OldEmvFmt for OldEmv {
            extras: extras,
            AMOUNT => amount: Hex4,
            "5F2A" => currency: Option<Hex4>,
        }
    }

    #[derive(Debug, Clone, PartialEq)]
    struct OldStrict<'a> {
        amount: &'a str,
        counter: Option<String>,
    }

    crate::ber_tlv_format! {
        struct OldStrictFmt for<'a> OldStrict<'a>, allow_zero_padding = true {
            "9F02" => amount: Field<Ascii<0, 8>, crate::Rest>,
            "9F36" => counter: Option<Hex4>,
        }
    }

    #[test]
    fn ber_tlv_records_match_ber_tlv_format() {
        let mut extras = std::collections::BTreeMap::new();
        extras.insert("t9F03_unknown".to_owned(), "00".to_owned());
        let value = Emv {
            amount: "000000012345".into(),
            extras: extras.clone(),
            currency: Some("0978".into()),
        };
        let old = OldEmv {
            amount: value.amount.clone(),
            extras,
            currency: value.currency.clone(),
        };
        // The old macro writes extras last; this record declares them second.
        let wire = encode::<Emv, _>(&value).unwrap();
        assert_eq!(wire, b"\x9F\x02\x06\x00\x00\x00\x01\x23\x45\x9F\x03\x01\x00\x5F\x2A\x02\x09\x78");
        let old_wire = encode::<OldEmvFmt, _>(&old).unwrap();
        let mut scratch = [0; 128];
        assert_eq!(crate::decode::<Emv, Emv>(&wire, &mut scratch), Ok(value.clone()));
        let mut scratch = [0; 128];
        assert_eq!(crate::decode::<Emv, Emv>(&old_wire, &mut scratch), Ok(value));
        // Errors match: a repeated known tag, a missing required one, and an
        // extras entry claiming a declared tag.
        for wire in [&b"\x9F\x02\x01\x00\x9F\x02\x01\x00"[..], b"\x5F\x2A\x01\x00"] {
            let mut scratch = [0; 128];
            let new = crate::decode::<Emv, Emv>(wire, &mut scratch).unwrap_err();
            let mut scratch = [0; 128];
            assert_eq!(Err(new), crate::decode::<OldEmvFmt, OldEmv>(wire, &mut scratch).map(|_| ()));
        }
        let mut claimed = std::collections::BTreeMap::new();
        claimed.insert("t5F2A_unknown".to_owned(), "00".to_owned());
        let value = Emv {
            amount: "00".into(),
            extras: claimed.clone(),
            currency: None,
        };
        let old = OldEmv {
            amount: "00".into(),
            extras: claimed,
            currency: None,
        };
        assert_eq!(encode::<Emv, _>(&value), encode::<OldEmvFmt, _>(&old));

        // Without extras, unknown tags are rejected; padding is allowed when asked for.
        for wire in [
            &b"\x00\x9F\x02\x02AB\x00\x9F\x36\x01\x07\x00"[..],
            b"\x9F\x02\x02AB\x9F\x03\x01\x00",
        ] {
            let mut scratch = [0; 128];
            let new = crate::decode::<Strict, Strict>(wire, &mut scratch);
            let mut scratch = [0; 128];
            let old = crate::decode::<OldStrictFmt, OldStrict>(wire, &mut scratch);
            assert_eq!(
                new.as_ref().map(|value| (value.amount, value.counter.clone())),
                old.as_ref().map(|value| (value.amount, value.counter.clone()))
            );
        }
    }

    #[test]
    fn ber_tag_check_agrees_with_the_tag_parser() {
        let check = |bytes: &[u8]| {
            let hex: std::string::String = bytes.iter().map(|byte| format!("{byte:02X}")).collect();
            let valid = crate::primitive::bertlv::parse_ber_tag_hex(&hex).is_ok();
            assert_eq!(crate::composite::check_ber_tags(&[Some(&hex)]).is_ok(), valid, "{hex}");
        };
        for first in 0..=255u8 {
            check(&[first]);
            for second in 0..=255u8 {
                check(&[first, second]);
            }
            for rest in [[0x81, 0x01], [0x80, 0x01], [0x81, 0x80], [0xFF, 0x7F]] {
                check(&[first, rest[0], rest[1]]);
                check(&[first, rest[0], rest[1], 0x01]);
                check(&[first, rest[0], rest[1], 0x80]);
            }
        }
        for tag in ["9f02", "9F0", "", "9F02X1", "1F8181817F"] {
            assert!(crate::composite::check_ber_tags(&[Some(tag)]).is_err(), "{tag}");
        }
        assert!(crate::composite::check_ber_tags(&[Some("9F02"), None, Some("9F02")]).is_err());
        assert!(crate::composite::check_ber_tags(&[None, Some("5A"), None]).is_err());
        assert!(crate::composite::check_ber_tags(&[None, Some("5A"), Some("9F02")]).is_ok());
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
