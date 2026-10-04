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
/// - `#[wire(tlv(tag = T))]`: tagged entries of any other kind. Each entry is
///   a tag written by `T`, a fixed-width scalar format such as
///   `Field<Alphanum<2, 2>, Fixed<2>>` (or a binary byte shown as hex), then
///   the value in its field's own format, which carries its own length, if
///   any: `Field<Ascii<0, 99>, AsciiLength<2>>` for one tag, a three-digit
///   length or none at all for another. Otherwise it behaves like `ber_tlv`:
///   declaration order on encode, any order on decode, a repeated known tag
///   is rejected, and unknown tags need an `extras` field.
///
/// An `Option` field is recognized by its spelling: `Option<T>`, or a path
/// such as `std::option::Option<T>`; a type alias for one is a required
/// field.
///
/// Fields take at most one `#[wire(...)]` attribute:
///
/// - `fmt = F`: the field's format. Without it, the field's type must be a
///   format itself, such as another wire struct; for an `Option<T>` field,
///   `T` must.
/// - `bit = N`: the field's number in a bitmap record.
/// - `tag = "9F02"`: the field's tag in a BER-TLV record, in uppercase hex; a
///   constant works too. Tags are checked at compile time: each must be one
///   valid tag, and none may repeat. In a `tlv` record, the tag is the text
///   the tag format reads, such as `"05"`; none may repeat.
/// - `fixed_value = "H"`, with `fmt`: a constant written through the format
///   on a field of type `()`, for a spec field whose value never changes.
///   Decoding reads a value with the format and requires it to equal the
///   constant, as leniently as the format reads. Uses
///   [`FixedValue`](crate::FixedValue).
/// - `fixed_bytes = [b' '; 10]`: constant wire bytes on a field of type `()`,
///   such as a filler: a byte string, an array, or a constant of type
///   `[u8; N]`, `&[u8; N]` or `&[u8]`. Decoding requires exactly these bytes.
///   Uses [`FixedBytes`](crate::FixedBytes).
///
///   Fixed fields work in every layout: in a bitmap or BER-TLV record they
///   take a `bit` or `tag` like any field, and may be `Option<()>`. A
///   mismatch is `Invalid` with the field's name in the error path. Mark them
///   `#[serde(skip)]` when the struct is also serialized.
/// - `absent_bytes = [b' '; 12]`, `absent_value = "000000"` or
///   `absent = A`: on an `Option<T>` field whose bytes are always on the
///   wire, the encoding of `None`, through [`OptionAs`](crate::OptionAs).
///   `absent_bytes` gives raw wire bytes, spelled as for `fixed_bytes`,
///   with [`AbsentBytes`](crate::AbsentBytes); `absent_value` a value encoded
///   through the field's format, typed like any field value (`0u64` or
///   `"000000"`), compared after encoding it per decode; `absent` a custom
///   [`AbsentFmt`](crate::AbsentFmt). Decoding matches the pattern first and
///   otherwise decodes with the format, whose errors are returned. A present
///   value that encodes like the pattern reads back as `None`. The field is
///   required by its container: a bitmap sets its bit and BER-TLV writes its
///   tag even for `None`, and in a concat record it is not part of the
///   optional tail.
/// - `extras`: in a BER-TLV or TLV record, the field collecting unknown tags.
///   In BER-TLV it is such as a `BTreeMap<String, String>` of uppercase hex
///   values keyed like `t9F03_unknown`. In TLV, `fmt` reads every unknown
///   tag's value, and the key is the tag's text: `#[wire(extras, fmt =
///   Field<Ascii<0, 99>, AsciiLength<2>>)]` on a `BTreeMap<String, String>`.
///   At most one; it takes no `fmt` or `tag` and is written at its declared
///   position. Encoding rejects an entry that claims a declared tag, even one
///   whose field is absent.
///
/// - `select = field`: the field is a selected enum, decoded by the value of
///   an earlier text field of the same record, such as the MTI. Not in
///   BER-TLV records.
///
///   `select = func(a, b)` takes the key from a function of earlier fields
///   instead: it receives a reference to each and returns anything that is
///   `AsRef<str>`. That covers a key that is part of a field, a key made from
///   several fields, many codes sharing one variant, and a key field that is
///   an `Option`: `fn kind(k: &Option<String>) -> &str { k.as_deref().unwrap_or("none") }`
///   with a variant `#[wire(rename = "none")]`.
///
///   The selected field may itself be an `Option`, present as its container
///   says: a bitmap bit, a concat tail, a non-empty delimited segment.
///
/// Enums state their kind too:
///
/// - `#[wire(names)]`: a unit enum written as text, its value type for a
///   field whose `fmt` decides the encoding. Each variant's name is
///   `#[wire(rename = "…")]` or its identifier. An unknown name is `Invalid`.
/// - `#[wire(codes)]`: a unit enum written as a number, through the field's
///   integer encoding, so a binary or packed integer field needs no text.
///   Every variant has `#[wire(code = N)]`; an unknown code is `Invalid`.
/// - `#[wire(selected)]`: an enum whose variant an earlier field chooses. Each
///   variant has a name, as above, and may hold one value, a record such as
///   `Request(Request)` or one with a format, `Response(#[wire(fmt = F)]
///   Response)`. Decoding takes the variant whose name equals the key; its
///   body's errors are returned, with the variant in the path. A variant may
///   also accept other keys with `#[wire(alias = "0101")]`, repeated as
///   needed; names and aliases may not repeat across the enum. One
///   `#[wire(other)]` variant holding a value takes any other key's body;
///   without it, an unknown key is `Invalid`. Encoding writes only the
///   variant's body, so the key field must agree with the variant; nothing
///   checks it; `wire_name()` gives a variant's primary name for setting the
///   key, `None` for `other`. The enum implements
///   [`FieldEncode`](crate::FieldEncode) and
///   [`ContextDecode<'de, Self, str>`](crate::ContextDecode), whose context is
///   the key, so a `Frame` around it passes the key through too:
///   `#[wire(select = kind, fmt = Frame<AsciiLength<3>, Body>)]`.
///
/// ```
/// use finfmt::{Field, Fixed, Numeric};
///
/// type N2 = Field<Numeric<2, 2>, Fixed<2>>;
/// type N4 = Field<Numeric<4, 4>, Fixed<4>>;
///
/// finfmt::wire_type! {
///     #[derive(Debug, PartialEq)]
///     #[wire(concat)]
///     pub struct Message {
///         #[wire(fmt = N4)]
///         pub mti: String,
///         #[wire(select = mti)]
///         pub body: Body,
///     }
///
///     #[derive(Debug, PartialEq)]
///     #[wire(selected)]
///     pub enum Body {
///         #[wire(rename = "0100")]
///         Request(Request),
///         #[wire(rename = "0800")]
///         Echo,
///     }
///
///     #[derive(Debug, PartialEq)]
///     #[wire(concat)]
///     pub struct Request {
///         #[wire(fmt = N2)]
///         pub channel: Channel,
///     }
///
///     #[derive(Debug, PartialEq)]
///     #[wire(names)]
///     pub enum Channel {
///         #[wire(rename = "01")]
///         Pos,
///         #[wire(rename = "02")]
///         Atm,
///     }
/// }
///
/// let message = Message { mti: "0100".into(), body: Body::Request(Request { channel: Channel::Atm }) };
/// let mut output = [0; 16];
/// let used = finfmt::encode::<Message, _>(&mut output, &mut [0; 32], &message).unwrap();
/// assert_eq!(&output[..used], b"010002");
/// let decoded: Message = finfmt::decode::<Message, _>(&output[..used], &mut [0; 32]).unwrap();
/// assert_eq!(decoded, message);
/// ```
///
/// A struct or enum may have one lifetime parameter, for fields borrowed from
/// the input or scratch.
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
/// concat record, a bitmap field number out of order, a repeated BER tag, an
/// unknown argument, and `#[cfg]` on an item, field or variant are compile
/// errors:
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
///     struct Conditional {
///         #[wire(fmt = N2)]
///         #[cfg(any())]
///         code: String,
///     }
/// }
/// ```
///
/// ```compile_fail
/// type N2 = finfmt::Field<finfmt::Numeric<2, 2>, finfmt::Fixed<2>>;
/// finfmt::wire_type! {
///     #[wire(selected)]
///     enum Repeated {
///         #[wire(rename = "0100", alias = "0101")]
///         Request(#[wire(fmt = N2)] String),
///         #[wire(rename = "0101")]
///         Advice(#[wire(fmt = N2)] String),
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
        $($crate::__finfmt_wire_item! { [] []; $(#[$($attr)*])*; $vis $kw $name [$($lt)?] [$($lt)? '__finfmt_de] { $($body)* } })*
    };
}

/// Reject `#[cfg]` on an item, field or variant: the generated code would
/// still name what it removes. One step for all attributes.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_no_cfg {
    ($(#[$name:ident $($args:tt)*])*) => {
        $($crate::__finfmt_wire_no_cfg! { @attr $name })*
    };
    (@attr cfg) => {
        compile_error!("wire_type!: #[cfg] is not supported on items, fields or variants; put it on the whole wire_type! block");
    };
    (@attr $name:ident) => {};
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
    ($kept:tt [concat ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { (concat $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [delimited = $separator:expr ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([delimited $separator] $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [bitmap = $format:ty ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([bitmap $format] $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [ber_tlv ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([ber_tlv false] $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [ber_tlv(allow_zero_padding) ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([ber_tlv true] $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [tlv(tag = $tag:ty $(,)?) ,]; ; $vis:vis struct $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_fields! { ([tlv ($tag)] $kept $vis $name $lt $de [] [0]) []; $($body)* }
    };
    ($kept:tt [$kind:ident ,]; ; $vis:vis enum $name:ident $lt:tt $de:tt { $($body:tt)* }) => {
        $crate::__finfmt_wire_variants! { ($kind $kept $vis $name $lt $de []) []; $($body)* }
    };
    ($kept:tt []; ; $vis:vis enum $name:ident $($rest:tt)*) => {
        compile_error!(concat!(
            "wire_type!: `",
            stringify!($name),
            "` needs a kind: #[wire(names)], #[wire(codes)] or #[wire(selected)]"
        ));
    };
    ($kept:tt []; ; $vis:vis $kw:ident $name:ident $($rest:tt)*) => {
        compile_error!(concat!("wire_type!: `", stringify!($name), "` needs a layout attribute, such as #[wire(concat)]"));
    };
    ($kept:tt [$($wire:tt)*]; ; $vis:vis $kw:ident $name:ident $($rest:tt)*) => {
        compile_error!(concat!("wire_type!: unsupported layout for `", stringify!($name), "`: ", stringify!($($wire)*)));
    };
}

/// Parse one field per step into `{ [kept attributes] (vis) name (position)
/// (type) kind (default format) [wire arguments] }`. `Option` may be spelled
/// as a `std::option` or `core::option` path. Doc comments are taken in the same
/// step; other attributes before the `#[wire]` one move into the field's kept
/// list one per step.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_fields {
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*] [$($pos:tt)*]) [];) => {
        $crate::__finfmt_wire_emit! { $layout $kept $vis $name $lt $de [$($done)*] }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*] [$($pos:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : $(::)? $(std::option::)? $(core::option::)? Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt $de [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field ($($pos)*) (::core::option::Option<$inner>) opt ($inner) [$($args)*]
            }] [$($pos)* + 1]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*] [$($pos:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $fvis:vis $field:ident : $ty:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt $de [$($done)* {
                [$($fkept)* $(#[doc = $doc])* $(#[$($attr)*])*] ($fvis) $field ($($pos)*) ($ty) req ($ty) [$($args)*]
            }] [$($pos)* + 1]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*] [$($pos:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* $fvis:vis $field:ident : $(::)? $(std::option::)? $(core::option::)? Option<$inner:ty> $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt $de [$($done)* {
                [$($fkept)* $(#[doc = $doc])*] ($fvis) $field ($($pos)*) (::core::option::Option<$inner>) opt ($inner) []
            }] [$($pos)* + 1]) [];
            $($($rest)*)?
        }
    };
    (($layout:tt $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*] [$($pos:tt)*]) [$($fkept:tt)*];
        $(#[doc = $doc:tt])* $fvis:vis $field:ident : $ty:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_fields! {
            ($layout $kept $vis $name $lt $de [$($done)* { [$($fkept)* $(#[doc = $doc])*] ($fvis) $field ($($pos)*) ($ty) req ($ty) [] }] [$($pos)* + 1]) [];
            $($($rest)*)?
        }
    };
    ($state:tt [$($fkept:tt)*]; #[$($attr:tt)*] $($rest:tt)*) => {
        $crate::__finfmt_wire_fields! { $state [$($fkept)* #[$($attr)*]]; $($rest)* }
    };
}

/// Parse one variant per step into `{ [kept attributes] name [wire arguments]
/// [payload] }`, where the payload is empty or `($type) ($format) binding`.
/// Doc comments are taken in the same step, like a struct's fields.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_variants {
    (($kind:ident $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*]) [];) => {
        $crate::__finfmt_wire_enum! { $kind $kept $vis $name $lt $de [$($done)*] }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $variant:ident
        (#[wire(fmt = $fmt:ty)] $payload:ty $(,)?) $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state
            { [$($vkept)* $(#[doc = $doc])* $(#[$($attr)*])*] $variant [$($args)*] [($payload) ($fmt) inner] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $variant:ident
        ($payload:ty $(,)?) $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state
            { [$($vkept)* $(#[doc = $doc])* $(#[$($attr)*])*] $variant [$($args)*] [($payload) ($payload) inner] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* #[wire($($args:tt)*)] $(#[$($attr:tt)*])* $variant:ident
        $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state
            { [$($vkept)* $(#[doc = $doc])* $(#[$($attr)*])*] $variant [$($args)*] [] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* $variant:ident (#[wire(fmt = $fmt:ty)] $payload:ty $(,)?) $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state
            { [$($vkept)* $(#[doc = $doc])*] $variant [] [($payload) ($fmt) inner] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* $variant:ident ($payload:ty $(,)?) $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state
            { [$($vkept)* $(#[doc = $doc])*] $variant [] [($payload) ($payload) inner] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; $(#[doc = $doc:tt])* $variant:ident $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variants! { @push $state { [$($vkept)* $(#[doc = $doc])*] $variant [] [] }; $($($rest)*)? }
    };
    ($state:tt [$($vkept:tt)*]; #[$($attr:tt)*] $($rest:tt)*) => {
        $crate::__finfmt_wire_variants! { $state [$($vkept)* #[$($attr)*]]; $($rest)* }
    };
    (@push ($kind:ident $kept:tt $vis:vis $name:ident $lt:tt $de:tt [$($done:tt)*]) $entry:tt; $($rest:tt)*) => {
        $crate::__finfmt_wire_variants! { ($kind $kept $vis $name $lt $de [$($done)* $entry]) []; $($rest)* }
    };
}

/// Emit the enum without its `#[wire]` attributes, and its impls: a value
/// mapping for a unit enum, or a format with a selected decode.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_enum {
    ($kind:ident [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($vkept:tt)*] $variant:ident [$($args:tt)*] [$(($payload:ty) ($fmt:ty) $bind:ident)?] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($vkept)*)* }
        $($kept)*
        $vis enum $name $(<$lt>)? {
            $($($vkept)* $variant $(($payload))?,)*
        }

        const _: () = {
            $($crate::__finfmt_wire_variant! { {@check $kind} $variant [$(($fmt) $bind)?]; [] [] [] []; $($args)* })*
            $crate::__finfmt_wire_enum! { @impl $kind $name [$($lt)?] [$de]
                [$({ $variant [$($args)*] [$(($fmt) $bind)?] })*]
                (false $($(|| <$fmt as $crate::composite::FieldDecode<$de, $payload>>::TAKES_REST)?)*) }
        };
    };

    (@impl names $name:ident [$($lt:lifetime)?] [$de:lifetime] [$({ $variant:ident $args:tt $payload:tt })*] $takes_rest:tt) => {
        const _: () = if let Err(message) = $crate::composite::check_variant_names(&[
            $(Some($crate::__finfmt_wire_variant! { {@name} $variant $payload; [] [] [] []; $args })),*
        ]) {
            panic!("{}", message);
        };

        impl $crate::ScalarEncode for $name {
            #[inline]
            fn encode_scalar<F: $crate::ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), $crate::Error> {
                F::encode_str(output, scratch, match self {
                    $($name::$variant => $crate::__finfmt_wire_variant! { {@name} $variant $payload; [] [] [] []; $args },)*
                })
            }
        }

        impl<'de> $crate::ScalarDecode<'de> for $name {
            #[inline]
            fn decode_scalar<F: $crate::ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, $crate::Error> {
                $crate::composite::decode_mapped_text::<F, Self>(input, scratch, |text| {
                    $($crate::__finfmt_wire_variant! { {@decode_name text} $variant $payload; [] [] [] []; $args })*
                    None
                })
            }
        }
    };

    (@impl codes $name:ident [$($lt:lifetime)?] [$de:lifetime] [$({ $variant:ident $args:tt $payload:tt })*] $takes_rest:tt) => {
        const _: () = if let Err(message) = $crate::composite::check_variant_codes(&[
            $($crate::__finfmt_wire_variant! { {@code} $variant $payload; [] [] [] []; $args }),*
        ]) {
            panic!("{}", message);
        };

        impl $crate::ScalarEncode for $name {
            #[inline]
            fn encode_scalar<F: $crate::ScalarFmt>(&self, output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), $crate::Error> {
                F::encode_u64(output, scratch, match self {
                    $($name::$variant => $crate::__finfmt_wire_variant! { {@code} $variant $payload; [] [] [] []; $args },)*
                })
            }
        }

        impl<'de> $crate::ScalarDecode<'de> for $name {
            #[inline]
            fn decode_scalar<F: $crate::ScalarFmt>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self, $crate::Error> {
                let code = F::decode_u64(input, scratch)?;
                $($crate::__finfmt_wire_variant! { {@decode_code code} $variant $payload; [] [] [] []; $args })*
                $crate::__private::cold_path();
                Err($crate::Error::Invalid)
            }
        }
    };

    (@impl selected $name:ident [$($lt:lifetime)?] [$de:lifetime] [$({ $variant:ident $args:tt [$(($fmt:ty) $bind:ident)?] })*]
        ($($takes_rest:tt)*)) => {
        const _: () = if let Err(message) = $crate::composite::check_variant_keys(&[
            $($crate::__finfmt_wire_variant! { {@selected_keys} $variant [$(($fmt) $bind)?]; [] [] [] []; $args }),*
        ]) {
            panic!("{}", message);
        };

        impl $(<$lt>)? $name $(<$lt>)? {
            /// The variant's wire name, the key that selects it; `None` for
            /// the `other` variant, whose key is not kept.
            #[allow(dead_code)]
            pub fn wire_name(&self) -> Option<&'static str> {
                match self {
                    $($name::$variant { .. } => $crate::__finfmt_wire_variant! { {@selected_name} $variant [$(($fmt) $bind)?]; [] [] [] []; $args },)*
                }
            }
        }

        impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                let _ = (&output, &scratch);
                match value {
                    $($name::$variant $(($bind))? => {
                        $crate::__finfmt_wire_variant! { {@encode output, scratch} $variant [$(($fmt) $bind)?]; [] [] [] []; $args }
                    })*
                }
            }
        }

        impl<$de> $crate::composite::ContextDecode<$de, $name $(<$lt>)?, str> for $name $(<$lt>)? {
            // Any variant's body may take the rest.
            const TAKES_REST: bool = $($takes_rest)*;

            #[inline(always)]
            #[allow(unreachable_code)]
            fn decode_with(
                input: &mut &$de [u8],
                scratch: &mut &$de mut [u8],
                key: &str,
            ) -> Result<Self, $crate::CompositeError> {
                let _ = (&input, &scratch);
                $($crate::__finfmt_wire_variant! { {@decode input, scratch, key, $name} $variant [$(($fmt) $bind)?]; [] [] [] []; $args })*
                $($crate::__finfmt_wire_variant! { {@other input, scratch, $name} $variant [$(($fmt) $bind)?]; [] [] [] []; $args })*
                $crate::__private::cold_path();
                Err($crate::Error::Invalid.into())
            }
        }
    };

    (@impl $kind:ident $($rest:tt)*) => {
        compile_error!(concat!(
            "wire_type!: unsupported enum kind `",
            stringify!($kind),
            "`; use #[wire(names)], #[wire(codes)] or #[wire(selected)]"
        ));
    };
}

/// Parse a variant's `rename`, `code`, `other` and `alias` arguments, then
/// generate one phase of its enum's code. The slots are
/// `[rename] [code] [other] [aliases]`.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_variant {
    ({$($phase:tt)*} $variant:ident $payload:tt; [$($rename:literal)?] [$($code:literal)?] [$($other:ident)?] [$($alias:literal)*];) => {
        $crate::__finfmt_wire_variant! { @phase $($phase)* $variant [$($rename)?] [$($code)?] [$($other)?] [$($alias)*] $payload }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; [] $code:tt $other:tt $aliases:tt; rename = $rename:literal $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variant! { {$($phase)*} $variant $payload; [$rename] $code $other $aliases; $($($rest)*)? }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; $rename:tt [] $other:tt $aliases:tt; code = $code:literal $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variant! { {$($phase)*} $variant $payload; $rename [$code] $other $aliases; $($($rest)*)? }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; $rename:tt $code:tt [] $aliases:tt; other $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variant! { {$($phase)*} $variant $payload; $rename $code [other] $aliases; $($($rest)*)? }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; $rename:tt $code:tt $other:tt [$($alias:literal)*]; alias = $new:literal $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_variant! { {$($phase)*} $variant $payload; $rename $code $other [$($alias)* $new]; $($($rest)*)? }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; $rename:tt $code:tt $other:tt $aliases:tt; [$($args:tt)*]) => {
        $crate::__finfmt_wire_variant! { {$($phase)*} $variant $payload; $rename $code $other $aliases; $($args)* }
    };
    ({$($phase:tt)*} $variant:ident $payload:tt; $rename:tt $code:tt $other:tt $aliases:tt; $($args:tt)+) => {
        compile_error!(concat!("wire_type!: unsupported or repeated #[wire] variant arguments: ", stringify!($($args)+)))
    };

    // Which arguments each enum kind takes.
    (@phase @check names $variant:ident [$($rename:literal)?] [] [] [] []) => {};
    (@phase @check codes $variant:ident [] [$code:literal] [] [] []) => {};
    (@phase @check selected $variant:ident [$($rename:literal)?] [] [] $aliases:tt $payload:tt) => {};
    (@phase @check selected $variant:ident [] [] [other] [] [$($payload:tt)+]) => {};
    (@phase @check selected $variant:ident $rename:tt [] [other] $aliases:tt $payload:tt) => {
        compile_error!(concat!("wire_type!: the `other` variant `", stringify!($variant), "` holds the body and has no name or alias"));
    };
    (@phase @check $kind:ident $variant:ident $rename:tt $code:tt $other:tt [$($alias:literal)+] $payload:tt) => {
        compile_error!("wire_type!: `alias` is for #[wire(selected)] enums so far");
    };
    (@phase @check codes $variant:ident [] [] $other:tt $aliases:tt []) => {
        compile_error!(concat!("wire_type!: variant `", stringify!($variant), "` of a codes enum needs a `code`"));
    };
    (@phase @check $kind:ident $variant:ident $rename:tt $code:tt [other] $aliases:tt $payload:tt) => {
        compile_error!("wire_type!: `other` is only for selected enums so far");
    };
    (@phase @check $kind:ident $variant:ident $rename:tt $code:tt $other:tt $aliases:tt [$($payload:tt)+]) => {
        compile_error!(concat!("wire_type!: variant `", stringify!($variant), "` of a unit enum cannot hold a value"));
    };
    (@phase @check names $variant:ident $rename:tt [$code:literal] $other:tt $aliases:tt $payload:tt) => {
        compile_error!("wire_type!: `code` is for #[wire(codes)] enums");
    };
    (@phase @check $kind:ident $variant:ident [$rename:literal] $code:tt $other:tt $aliases:tt $payload:tt) => {
        compile_error!("wire_type!: `rename` is for #[wire(names)] and #[wire(selected)] enums");
    };
    (@phase @check selected $variant:ident $rename:tt [$code:literal] $other:tt $aliases:tt $payload:tt) => {
        compile_error!("wire_type!: `code` is for #[wire(codes)] enums");
    };
    // An unknown kind is reported once, by the enum itself.
    (@phase @check $kind:ident $($rest:tt)*) => {};

    // Unit enums.
    (@phase @name $variant:ident [$rename:literal] $($rest:tt)*) => { $rename };
    (@phase @name $variant:ident [] $($rest:tt)*) => { stringify!($variant) };
    (@phase @code $variant:ident $rename:tt [$code:literal] $($rest:tt)*) => { $code };
    (@phase @code $variant:ident $rename:tt [] $($rest:tt)*) => { 0 };
    (@phase @decode_name $text:ident $variant:ident $rename:tt $($rest:tt)*) => {
        if $text == $crate::__finfmt_wire_variant! { @phase @name $variant $rename } {
            return Some(Self::$variant);
        }
    };
    (@phase @decode_code $code_value:ident $variant:ident $rename:tt [$code:literal] $($rest:tt)*) => {
        if $code_value == $code {
            return Ok(Self::$variant);
        }
    };
    (@phase @decode_code $($rest:tt)*) => {};

    // Selected enums: the primary name and any alias key decode; `other`
    // takes the rest.
    (@phase @selected_name $variant:ident $rename:tt $code:tt [other] $aliases:tt $payload:tt) => { None };
    (@phase @selected_name $variant:ident $rename:tt $($rest:tt)*) => {
        Some($crate::__finfmt_wire_variant! { @phase @name $variant $rename })
    };
    // A variant's keys, for the duplicate check: none for `other`.
    (@phase @selected_keys $variant:ident $rename:tt $code:tt [other] $aliases:tt $payload:tt) => { &[None] };
    (@phase @selected_keys $variant:ident $rename:tt $code:tt $other:tt [$($alias:literal)*] $payload:tt) => {
        &[Some($crate::__finfmt_wire_variant! { @phase @name $variant $rename }) $(, Some($alias))*]
    };
    (@phase @encode $output:ident, $scratch:ident $variant:ident $rename:tt $code:tt $other:tt $aliases:tt [($fmt:ty) $bind:ident]) => {
        $crate::__private::encode_variant::<_, $fmt>($output, $scratch, $bind)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($variant)))
    };
    (@phase @encode $output:ident, $scratch:ident $variant:ident $rename:tt $code:tt $other:tt $aliases:tt []) => {
        Ok(())
    };
    (@phase @decode $input:ident, $scratch:ident, $key:ident, $name:ident $variant:ident $rename:tt $code:tt [other] $aliases:tt $payload:tt) => {};
    (@phase @decode $input:ident, $scratch:ident, $key:ident, $name:ident $variant:ident $rename:tt $code:tt [] [$($alias:literal)*] [($fmt:ty) $bind:ident]) => {
        if $key == $crate::__finfmt_wire_variant! { @phase @name $variant $rename } $(|| $key == $alias)* {
            return $crate::__private::decode_variant::<_, _, $fmt, _>($input, $scratch, $name::$variant)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($variant)));
        }
    };
    (@phase @decode $input:ident, $scratch:ident, $key:ident, $name:ident $variant:ident $rename:tt $code:tt [] [$($alias:literal)*] []) => {
        if $key == $crate::__finfmt_wire_variant! { @phase @name $variant $rename } $(|| $key == $alias)* {
            return Ok($name::$variant);
        }
    };
    (@phase @other $input:ident, $scratch:ident, $name:ident $variant:ident $rename:tt $code:tt [other] $aliases:tt [($fmt:ty) $bind:ident]) => {
        return $crate::__private::decode_variant::<_, _, $fmt, _>($input, $scratch, $name::$variant)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($variant)));
    };
    (@phase @other $($rest:tt)*) => {};
}

/// Emit the struct without its `#[wire]` attributes, and its format impls.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_emit {
    (concat [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident $pos:tt ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($fkept)*)* }
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = assert!(
            $crate::composite::optional_fields_trail(&[$($crate::__finfmt_wire_args! { {__finfmt_wire_consts @optional} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*]),
            concat!(
                "wire_type!: in the concat record `",
                stringify!($name),
                "`, only fields that may be omitted (Option without an absent form) may follow one"
            )
        );

        const _: () = {
            // Generated constants for fixed and absent fields, and the format impls.
            $crate::__finfmt_wire_consts! { @all concat $name [$($lt)?] [$({ $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*] }

            impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                    let _ = (&output, &scratch, value);
                    let mut omitted = false;
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_concat @encode value, output, scratch, omitted;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok(())
                }
            }

            impl<$de> $crate::composite::FieldDecode<$de, $name $(<$lt>)?> for $name $(<$lt>)? {
                // The last field decides.
                const TAKES_REST: bool = match <[bool]>::last(&[
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_consts @rest concat $de ($ty) ($default)} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*
                ]) {
                    Some(last) => *last,
                    None => false,
                };

                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn decode_field(
                    input: &mut &$de [u8],
                    scratch: &mut &$de mut [u8],
                ) -> Result<Self, $crate::CompositeError> {
                    let _ = (&input, &scratch);
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_concat @decode input, scratch;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok($name { $($field),* })
                }
            }
        };
    };
    ([delimited $separator:expr] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident $pos:tt ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($fkept)*)* }
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = {
            // Generated constants for fixed and absent fields, and the format impls.
            $crate::__finfmt_wire_consts! { @all delimited $name [$($lt)?] [$({ $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*] }

            impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                    let _ = (&output, &scratch, value);
                    const SEPARATOR: u8 = $separator;
                    let count = <[&str]>::len(&[$(stringify!($field)),*]);
                    let mut position = 0;
                    $(
                        position += 1;
                        $crate::__finfmt_wire_args! { {__finfmt_wire_delimited @encode value, output, scratch, SEPARATOR, position < count;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }
                    )*
                    Ok(())
                }
            }

            impl<$de> $crate::composite::FieldDecode<$de, $name $(<$lt>)?> for $name $(<$lt>)? {
                // The last field takes the rest.
                const TAKES_REST: bool = !<[&str]>::is_empty(&[$(stringify!($field)),*]);

                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn decode_field(
                    input: &mut &$de [u8],
                    scratch: &mut &$de mut [u8],
                ) -> Result<Self, $crate::CompositeError> {
                    let _ = (&input, &scratch);
                    const SEPARATOR: u8 = $separator;
                    let count = <[&str]>::len(&[$(stringify!($field)),*]);
                    let mut position = 0;
                    $(
                        position += 1;
                        $crate::__finfmt_wire_args! { {__finfmt_wire_delimited @decode input, scratch, SEPARATOR, position < count;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }
                    )*
                    Ok($name { $($field),* })
                }
            }
        };
    };
    ([bitmap $format:ty] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident $pos:tt ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($fkept)*)* }
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = if let Err(message) = $crate::composite::check_bitmap_fields(
            <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
            &[$($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @bit} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*],
            &[$($crate::__finfmt_wire_args! { {__finfmt_wire_consts @optional} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*],
        ) {
            panic!("{}", message);
        };

        const _: () = {
            // Generated constants for fixed and absent fields, and the format impls.
            $crate::__finfmt_wire_consts! { @all bitmap $name [$($lt)?] [$({ $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*] }

            impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                    let _ = (&output, &scratch, value);
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @head_encode value, output, scratch;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    let mut bitmap = $crate::bitmap::Bitmap::new();
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @set bitmap, value;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    $crate::bitmap::encode_bitmap::<<$format as $crate::bitmap::BitmapFormat>::Word>(
                        output,
                        &mut *scratch,
                        &bitmap,
                        <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
                    )
                    .map_err($crate::CompositeError::from)?;
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @body_encode value, output, scratch;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok(())
                }
            }

            impl<$de> $crate::composite::FieldDecode<$de, $name $(<$lt>)?> for $name $(<$lt>)? {
                // The last field decides.
                const TAKES_REST: bool = match <[bool]>::last(&[
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_consts @rest bitmap $de ($ty) ($default)} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*
                ]) {
                    Some(last) => *last,
                    None => false,
                };

                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn decode_field(
                    input: &mut &$de [u8],
                    scratch: &mut &$de mut [u8],
                ) -> Result<Self, $crate::CompositeError> {
                    let _ = (&input, &scratch);
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @head_decode input, scratch;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    let bitmap = $crate::bitmap::decode_bitmap::<<$format as $crate::bitmap::BitmapFormat>::Word>(
                        input,
                        &mut **scratch,
                        <$format as $crate::bitmap::BitmapFormat>::LAYOUT,
                    )
                    .map_err($crate::CompositeError::from)?;
                    // Fields this record does not declare are rejected before any is decoded.
                    let mut unknown = bitmap;
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @clear unknown;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    if unknown != $crate::bitmap::Bitmap::new() {
                        $crate::__private::cold_path();
                        return Err($crate::Error::Invalid.into());
                    }
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_bitmap @body_decode bitmap, input, scratch;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok($name { $($field),* })
                }
            }
        };
    };
    ([tlv ($tagfmt:ty)] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident $pos:tt ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($fkept)*)* }
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = if let Err(message) = $crate::composite::check_tlv_tags(&[$($crate::__finfmt_wire_args! { {__finfmt_wire_ber @tag} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*]) {
            panic!("{}", message);
        };

        const _: () = {
            // Generated constants for fixed and absent fields, and the format impls.
            $crate::__finfmt_wire_consts! { @all tlv $name [$($lt)?] [$({ $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*] }

            impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                    let _ = (&output, &scratch, value);
                    // Extras must not use a declared tag, even one whose field is absent.
                    #[allow(dead_code)]
                    const KNOWN_TAGS: &[&str] = &[$($crate::__finfmt_wire_args! { {__finfmt_wire_ber @known} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*];
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_tlv @encode value, output, scratch, KNOWN_TAGS, ($tagfmt);} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok(())
                }
            }

            impl<$de> $crate::composite::FieldDecode<$de, $name $(<$lt>)?> for $name $(<$lt>)? {
                // Entries are read until the input ends.
                const TAKES_REST: bool = true;

                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn decode_field(
                    input: &mut &$de [u8],
                    scratch: &mut &$de mut [u8],
                ) -> Result<Self, $crate::CompositeError> {
                    let _ = (&input, &scratch);
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @init} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    while !input.is_empty() {
                        let mut tag = [0; $crate::composite::MAX_TLV_TAG];
                        let tag = $crate::composite::decode_tlv_tag::<$tagfmt>(input, &mut **scratch, &mut tag)?;
                        let mut matched = false;
                        $($crate::__finfmt_wire_args! { {__finfmt_wire_tlv @match tag, input, scratch, matched;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                        $($crate::__finfmt_wire_args! { {__finfmt_wire_tlv @unknown tag, input, scratch, matched;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                        if !matched {
                            $crate::__private::cold_path();
                            return Err($crate::CompositeError::from($crate::Error::Invalid));
                        }
                    }
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @finish} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok($name { $($field),* })
                }
            }
        };
    };
    ([ber_tlv $padding:tt] [$($kept:tt)*] $vis:vis $name:ident [$($lt:lifetime)?] [$de:lifetime $($unused:lifetime)?]
        [$({ [$($fkept:tt)*] ($fvis:vis) $field:ident $pos:tt ($ty:ty) $kind:ident ($default:ty) [$($args:tt)*] })*]) => {
        $crate::__finfmt_wire_no_cfg! { $($kept)* $($($fkept)*)* }
        $($kept)*
        $vis struct $name $(<$lt>)? {
            $($($fkept)* $fvis $field: $ty,)*
        }

        const _: () = if let Err(message) = $crate::composite::check_ber_tags(&[$($crate::__finfmt_wire_args! { {__finfmt_wire_ber @tag} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*]) {
            panic!("{}", message);
        };

        const _: () = {
            // Generated constants for fixed and absent fields, and the format impls.
            $crate::__finfmt_wire_consts! { @all ber_tlv $name [$($lt)?] [$({ $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*] }

            impl $(<$lt>)? $crate::composite::FieldEncode<$name $(<$lt>)?> for $name $(<$lt>)? {
                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &Self) -> Result<(), $crate::CompositeError> {
                    let _ = (&output, &scratch, value);
                    // Extras must not use a declared tag, even one whose field is absent.
                    #[allow(dead_code)]
                    const KNOWN_TAGS: &[&str] = &[$($crate::__finfmt_wire_args! { {__finfmt_wire_ber @known} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* }),*];
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @encode value, output, scratch, KNOWN_TAGS;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok(())
                }
            }

            impl<$de> $crate::composite::FieldDecode<$de, $name $(<$lt>)?> for $name $(<$lt>)? {
                // Entries are read until the input ends.
                const TAKES_REST: bool = true;

                #[inline(always)]
                #[allow(unused_assignments, unused_mut, unused_variables)]
                fn decode_field(
                    input: &mut &$de [u8],
                    scratch: &mut &$de mut [u8],
                ) -> Result<Self, $crate::CompositeError> {
                    let _ = (&input, &scratch);
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @init} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    while let Some(entry) =
                        $crate::composite::decode_ber_tlv_collection_entry::<$padding>(input).map_err($crate::CompositeError::from)?
                    {
                        let mut value_input = entry.value;
                        let mut matched = false;
                        let mut tag_hex = [0; $crate::primitive::bertlv::MAX_BER_TAG_HEX];
                        let tag_hex = $crate::primitive::bertlv::format_ber_tag_hex(&mut tag_hex, entry.tag);
                        $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @match tag_hex, value_input, scratch, matched;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                        $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @unknown entry, value_input, scratch, matched;} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                        if !matched {
                            $crate::__private::cold_path();
                            return Err($crate::CompositeError::from($crate::Error::Invalid));
                        }
                    }
                    $($crate::__finfmt_wire_args! { {__finfmt_wire_ber @finish} $kind $field $pos ($default); [] [] [] [] [] [] []; $($args)* })*
                    Ok($name { $($field),* })
                }
            }
        };
    };
}

/// Parse a field's `#[wire]` arguments, in any order, into its final kind,
/// format, bit, tag, extras flag, selector and generated constant, then call
/// `$target!` with one phase of its layout's code. The slots are
/// `[fmt] [bit] [tag] [extras] [fixed] [absent] [select]`.
///
/// A fixed or absent field's constant is implemented on the record, keyed by
/// the field's position `$pos`, and its format names it through `Self`. An
/// absent form makes an `Option<T>` field required by its container, with
/// `OptionAs` deciding presence from the bytes.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_args {
    // An `extras` collection.
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [] [] [] [extras] [] [] [];) => {
        $crate::$target! { $($phase)* $kind $field (()) [] [] [extras] [] [] }
    };
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [$fmt:ty] [] [] [extras] [] [] [];) => {
        $crate::$target! { $($phase)* $kind $field (()) [] [] [extras ($fmt)] [] [] }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [extras] $fixed:tt $absent:tt $select:tt;) => {
        compile_error!("wire_type!: an `extras` field takes no `bit`, `tag`, `select`, fixed value or absent form")
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [] $fixed:tt $absent:tt [$($select:tt)+];) => {
        $crate::__finfmt_wire_args! { @select $phase $kind $field $pos $default; $fmt $bit $tag [] $fixed $absent [$($select)+]; }
    };
    // Fixed fields.
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [] [$($fixed:tt)+] [$($absent:tt)+] [];) => {
        compile_error!("wire_type!: a fixed field has no absent form")
    };
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [value $value:expr] [] [];) => {
        $crate::$target! {
            $($phase)* $kind $field ($crate::FixedValue<$fmt, $crate::__private::RecordBytes<Self, { $pos }>>) $bit $tag [] []
            [$pos fixed_value $value]
        }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; [] $bit:tt $tag:tt [] [value $value:expr] [] [];) => {
        compile_error!("wire_type!: `fixed_value` needs a `fmt` to encode it")
    };
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [] $bit:tt $tag:tt [] [bytes $value:expr] [] [];) => {
        $crate::$target! {
            $($phase)* $kind $field ($crate::FixedBytes<$crate::__private::RecordBytes<Self, { $pos }>>) $bit $tag [] []
            [$pos bytes $value]
        }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; [$fmt:ty] $bit:tt $tag:tt [] [bytes $value:expr] [] [];) => {
        compile_error!("wire_type!: `fixed_bytes` takes no `fmt`")
    };
    // Absent forms: `Option<T>` through `OptionAs`, with the field's format
    // (or `T`) for present values.
    ($phase:tt req $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [] [] [$($absent:tt)+] [];) => {
        compile_error!("wire_type!: an absent form needs an `Option` field")
    };
    ($phase:tt opt $field:ident $pos:tt ($default:ty); [] $bit:tt $tag:tt [] [] [$($absent:tt)+] [];) => {
        $crate::__finfmt_wire_args! { $phase opt $field $pos ($default); [$default] $bit $tag [] [] [$($absent)+] []; }
    };
    ({$target:ident $($phase:tt)*} opt $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [] [type $absent:ty] [];) => {
        $crate::$target! { $($phase)* req $field ($crate::OptionAs<$fmt, $absent>) $bit $tag [] [] [] }
    };
    ({$target:ident $($phase:tt)*} opt $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [] [value $value:expr] [];) => {
        $crate::$target! {
            $($phase)* req $field ($crate::OptionAs<$fmt, $crate::__private::RecordAbsent<Self, { $pos }>>) $bit $tag [] []
            [$pos absent_value ($fmt) $value]
        }
    };
    ({$target:ident $($phase:tt)*} opt $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [] [bytes $value:expr] [];) => {
        $crate::$target! {
            $($phase)* req $field ($crate::OptionAs<$fmt, $crate::AbsentBytes<$crate::__private::RecordBytes<Self, { $pos }>>>) $bit $tag
            [] [] [$pos bytes $value]
        }
    };
    // Plain fields.
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [] [] [];) => {
        $crate::$target! { $($phase)* $kind $field ($fmt) $bit $tag [] [] [] }
    };
    ({$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [] $bit:tt $tag:tt [] [] [] [];) => {
        $crate::$target! { $($phase)* $kind $field ($default) $bit $tag [] [] [] }
    };
    // A field decoded by a selected enum, keyed by an earlier field.
    (@select {$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [$fmt:ty] $bit:tt $tag:tt [] [] [] [$($select:tt)+];) => {
        $crate::$target! { $($phase)* $kind $field ($fmt) $bit $tag [] [$($select)+] [] }
    };
    (@select {$target:ident $($phase:tt)*} $kind:ident $field:ident $pos:tt ($default:ty); [] $bit:tt $tag:tt [] [] [] [$($select:tt)+];) => {
        $crate::$target! { $($phase)* $kind $field ($default) $bit $tag [] [$($select)+] [] }
    };
    (@select $phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [] $fixed:tt $absent:tt [$($select:tt)+];) => {
        compile_error!("wire_type!: a `select` field has no fixed value or absent form")
    };
    // One argument at a time, each at most once.
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; [] $bit:tt $tag:tt $extras:tt $fixed:tt $absent:tt $select:tt;
        fmt = $fmt:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; [$fmt] $bit $tag $extras $fixed $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt [] $tag:tt $extras:tt $fixed:tt $absent:tt $select:tt;
        bit = $bit:literal $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt [$bit] $tag $extras $fixed $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt [] $extras:tt $fixed:tt $absent:tt $select:tt;
        tag = $tag:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit [$tag] $extras $fixed $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt [] $fixed:tt $absent:tt $select:tt;
        extras $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag [extras] $fixed $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt [] $absent:tt $select:tt;
        fixed_value = $value:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras [value $value] $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt [] $absent:tt $select:tt;
        fixed_bytes = $value:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras [bytes $value] $absent $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt [] $select:tt;
        absent_bytes = $value:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras $fixed [bytes $value] $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt [] $select:tt;
        absent_value = $value:expr $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras $fixed [value $value] $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt [] $select:tt;
        absent = $absent:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras $fixed [type $absent] $select; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt $absent:tt [];
        select = $($func:ident)::+ ($($arg:ident),+ $(,)?) $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! {
            $phase $kind $field $pos $default; $fmt $bit $tag $extras $fixed $absent [($($func)::+) ($($arg),+)]; $($($rest)*)?
        }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt $absent:tt [];
        select = $select:ident $(, $($rest:tt)*)?) => {
        $crate::__finfmt_wire_args! { $phase $kind $field $pos $default; $fmt $bit $tag $extras $fixed $absent [$select]; $($($rest)*)? }
    };
    ($phase:tt $kind:ident $field:ident $pos:tt $default:tt; $fmt:tt $bit:tt $tag:tt $extras:tt $fixed:tt $absent:tt $select:tt; $($args:tt)+) => {
        compile_error!(concat!("wire_type!: unsupported or repeated #[wire] field arguments: ", stringify!($($args)+)))
    };
}

/// Per field and layout: check that the arguments belong to the layout, and
/// implement the constant a fixed or absent field needs on the record, keyed
/// by the field's position, so no generated name meets the user's paths.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_consts {
    (@optional opt $($rest:tt)*) => { true };
    (@optional req $($rest:tt)*) => { false };

    // Whether the field may take the rest of the input, as its format's
    // `TAKES_REST`. A concat field the record may omit ends the record only
    // at the end of the input, so it takes the rest.
    (@rest $layout:ident $de:lifetime $ty:tt $inner:tt $kind:ident $field:ident $fmt:tt $bit:tt $tag:tt [extras $($x:tt)*] $($slots:tt)*) => {
        false
    };
    (@rest concat $de:lifetime $ty:tt $inner:tt opt $($rest:tt)*) => { true };
    (@rest $layout:ident $de:lifetime $ty:tt ($inner:ty) opt $field:ident ($fmt:ty) $bit:tt $tag:tt [] [$($select:tt)+] $items:tt) => {
        <$fmt as $crate::composite::ContextDecode<$de, $inner, str>>::TAKES_REST
    };
    (@rest $layout:ident $de:lifetime $ty:tt ($inner:ty) opt $field:ident ($fmt:ty) $($slots:tt)*) => {
        <$fmt as $crate::composite::FieldDecode<$de, $inner>>::TAKES_REST
    };
    (@rest $layout:ident $de:lifetime ($ty:ty) $inner:tt req $field:ident ($fmt:ty) $bit:tt $tag:tt [] [$($select:tt)+] $items:tt) => {
        <$fmt as $crate::composite::ContextDecode<$de, $ty, str>>::TAKES_REST
    };
    (@rest $layout:ident $de:lifetime ($ty:ty) $inner:tt req $field:ident ($fmt:ty) $($slots:tt)*) => {
        <$fmt as $crate::composite::FieldDecode<$de, $ty>>::TAKES_REST
    };

    (@all $layout:ident $name:ident $lt:tt [$({ $($field:tt)* })*]) => {
        $($crate::__finfmt_wire_args! { {__finfmt_wire_consts @consts $layout $name $lt} $($field)* })*
    };
    (@consts $layout:ident $name:ident $lt:tt $kind:ident $field:ident ($fmt:ty) $bit:tt $tag:tt $extras:tt $select:tt $item:tt) => {
        $crate::__finfmt_wire_consts! { @check $layout $kind $bit $tag $extras }
        $crate::__finfmt_wire_consts! { @check_select $layout $kind $select }
        $crate::__finfmt_wire_consts! { @item $name $lt $item }
    };

    (@check_select $layout:ident $kind:ident []) => {};
    (@check_select ber_tlv $kind:ident [$($select:tt)+]) => {
        compile_error!("wire_type!: `select` is not for BER-TLV records, whose entries arrive in any order");
    };
    (@check_select tlv $kind:ident [$($select:tt)+]) => {
        compile_error!("wire_type!: `select` is not for TLV records, whose entries arrive in any order");
    };
    (@check_select $layout:ident $kind:ident [$($select:tt)+]) => {};

    (@check concat $kind:ident [] [] []) => {};
    (@check delimited $kind:ident [] [] []) => {};
    (@check bitmap $kind:ident $bit:tt [] []) => {};
    (@check ber_tlv $kind:ident [] [$($tag:tt)+] []) => {};
    (@check ber_tlv req [] [] [extras]) => {};
    (@check ber_tlv $kind:ident [] [] [extras ($fmt:ty)]) => {
        compile_error!("wire_type!: a BER-TLV `extras` field takes no `fmt`; its values are hex");
    };
    (@check ber_tlv opt [] [] [extras]) => {
        compile_error!("wire_type!: an `extras` field cannot be an Option");
    };
    (@check ber_tlv $kind:ident [] [] []) => {
        compile_error!("wire_type!: a BER-TLV field needs a `tag`, or `extras` for the unknown-tag collection");
    };
    (@check tlv $kind:ident [] [$($tag:tt)+] []) => {};
    (@check tlv req [] [] [extras ($fmt:ty)]) => {};
    (@check tlv $kind:ident [] [] [extras]) => {
        compile_error!("wire_type!: a TLV `extras` field needs `fmt`, the format of unknown tags' values");
    };
    (@check tlv opt [] [] [extras $($fmt:tt)*]) => {
        compile_error!("wire_type!: an `extras` field cannot be an Option");
    };
    (@check tlv $kind:ident [] [] []) => {
        compile_error!("wire_type!: a TLV field needs a `tag`, or `extras` for the unknown-tag collection");
    };
    (@check $layout:ident $kind:ident [$($bit:tt)+] $tag:tt $extras:tt) => {
        compile_error!("wire_type!: `bit` is for bitmap records");
    };
    (@check $layout:ident $kind:ident $bit:tt [$($tag:tt)+] $extras:tt) => {
        compile_error!("wire_type!: `tag` is for BER-TLV and TLV records");
    };
    (@check $layout:ident $kind:ident $bit:tt $tag:tt [extras $($fmt:tt)*]) => {
        compile_error!("wire_type!: `extras` is for BER-TLV and TLV records");
    };

    (@item $name:ident $lt:tt []) => {};
    (@item $name:ident [$($lt:lifetime)?] [$pos:tt fixed_value $value:expr]) => {
        impl $(<$lt>)? $crate::__private::FieldBytes<{ $pos }> for $name $(<$lt>)? {
            const BYTES: &'static [u8] = {
                let text: &str = $value;
                text.as_bytes()
            };
        }
    };
    (@item $name:ident [$($lt:lifetime)?] [$pos:tt bytes $value:expr]) => {
        impl $(<$lt>)? $crate::__private::FieldBytes<{ $pos }> for $name $(<$lt>)? {
            #[allow(unused_parens)]
            const BYTES: &'static [u8] = $crate::__private::BytePattern($value).bytes();
        }
    };
    (@item $name:ident [$($lt:lifetime)?] [$pos:tt absent_value ($fmt:ty) $value:expr]) => {
        impl $(<$lt>)? $crate::__private::FieldAbsent<{ $pos }> for $name $(<$lt>)? {
            #[inline(always)]
            fn encode_absent(output: &mut &mut [u8], scratch: &mut [u8]) -> Result<(), $crate::Error> {
                <$fmt as $crate::composite::FieldEncode<_>>::encode_field(output, scratch, &$value).map_err(|error| error.kind)
            }
        }
    };
}

/// The key text a `select` field passes to its selected enum: an earlier
/// field's text, or what a function of earlier fields returns.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_key {
    ($select:ident) => {
        ::core::convert::AsRef::<str>::as_ref(&$select)
    };
    // Each argument is tied to its field's type first, so the function's
    // parameter types do not decide how the field decodes.
    (($($func:ident)::+) ($($arg:ident),+)) => {
        ::core::convert::AsRef::<str>::as_ref(&{
            $($crate::__private::same_type(&$arg, |record: &Self| &record.$arg);)+
            $($func)::+($(&$arg),+)
        })
    };
}

/// One concat field: written in order; `Option` fields form the tail.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_concat {
    (@encode $value:ident, $output:ident, $scratch:ident, $omitted:ident; req $field:ident ($fmt:ty) $($slots:tt)*) => {
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $omitted:ident; opt $field:ident ($fmt:ty) $($slots:tt)*) => {
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
    (@decode $input:ident, $scratch:ident; req $field:ident ($fmt:ty) $bit:tt $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let $field = <$fmt as $crate::composite::ContextDecode<'_, _, str>>::decode_with($input, $scratch, $crate::__finfmt_wire_key!($($select)+))
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@decode $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) $bit:tt $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$fmt as $crate::composite::ContextDecode<'_, _, str>>::decode_with($input, $scratch, $crate::__finfmt_wire_key!($($select)+))
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
    };
    (@decode $input:ident, $scratch:ident; req $field:ident ($fmt:ty) $($slots:tt)*) => {
        let $field = <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@decode $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) $($slots:tt)*) => {
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

/// One delimited field, then the separator unless it is the last. Only the
/// last field may contain the separator.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_delimited {
    (@encode $value:ident, $output:ident, $scratch:ident, $separator:ident, $more:expr; req $field:ident ($fmt:ty) $($slots:tt)*) => {
        $crate::__finfmt_wire_delimited! { @value &$value.$field, $output, $scratch, $separator, $more; $field ($fmt) }
        $crate::__finfmt_wire_delimited! { @next $output, $separator, $more }
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $separator:ident, $more:expr; opt $field:ident ($fmt:ty) $($slots:tt)*) => {
        if let Some(inner) = $value.$field.as_ref() {
            $crate::__finfmt_wire_delimited! { @value inner, $output, $scratch, $separator, $more; $field ($fmt) }
        }
        $crate::__finfmt_wire_delimited! { @next $output, $separator, $more }
    };
    (@value $value:expr, $output:ident, $scratch:ident, $separator:ident, $more:expr; $field:ident ($fmt:ty)) => {
        $crate::composite::encode_delimited_value::<_, $fmt>($output, $scratch, $value, if $more { Some($separator) } else { None })
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@next $output:ident, $separator:ident, $more:expr) => {
        if $more {
            $crate::composite::encode_delimiter($output, $separator)?;
        }
    };
    (@decode $input:ident, $scratch:ident, $separator:ident, $more:expr; req $field:ident ($fmt:ty) $bit:tt $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let segment = $crate::composite::decode_delimited_field($input, $separator, $more)?;
        let $field = $crate::composite::decode_delimited_context::<_, str, $fmt>(segment, $scratch, $crate::__finfmt_wire_key!($($select)+))
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@decode $input:ident, $scratch:ident, $separator:ident, $more:expr; opt $field:ident ($fmt:ty) $bit:tt $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let segment = $crate::composite::decode_delimited_field($input, $separator, $more)?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_context::<_, str, $fmt>(segment, $scratch, $crate::__finfmt_wire_key!($($select)+))
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
    };
    (@decode $input:ident, $scratch:ident, $separator:ident, $more:expr; req $field:ident ($fmt:ty) $($slots:tt)*) => {
        let segment = $crate::composite::decode_delimited_field($input, $separator, $more)?;
        let $field = $crate::composite::decode_delimited_value::<_, $fmt>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@decode $input:ident, $scratch:ident, $separator:ident, $more:expr; opt $field:ident ($fmt:ty) $($slots:tt)*) => {
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

/// One phase of a bitmap record for one field. Fields without a bit are the
/// header.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_bitmap {
    (@bit $kind:ident $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        Some($bit)
    };
    (@bit $kind:ident $field:ident ($fmt:ty) [] $($slots:tt)*) => {
        None
    };

    (@head_encode $value:ident, $output:ident, $scratch:ident; req $field:ident ($fmt:ty) [] $($slots:tt)*) => {
        $crate::__finfmt_wire_concat! { @encode $value, $output, $scratch, unused; req $field ($fmt) }
    };
    (@head_encode $($rest:tt)*) => {};

    (@set $bitmap:ident, $value:ident; req $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        $bitmap.set($bit, true);
    };
    (@set $bitmap:ident, $value:ident; opt $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        if $value.$field.is_some() {
            $bitmap.set($bit, true);
        }
    };
    (@set $($rest:tt)*) => {};

    (@body_encode $value:ident, $output:ident, $scratch:ident; req $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@body_encode $value:ident, $output:ident, $scratch:ident; opt $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        if let Some(inner) = $value.$field.as_ref() {
            <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        }
    };
    (@body_encode $($rest:tt)*) => {};

    (@head_decode $input:ident, $scratch:ident; req $field:ident ($fmt:ty) [] $tag:tt $extras:tt $select:tt $items:tt) => {
        $crate::__finfmt_wire_concat! { @decode $input, $scratch; req $field ($fmt) [] $tag $extras $select $items }
    };
    // Rejected by the record's compile-time check; bound to keep errors quiet.
    (@head_decode $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) [] $($slots:tt)*) => {
        let $field = None;
    };
    (@head_decode $($rest:tt)*) => {};

    (@clear $unknown:ident; $kind:ident $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        $unknown.set($bit, false);
    };
    (@clear $($rest:tt)*) => {};

    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) [$bit:literal] $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let $field = if $bitmap.get($bit) {
            Some(
                <$fmt as $crate::composite::ContextDecode<'_, _, str>>::decode_with($input, $scratch, $crate::__finfmt_wire_key!($($select)+))
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
    };
    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; req $field:ident ($fmt:ty) [$bit:literal] $tag:tt $extras:tt [$($select:tt)+] $items:tt) => {
        let $field = if $bitmap.get($bit) {
            <$fmt as $crate::composite::ContextDecode<'_, _, str>>::decode_with($input, $scratch, $crate::__finfmt_wire_key!($($select)+))
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
    };
    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; req $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
        let $field = if $bitmap.get($bit) {
            <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
    };
    (@body_decode $bitmap:ident, $input:ident, $scratch:ident; opt $field:ident ($fmt:ty) [$bit:literal] $($slots:tt)*) => {
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

/// One phase of a BER-TLV record for one field: a tagged field, or the
/// `extras` collection.
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_ber {
    (@tag $kind:ident $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        Some($tag)
    };
    (@tag $kind:ident $field:ident ($fmt:ty) $bit:tt [] $($slots:tt)*) => {
        None
    };
    (@known $kind:ident $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        $tag
    };
    // No unknown tag is empty, so the extras slot matches nothing.
    (@known $kind:ident $field:ident ($fmt:ty) $bit:tt [] $($slots:tt)*) => {
        ""
    };

    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; req $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            <$fmt as $crate::composite::FieldEncode<_>>::encode_field(value_out, scratch, &$value.$field)
        })?;
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; opt $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$fmt as $crate::composite::FieldEncode<_>>::encode_field(value_out, scratch, inner)
            })?;
        }
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident; req $field:ident ($fmt:ty) $bit:tt [] [extras] $($slots:tt)*) => {
        $crate::composite::BerTlvExtras::encode_unknowns(&$value.$field, $output, $scratch, $known)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@encode $($rest:tt)*) => {};

    (@init $kind:ident $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        let mut $field = None;
    };
    (@init $kind:ident $field:ident ($fmt:ty) $bit:tt [] $($slots:tt)*) => {
        let mut $field = ::core::default::Default::default();
    };

    (@match $tag_hex:ident, $value_input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
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

    (@unknown $entry:ident, $value_input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident ($fmt:ty) $bit:tt [] [extras] $($slots:tt)*) => {
        if !$matched {
            $crate::composite::BerTlvExtras::decode_unknown(&mut $field, $entry.tag, $value_input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
            $matched = true;
        }
    };
    (@unknown $($rest:tt)*) => {};

    (@finish req $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
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

/// One phase of a generic TLV record for one field: a tagged field, or the
/// `extras` collection. Tags, declarations and finishing are shared with
/// [`__finfmt_wire_ber`].
#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_wire_tlv {
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident, ($tagfmt:ty); req $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        $crate::composite::encode_tlv_tag::<$tagfmt>($output, $scratch, $tag, stringify!($field))?;
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident, ($tagfmt:ty); opt $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_tlv_tag::<$tagfmt>($output, $scratch, $tag, stringify!($field))?;
            <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        }
    };
    (@encode $value:ident, $output:ident, $scratch:ident, $known:ident, ($tagfmt:ty); req $field:ident ($fmt:ty) $bit:tt [] [extras ($vfmt:ty)] $($slots:tt)*) => {
        $crate::composite::TlvExtras::encode_unknowns::<$tagfmt, $vfmt>(&$value.$field, $output, $scratch, $known)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
    };
    (@encode $($rest:tt)*) => {};

    (@match $tag_value:ident, $input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident ($fmt:ty) $bit:tt [$tag:expr] $($slots:tt)*) => {
        if !$matched && $tag_value == <str>::as_bytes($tag) {
            $matched = true;
            $crate::composite::decode_tlv_field($input, $scratch, &mut $field, stringify!($field), |input, scratch| {
                <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field(input, scratch)
            })?;
        }
    };
    (@match $($rest:tt)*) => {};

    (@unknown $tag_value:ident, $input:ident, $scratch:ident, $matched:ident; $kind:ident $field:ident ($fmt:ty) $bit:tt [] [extras ($vfmt:ty)] $($slots:tt)*) => {
        if !$matched {
            $crate::composite::TlvExtras::decode_unknown::<$vfmt>(&mut $field, $tag_value, $input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
            $matched = true;
        }
    };
    (@unknown $($rest:tt)*) => {};
}

// These tests use serde derives and `SerdeScalar` throughout.
#[cfg(all(test, feature = "serde"))]
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
    fn concat_records_roundtrip_with_optional_tails() {
        for (first, second, wire) in [
            (None, None, &b"0100073ABC"[..]),
            (Some(15), None, b"0100073ABC15"),
            (Some(15), Some("42"), b"0100073ABC1542"),
        ] {
            let value = record(first, second);
            assert_eq!(encode::<Record, _>(&value).as_deref(), Ok(wire));
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
        assert_eq!(bad.kind, Error::Invalid);
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

    #[test]
    fn delimited_records_roundtrip_and_reject_bad_segments() {
        for (middle, last, wire) in [(None, None, &b"AB||07|"[..]), (Some(15), Some("X|Y"), b"AB|15|07|X|Y")] {
            let value = Delimited {
                first: "AB",
                middle,
                inner: Inner { code: "07".into() },
                last: last.map(Into::into),
            };
            assert_eq!(encode::<Delimited, _>(&value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<Delimited, Delimited>(wire, &mut scratch), Ok(value));
        }
        // A missing separator is the record's error; a bad segment is its field's.
        for (wire, path) in [
            (&b"AB|1"[..], &[][..]),
            (b"AB|15|7X|", &[PathSegment::Field("inner"), PathSegment::Field("code")]),
            (b"AB|15|07", &[]),
        ] {
            let mut scratch = [0; 64];
            let error = crate::decode::<Delimited, Delimited>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, path));
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

    #[test]
    fn bitmap_records_roundtrip_and_reject_unknown_or_missing_fields() {
        for (pan, code) in [(None, None), (Some("4111"), Some(42))] {
            let value = Message {
                mti: "0100",
                pan: pan.map(Into::into),
                inner: Inner { code: "07".into() },
                code,
            };
            let wire = encode::<Message, _>(&value).unwrap();
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<Message, Message>(&wire, &mut scratch), Ok(value));
        }
        // Unknown and missing fields are rejected alike.
        // Field 4 is unknown; field 3 is required.
        for (wire, path) in [
            (&b"0100100000000000000007"[..], &[][..]),
            (b"010040000000000000000", &[PathSegment::Field("inner")]),
        ] {
            let mut scratch = [0; 64];
            let error = crate::decode::<Message, Message>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, path));
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

    #[test]
    fn ber_tlv_records_roundtrip_extras_and_padding() {
        let mut extras = std::collections::BTreeMap::new();
        extras.insert("t9F03_unknown".to_owned(), "00".to_owned());
        let value = Emv {
            amount: "000000012345".into(),
            extras,
            currency: Some("0978".into()),
        };
        // Extras are written where the record declares them, and read in any order.
        let wire = encode::<Emv, _>(&value).unwrap();
        assert_eq!(wire, b"\x9F\x02\x06\x00\x00\x00\x01\x23\x45\x9F\x03\x01\x00\x5F\x2A\x02\x09\x78");
        let mut scratch = [0; 128];
        assert_eq!(crate::decode::<Emv, Emv>(&wire, &mut scratch), Ok(value.clone()));
        let extras_last = b"\x9F\x02\x06\x00\x00\x00\x01\x23\x45\x5F\x2A\x02\x09\x78\x9F\x03\x01\x00";
        let mut scratch = [0; 128];
        assert_eq!(crate::decode::<Emv, Emv>(extras_last, &mut scratch), Ok(value));
        // A repeated known tag, a missing required one, and an extras entry
        // claiming a declared tag are rejected.
        for wire in [&b"\x9F\x02\x01\x00\x9F\x02\x01\x00"[..], b"\x5F\x2A\x01\x00"] {
            let mut scratch = [0; 128];
            let error = crate::decode::<Emv, Emv>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field("amount")][..]));
        }
        let mut claimed = std::collections::BTreeMap::new();
        claimed.insert("t5F2A_unknown".to_owned(), "00".to_owned());
        let value = Emv {
            amount: "00".into(),
            extras: claimed,
            currency: None,
        };
        let error = encode::<Emv, _>(&value).unwrap_err();
        assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field("extras")][..]));

        // Without extras, unknown tags are rejected; padding is allowed when asked for.
        let mut scratch = [0; 128];
        assert_eq!(
            crate::decode::<Strict, Strict>(b"\x00\x9F\x02\x02AB\x00\x9F\x36\x01\x07\x00", &mut scratch),
            Ok(Strict {
                amount: "AB",
                counter: Some("07".into())
            })
        );
        let mut scratch = [0; 128];
        let error = crate::decode::<Strict, Strict>(b"\x9F\x02\x02AB\x9F\x03\x01\x00", &mut scratch).unwrap_err();
        assert_eq!((error.kind, error.path()), (Error::Invalid, &[][..]));
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

    type A1 = Field<Ascii<1, 1>, Fixed<1>>;
    type Padded = Field<Ascii<0, 4>, Fixed<4>, crate::PadRight<4>>;
    const FILLER: [u8; 3] = [b'.'; 3];

    crate::wire_type! {
        #[derive(Debug, Default, PartialEq)]
        #[wire(concat)]
        struct Constants {
            #[wire(fmt = A1, fixed_value = "H")]
            record_type: (),
            #[wire(fmt = N2)]
            code: String,
            #[wire(fixed_bytes = [b' '; 2])]
            gap: (),
            #[wire(fixed_value = "AB", fmt = Padded)]
            padded: (),
            #[wire(fixed_bytes = FILLER)]
            filler: (),
            #[wire(fixed_bytes = b"!")]
            end: Option<()>,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(delimited = b'|')]
        struct FixedDelimited {
            #[wire(fmt = A1, fixed_value = "D")]
            kind: (),
            #[wire(fmt = N2)]
            code: String,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(bitmap = HexBitmap)]
        struct FixedBitmap {
            #[wire(fmt = N4, fixed_value = "0800")]
            mti: (),
            #[wire(bit = 70, fmt = N2, fixed_value = "01")]
            network: Option<()>,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(ber_tlv)]
        struct FixedBer {
            #[wire(tag = "9F35", fixed_bytes = b"\x22")]
            terminal_type: (),
            #[wire(tag = "9F02", fmt = Hex4)]
            amount: String,
        }
    }

    #[test]
    fn fixed_fields_write_and_check_constants() {
        let value = Constants {
            code: "42".into(),
            end: Some(()),
            ..Default::default()
        };
        assert_eq!(encode::<Constants, _>(&value).as_deref(), Ok(&b"H42  AB  ...!"[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Constants, Constants>(b"H42  AB  ...!", &mut scratch), Ok(value));
        // A value is compared after decoding, so its format's leniency applies;
        // bytes are compared exactly. The decoded constant keeps no scratch.
        let mut scratch = [0; 64];
        let mut input = &b"H07  AB  ..."[..];
        let mut arena = &mut scratch[..];
        assert!(<Constants as crate::FieldDecode<'_, Constants>>::decode_field(&mut input, &mut arena).is_ok());
        assert_eq!(arena.len(), 64);
        for (wire, field) in [
            (&b"X42  AB  ..."[..], "record_type"),
            (b"H42 xAB  ...", "gap"),
            (b"H42  AC  ...", "padded"),
            (b"H42  AB  .:.", "filler"),
        ] {
            let mut scratch = [0; 64];
            let error = crate::decode::<Constants, Constants>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field(field)][..]));
        }

        let value = FixedDelimited {
            code: "42".into(),
            ..Default::default()
        };
        assert_eq!(encode::<FixedDelimited, _>(&value).as_deref(), Ok(&b"D|42"[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<FixedDelimited, FixedDelimited>(b"D|42", &mut scratch), Ok(value));

        for network in [None, Some(())] {
            let value = FixedBitmap {
                network,
                ..Default::default()
            };
            let wire = encode::<FixedBitmap, _>(&value).unwrap();
            assert!(wire.starts_with(b"0800"));
            assert_eq!(wire.ends_with(b"01"), network.is_some());
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<FixedBitmap, FixedBitmap>(&wire, &mut scratch), Ok(value));
        }

        let value = FixedBer {
            amount: "0100".into(),
            ..Default::default()
        };
        let wire = encode::<FixedBer, _>(&value).unwrap();
        assert_eq!(wire, b"\x9F\x35\x01\x22\x9F\x02\x02\x01\x00");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<FixedBer, FixedBer>(&wire, &mut scratch), Ok(value));
        let mut scratch = [0; 64];
        let error = crate::decode::<FixedBer, FixedBer>(b"\x9F\x35\x01\x21\x9F\x02\x02\x01\x00", &mut scratch).unwrap_err();
        assert_eq!(error.path(), &[PathSegment::Field("terminal_type")]);
    }

    mod code {
        pub(super) type F = super::A1;
    }
    mod flag {
        pub(super) type F = super::A1;
    }
    const MARK: &[u8; 2] = b"HD";
    const MARK_SLICE: &[u8] = b"HD";
    const MARK_ARRAY: [u8; 2] = *b"HD";
    type A2 = Field<Ascii<2, 2>, Fixed<2>>;

    crate::wire_type! {
        #[derive(Debug, Default, PartialEq)]
        #[wire(concat)]
        struct Spellings {
            #[wire(fixed_bytes = b"HD")]
            literal: (),
            #[wire(fixed_bytes = (b"HD"))]
            parenthesized: (),
            #[wire(fixed_bytes = MARK)]
            reference: (),
            #[wire(fixed_bytes = MARK_ARRAY)]
            array: (),
            #[wire(fixed_bytes = MARK_SLICE)]
            slice: (),
            #[wire(fixed_bytes = [0x48, 0x44])]
            list: (),
            #[wire(fmt = A2, absent_bytes = MARK)]
            absent: Option<String>,
        }

        /// Fields named like the modules their siblings' formats live in.
        #[derive(Debug, Default, PartialEq)]
        #[wire(concat)]
        struct Shadowing {
            #[wire(fmt = code::F, fixed_value = "H")]
            code: (),
            #[wire(fmt = flag::F, absent_bytes = b" ")]
            flag: Option<String>,
            #[wire(fmt = code::F, absent_value = "0")]
            sibling: Option<String>,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct Child<'a> {
            #[wire(fmt = A1)]
            code: &'a str,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct Parent<'a> {
            #[wire(absent_value = Child { code: "0" })]
            child: Option<Child<'a>>,
        }
    }

    crate::wire_type! {
        #[derive(Debug, Default, PartialEq)]
        #[wire(concat)]
        struct OptionPaths {
            #[wire(fmt = A1)]
            std: std::option::Option<String>,
            #[wire(fmt = A1)]
            core: ::core::option::Option<String>,
        }
    }

    #[test]
    fn option_paths_are_option_fields() {
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<OptionPaths, OptionPaths>(b"", &mut scratch),
            Ok(OptionPaths::default())
        );
        let some = OptionPaths {
            std: Some("A".into()),
            core: Some("B".into()),
        };
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<OptionPaths, OptionPaths>(b"AB", &mut scratch), Ok(some));
    }

    #[test]
    fn generated_constants_take_any_spelling_and_shadow_nothing() {
        let wire = encode::<Spellings, _>(&Spellings::default()).unwrap();
        assert_eq!(wire, b"HDHDHDHDHDHDHD");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Spellings, Spellings>(&wire, &mut scratch), Ok(Spellings::default()));

        let wire = encode::<Shadowing, _>(&Shadowing::default()).unwrap();
        assert_eq!(wire, b"H 0");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Shadowing, Shadowing>(&wire, &mut scratch), Ok(Shadowing::default()));

        // An absent value of a borrowed nested record.
        assert_eq!(encode::<Parent, _>(&Parent { child: None }).as_deref(), Ok(&b"0"[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Parent, Parent>(b"0", &mut scratch), Ok(Parent { child: None }));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<Parent, Parent>(b"1", &mut scratch),
            Ok(Parent {
                child: Some(Child { code: "1" })
            })
        );
    }

    type N6 = Field<Numeric<6, 6>, Fixed<6>>;
    type Amount = Field<Numeric<1, 6>, Fixed<6>, crate::PadLeft<6, b'0', 1>>;
    const BLANKS: [u8; 6] = [b' '; 6];

    crate::wire_type! {
        #[derive(Debug, Default, PartialEq)]
        #[wire(concat)]
        struct Absent {
            #[wire(fmt = Amount, absent_bytes = b"      ")]
            literal: Option<u64>,
            #[wire(fmt = Amount, absent_bytes = BLANKS)]
            constant: Option<u64>,
            #[wire(absent_value = 0u64, fmt = Amount)]
            zero: Option<u64>,
            #[wire(fmt = N6, absent_value = "999999")]
            text: Option<String>,
            #[wire(fmt = Amount, absent = crate::AbsentBytes<crate::Fill<b'*', 6>>)]
            custom: Option<u64>,
            #[wire(fmt = N2)]
            tail: Option<String>,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(delimited = b'|')]
        struct AbsentDelimited {
            #[wire(fmt = N2, absent_bytes = b"--")]
            first: Option<String>,
            #[wire(fmt = N2)]
            second: Option<String>,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(bitmap = HexBitmap)]
        struct AbsentBitmap {
            #[wire(bit = 3, fmt = N6, absent_bytes = BLANKS)]
            amount: Option<u64>,
        }

        #[derive(Debug, Default, PartialEq)]
        #[wire(ber_tlv)]
        struct AbsentBer {
            #[wire(tag = "9F02", fmt = Field<Ascii<0, 6>, crate::Rest>, absent_bytes = b"")]
            amount: Option<String>,
        }
    }

    #[test]
    fn absent_forms_map_option_onto_wire_patterns() {
        // Absent fields are always on the wire, so a later `Option` tail still works.
        let none = Absent::default();
        let wire = b"            000000999999******";
        assert_eq!(encode::<Absent, _>(&none).as_deref(), Ok(&wire[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Absent, Absent>(wire, &mut scratch), Ok(none));
        let some = Absent {
            literal: Some(1),
            constant: Some(2),
            zero: Some(3),
            text: Some("000004".into()),
            custom: Some(5),
            tail: Some("06".into()),
        };
        let wire = b"00000100000200000300000400000506";
        assert_eq!(encode::<Absent, _>(&some).as_deref(), Ok(&wire[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Absent, Absent>(wire, &mut scratch), Ok(some));
        // A present value that encodes like the pattern reads back as absent.
        let zero = Absent {
            zero: Some(0),
            ..Default::default()
        };
        let mut scratch = [0; 64];
        let wire = encode::<Absent, _>(&zero).unwrap();
        assert_eq!(
            crate::decode::<Absent, Absent>(&wire, &mut scratch).map(|value| value.zero),
            Ok(None)
        );
        // Anything else decodes through the field's format, whose errors are returned.
        let mut scratch = [0; 64];
        let error = crate::decode::<Absent, Absent>(b"  x         000000999999******", &mut scratch).unwrap_err();
        assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field("literal")][..]));
        // Same bytes as explicit `OptionAs` formats.
        for (literal, custom) in [(None, None), (Some(7), None), (None, Some(8))] {
            let new = Absent {
                literal,
                constant: literal,
                zero: literal,
                text: None,
                custom,
                tail: None,
            };
            let new_wire = encode::<Absent, _>(&new).unwrap();
            let literal_wire = encode::<crate::OptionAs<Amount, crate::AbsentBytes<crate::Fill<b' ', 6>>>, _>(&literal).unwrap();
            let custom_wire = encode::<crate::OptionAs<Amount, crate::AbsentBytes<crate::Fill<b'*', 6>>>, _>(&custom).unwrap();
            assert_eq!((&new_wire[..6], &new_wire[24..30]), (&literal_wire[..], &custom_wire[..]));
        }

        // In the other layouts the field is required by its container.
        let mut scratch = [0; 64];
        assert_eq!(
            encode::<AbsentDelimited, _>(&AbsentDelimited::default()).as_deref(),
            Ok(&b"--|"[..])
        );
        assert_eq!(
            crate::decode::<AbsentDelimited, AbsentDelimited>(b"--|", &mut scratch),
            Ok(AbsentDelimited::default())
        );
        let wire = encode::<AbsentBitmap, _>(&AbsentBitmap::default()).unwrap();
        assert_eq!(wire, b"2000000000000000      ");
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<AbsentBitmap, AbsentBitmap>(&wire, &mut scratch),
            Ok(AbsentBitmap::default())
        );
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<AbsentBitmap, AbsentBitmap>(b"0000000000000000", &mut scratch).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
        let wire = encode::<AbsentBer, _>(&AbsentBer::default()).unwrap();
        assert_eq!(wire, b"\x9F\x02\x00");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<AbsentBer, AbsentBer>(&wire, &mut scratch), Ok(AbsentBer::default()));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<AbsentBer, AbsentBer>(b"", &mut scratch).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
    }

    crate::wire_type! {
        #[derive(Debug, PartialEq)]
        #[wire(delimited = b'|')]
        struct Pair {
            #[wire(fmt = Text)]
            a: String,
            #[wire(fmt = Text)]
            b: String,
        }

        #[derive(Debug, PartialEq)]
        #[wire(ber_tlv)]
        struct Tags {
            #[wire(tag = "5A", fmt = Text)]
            a: Option<String>,
            #[wire(tag = "5B", fmt = Text)]
            b: Option<String>,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct EndsInRest {
            #[wire(fmt = A1)]
            code: String,
            #[wire(fmt = Text)]
            text: String,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct HoldsPair {
            #[wire(absent_bytes = b"A|B")]
            pair: Option<Pair>,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct HoldsTags {
            #[wire(absent_bytes = b"\x5A\x00")]
            tags: Option<Tags>,
        }

        #[derive(Debug, PartialEq)]
        #[wire(concat)]
        struct HoldsRest {
            #[wire(fmt = A1)]
            code: String,
            #[wire(absent_bytes = b"--")]
            last: Option<EndsInRest>,
        }
    }

    #[test]
    fn absent_patterns_match_all_of_a_record_taking_the_rest() {
        use crate::composite::{ContextDecode, FieldDecode};
        // Records and enums whose last part may take the rest say so.
        const {
            assert!(<Pair as FieldDecode<Pair>>::TAKES_REST);
            assert!(<Tags as FieldDecode<Tags>>::TAKES_REST);
            assert!(<EndsInRest as FieldDecode<EndsInRest>>::TAKES_REST);
            assert!(<HoldsRest as FieldDecode<HoldsRest>>::TAKES_REST);
            // An omitted concat tail ends only at the end of the input.
            assert!(<Record as FieldDecode<Record>>::TAKES_REST);
            assert!(!<Inner as FieldDecode<Inner>>::TAKES_REST);
            assert!(!<Codes as FieldDecode<Codes>>::TAKES_REST);
            assert!(!<AbsentBitmap as FieldDecode<AbsentBitmap>>::TAKES_REST);
            assert!(<Body as ContextDecode<Body, str>>::TAKES_REST);
            assert!(<crate::Frame<crate::Rest, Body> as ContextDecode<Body, str>>::TAKES_REST);
            assert!(!<crate::Frame<AsciiLength<1>, Body> as ContextDecode<Body, str>>::TAKES_REST);
        }
        // Only the whole remainder is absent; a longer value starting with
        // the pattern is present.
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsPair, HoldsPair>(b"A|B", &mut scratch),
            Ok(HoldsPair { pair: None })
        );
        let pair = Pair {
            a: "A".into(),
            b: "BX".into(),
        };
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsPair, HoldsPair>(b"A|BX", &mut scratch),
            Ok(HoldsPair { pair: Some(pair) })
        );
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsTags, HoldsTags>(b"\x5A\x00", &mut scratch),
            Ok(HoldsTags { tags: None })
        );
        let tags = Tags {
            a: Some("".into()),
            b: Some("X".into()),
        };
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsTags, HoldsTags>(b"\x5A\x00\x5B\x01X", &mut scratch),
            Ok(HoldsTags { tags: Some(tags) })
        );
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsRest, HoldsRest>(b"1--", &mut scratch),
            Ok(HoldsRest {
                code: "1".into(),
                last: None
            })
        );
        let last = EndsInRest {
            code: "-".into(),
            text: "-X".into(),
        };
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<HoldsRest, HoldsRest>(b"1--X", &mut scratch),
            Ok(HoldsRest {
                code: "1".into(),
                last: Some(last)
            })
        );
    }

    type N1 = Field<Numeric<1, 2>, Fixed<2>, crate::PadLeft<2, b'0', 1>>;
    type Ebcdic2 = Field<Ascii<2, 2>, Fixed<2>, crate::Ebcdic037>;
    type Rest = Field<Ascii<0, 20>, crate::Rest>;

    crate::wire_type! {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[wire(names)]
        enum Channel {
            #[wire(rename = "01")]
            Pos,
            /// Written as its identifier.
            Atm,
        }

        #[derive(Debug, Clone, Copy, PartialEq)]
        #[wire(codes)]
        enum Kind {
            #[wire(code = 0)]
            Purchase,
            #[wire(code = 20)]
            Refund,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(concat)]
        struct Codes {
            #[wire(fmt = N2)]
            channel: Channel,
            #[wire(fmt = Ebcdic2)]
            ebcdic: Channel,
            #[wire(fmt = Field<Ascii<3, 3>, Fixed<3>>)]
            named: Channel,
            #[wire(fmt = N1)]
            text: Kind,
            #[wire(fmt = crate::FixedBinaryBe<1>)]
            binary: Kind,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(concat)]
        struct MtiMessage<'a> {
            #[wire(fmt = N4)]
            mti: &'a str,
            #[wire(select = mti)]
            body: Body,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(selected)]
        enum Body {
            #[wire(rename = "0100")]
            Request(Inner),
            /// A payload with its own format.
            #[wire(rename = "0110")]
            Response(#[wire(fmt = N2)] String),
            #[wire(rename = "0800")]
            Echo,
            #[wire(other)]
            Unknown(#[wire(fmt = Rest)] String),
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(delimited = b'|')]
        struct DelimitedMessage {
            #[wire(fmt = N4)]
            mti: String,
            #[wire(select = mti)]
            body: Body,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(bitmap = HexBitmap)]
        struct BitmapMessage {
            #[wire(fmt = N4)]
            mti: String,
            #[wire(bit = 3, select = mti)]
            body: Body,
        }
    }

    #[test]
    fn unit_enums_map_names_and_codes() {
        let value = Codes {
            channel: Channel::Pos,
            ebcdic: Channel::Pos,
            named: Channel::Atm,
            text: Kind::Refund,
            binary: Kind::Refund,
        };
        let wire = encode::<Codes, _>(&value).unwrap();
        assert_eq!(wire, b"01\xF0\xF1Atm20\x14");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<Codes, Codes>(&wire, &mut scratch), Ok(value));
        // Text decoded into scratch for the comparison is not kept.
        let mut scratch = [0; 64];
        let mut arena = &mut scratch[..];
        let mut input = &wire[..];
        <Codes as crate::FieldDecode<'_, Codes>>::decode_field(&mut input, &mut arena).unwrap();
        assert_eq!(arena.len(), 64);
        // Unknown names and codes are invalid, in the field's path.
        for (wire, field) in [
            (&b"02\xF0\xF1Atm20\x14"[..], "channel"),
            (b"01\xF0\xF1Pos20\x14", "named"),
            (b"01\xF0\xF1Atm21\x14", "text"),
            (b"01\xF0\xF1Atm20\x15", "binary"),
        ] {
            let mut scratch = [0; 64];
            let error = crate::decode::<Codes, Codes>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field(field)][..]));
        }
    }

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(concat)]
        struct FramedMessage {
            #[wire(fmt = N4)]
            mti: String,
            #[wire(select = mti, fmt = crate::Frame<AsciiLength<2>, Body>)]
            body: Body,
        }
    }

    #[test]
    fn selected_bodies_pass_through_frames_and_name_their_key() {
        for (value, wire) in [
            (
                FramedMessage {
                    mti: "0100".into(),
                    body: Body::Request(Inner { code: "07".into() }),
                },
                &b"01000207"[..],
            ),
            (
                FramedMessage {
                    mti: "0420".into(),
                    body: Body::Unknown("ABC".into()),
                },
                b"042003ABC",
            ),
        ] {
            assert_eq!(encode::<FramedMessage, _>(&value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<FramedMessage, FramedMessage>(wire, &mut scratch), Ok(value));
        }
        // The frame bounds the body: one longer than its variant is invalid.
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<FramedMessage, FramedMessage>(b"0100030712", &mut scratch).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
        assert_eq!(Body::Echo.wire_name(), Some("0800"));
        assert_eq!(Body::Request(Inner { code: "07".into() }).wire_name(), Some("0100"));
        assert_eq!(Body::Unknown("X".into()).wire_name(), None);
    }

    fn kind_key(kind: &Option<String>) -> &str {
        kind.as_deref().unwrap_or("none")
    }

    fn prefix_key(code: &str) -> &str {
        code.get(..2).unwrap_or_default()
    }

    fn pair_key(code: &str, flag: &str) -> &'static str {
        if code.starts_with('9') || flag == "L" { "02" } else { "01" }
    }

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(selected)]
        enum Keyed {
            #[wire(rename = "01")]
            Short(#[wire(fmt = N2)] String),
            #[wire(rename = "02")]
            Long(#[wire(fmt = N4)] String),
            #[wire(rename = "none")]
            Plain(#[wire(fmt = A1)] String),
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(bitmap = HexBitmap)]
        struct KeyedBitmap {
            #[wire(fmt = N2, bit = 2)]
            kind: Option<String>,
            #[wire(bit = 3, select = kind_key(kind))]
            body: Option<Keyed>,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(concat)]
        struct KeyedConcat<'a> {
            #[wire(fmt = N4)]
            code: &'a str,
            #[wire(fmt = A1)]
            flag: &'a str,
            #[wire(select = prefix_key(code))]
            first: Keyed,
            #[wire(select = self::pair_key(code, flag))]
            second: Option<Keyed>,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(delimited = b'|')]
        struct KeyedDelimited {
            #[wire(fmt = N2)]
            kind: Option<String>,
            #[wire(select = kind_key(kind))]
            body: Option<Keyed>,
        }
    }

    type Tag2 = Field<crate::Alphanum<2, 2>, Fixed<2>>;
    type ByteTag = Field<crate::UpperHexEven<2, 2>, Fixed<1>, crate::PackNibbles<crate::primitive::nibble::UpperHexDigits>>;
    type Ll = Field<Ascii<0, 99>, AsciiLength<2>>;
    type Lll = Field<Ascii<0, 999>, AsciiLength<3>>;
    type ByteLength = crate::Length<crate::FixedBinaryBe<1>>;

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq, Default)]
        #[wire(tlv(tag = Tag2))]
        struct TextTlv<'a> {
            #[wire(tag = "05", fmt = Ll)]
            name: Option<&'a str>,
            /// A tag with a three-digit length.
            #[wire(tag = "06", fmt = Lll)]
            address: Option<String>,
            #[wire(tag = "11", fmt = Ll)]
            city: String,
            /// A tag whose value has no length.
            #[wire(tag = "20", fmt = N4)]
            code: Option<String>,
            #[wire(extras, fmt = Ll)]
            extras: std::collections::BTreeMap<String, String>,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(tlv(tag = ByteTag))]
        struct ByteTlv {
            #[wire(tag = "61", fmt = Field<crate::UpperHexEven<0, 16>, ByteLength, crate::PackNibbles<crate::primitive::nibble::UpperHexDigits>>)]
            key: String,
            #[wire(tag = "62", fmt = crate::Frame<ByteLength, InnerTlv>)]
            nested: Option<InnerTlv>,
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(tlv(tag = Tag2))]
        struct InnerTlv {
            #[wire(tag = "AA", fmt = Ll)]
            a: String,
        }
    }

    #[test]
    fn tlv_records_dispatch_tags_to_formats_with_their_own_lengths() {
        let value = TextTlv {
            name: Some("AB"),
            address: Some("STREET".into()),
            city: "HKI".into(),
            code: Some("1234".into()),
            extras: [("99".to_string(), "Z".to_string())].into(),
        };
        // Declaration order on encode; any order on decode.
        let wire = b"0502AB06006STREET1103HKI2012349901Z";
        assert_eq!(encode::<TextTlv, _>(&value).as_deref(), Ok(&wire[..]));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<TextTlv, TextTlv>(b"9901Z2012341103HKI06006STREET0502AB", &mut scratch),
            Ok(value)
        );
        let absent = TextTlv {
            city: "X".into(),
            ..Default::default()
        };
        assert_eq!(encode::<TextTlv, _>(&absent).as_deref(), Ok(&b"1101X"[..]));
        // A repeated tag, a missing required one, extras claiming a declared tag.
        for (wire, field) in [(&b"1101X1101Y"[..], "city"), (b"0502AB", "city")] {
            let mut scratch = [0; 64];
            let error = crate::decode::<TextTlv, TextTlv>(wire, &mut scratch).unwrap_err();
            assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field(field)][..]));
        }
        let claimed = TextTlv {
            city: "X".into(),
            extras: [("05".to_string(), "A".to_string())].into(),
            ..Default::default()
        };
        let error = encode::<TextTlv, _>(&claimed).unwrap_err();
        assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field("extras")][..]));

        // Binary one-byte tags and lengths, with a nested record as a value.
        let value = ByteTlv {
            key: "ABCD".into(),
            nested: Some(InnerTlv { a: "XY".into() }),
        };
        let wire = b"\x61\x02\xAB\xCD\x62\x06AA02XY";
        assert_eq!(encode::<ByteTlv, _>(&value).as_deref(), Ok(&wire[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<ByteTlv, ByteTlv>(wire, &mut scratch), Ok(value));
        // An unknown tag without extras is invalid.
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<ByteTlv, ByteTlv>(b"\x61\x01\xAB\x63\x00", &mut scratch).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn select_functions_key_by_parts_pairs_and_missing_fields() {
        // An absent key field selects through the function's fallback name.
        for (value, wire) in [
            (
                KeyedBitmap {
                    kind: Some("01".into()),
                    body: Some(Keyed::Short("12".into())),
                },
                &b"60000000000000000112"[..],
            ),
            (
                KeyedBitmap {
                    kind: None,
                    body: Some(Keyed::Plain("X".into())),
                },
                b"2000000000000000X",
            ),
            (
                KeyedBitmap {
                    kind: Some("02".into()),
                    body: None,
                },
                b"400000000000000002",
            ),
        ] {
            assert_eq!(encode::<KeyedBitmap, _>(&value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<KeyedBitmap, KeyedBitmap>(wire, &mut scratch), Ok(value));
        }
        let mut scratch = [0; 64];
        let error = crate::decode::<KeyedBitmap, KeyedBitmap>(b"60000000000000000312", &mut scratch).unwrap_err();
        assert_eq!((error.kind, error.path()), (Error::Invalid, &[PathSegment::Field("body")][..]));

        // A key from part of a field, and one from two fields; the tail may be omitted.
        let value = KeyedConcat {
            code: "0199",
            flag: "L",
            first: Keyed::Short("34".into()),
            second: Some(Keyed::Long("5678".into())),
        };
        assert_eq!(encode::<KeyedConcat, _>(&value).as_deref(), Ok(&b"0199L345678"[..]));
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<KeyedConcat, KeyedConcat>(b"0199L345678", &mut scratch), Ok(value));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<KeyedConcat, KeyedConcat>(b"0299S1234", &mut scratch).map(|value| (value.first, value.second)),
            Ok((Keyed::Long("1234".into()), None))
        );

        // Delimited: an empty segment is an absent key or body.
        for (value, wire) in [
            (
                KeyedDelimited {
                    kind: None,
                    body: Some(Keyed::Plain("X".into())),
                },
                &b"|X"[..],
            ),
            (
                KeyedDelimited {
                    kind: Some("01".into()),
                    body: None,
                },
                b"01|",
            ),
        ] {
            assert_eq!(encode::<KeyedDelimited, _>(&value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<KeyedDelimited, KeyedDelimited>(wire, &mut scratch), Ok(value));
        }
    }

    crate::wire_type! {
        #[derive(Debug, Clone, PartialEq)]
        #[wire(selected)]
        enum Aliased {
            #[wire(rename = "0100", alias = "0101", alias = "0102")]
            Request(#[wire(fmt = N2)] String),
            #[wire(alias = "0111", rename = "0110")]
            Response(#[wire(fmt = N2)] String),
        }

        #[derive(Debug, Clone, PartialEq)]
        #[wire(concat)]
        struct AliasedMessage {
            #[wire(fmt = N4)]
            mti: String,
            #[wire(select = mti)]
            body: Aliased,
        }
    }

    #[test]
    fn aliases_select_a_variant_and_keep_the_key_in_its_field() {
        for (wire, body) in [
            (&b"010012"[..], Aliased::Request("12".into())),
            (b"010112", Aliased::Request("12".into())),
            (b"010212", Aliased::Request("12".into())),
            (b"011134", Aliased::Response("34".into())),
        ] {
            let mut scratch = [0; 64];
            let decoded = crate::decode::<AliasedMessage, AliasedMessage>(wire, &mut scratch).unwrap();
            assert_eq!((&decoded.mti[..], &decoded.body), (std::str::from_utf8(&wire[..4]).unwrap(), &body));
            assert_eq!(encode::<AliasedMessage, _>(&decoded).as_deref(), Ok(wire));
        }
        assert_eq!(Aliased::Request("12".into()).wire_name(), Some("0100"));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<AliasedMessage, AliasedMessage>(b"010312", &mut scratch).map_err(|error| error.kind),
            Err(Error::Invalid)
        );
    }

    #[test]
    fn selected_enums_decode_by_an_earlier_field() {
        let request = MtiMessage {
            mti: "0100",
            body: Body::Request(Inner { code: "07".into() }),
        };
        let response = MtiMessage {
            mti: "0110",
            body: Body::Response("42".into()),
        };
        for (value, wire) in [(&request, &b"010007"[..]), (&response, b"011042")] {
            assert_eq!(encode::<MtiMessage, _>(value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<MtiMessage, MtiMessage>(wire, &mut scratch).as_ref(), Ok(value));
        }
        // A unit variant has no body; `other` takes any other key's body.
        for (value, wire) in [
            (
                MtiMessage {
                    mti: "0800",
                    body: Body::Echo,
                },
                &b"0800"[..],
            ),
            (
                MtiMessage {
                    mti: "0420",
                    body: Body::Unknown("REVERSAL".into()),
                },
                b"0420REVERSAL",
            ),
        ] {
            assert_eq!(encode::<MtiMessage, _>(&value).as_deref(), Ok(wire));
            let mut scratch = [0; 64];
            assert_eq!(crate::decode::<MtiMessage, MtiMessage>(wire, &mut scratch), Ok(value));
        }
        // A matched name commits: the body's error is returned, not `other`.
        let mut scratch = [0; 64];
        let error = crate::decode::<MtiMessage, MtiMessage>(b"0100X7", &mut scratch).unwrap_err();
        assert_eq!(
            (error.kind, error.path()),
            (
                Error::Invalid,
                &[
                    PathSegment::Field("body"),
                    PathSegment::Field("Request"),
                    PathSegment::Field("code")
                ][..]
            )
        );

        let value = DelimitedMessage {
            mti: "0100".into(),
            body: Body::Request(Inner { code: "07".into() }),
        };
        assert_eq!(encode::<DelimitedMessage, _>(&value).as_deref(), Ok(&b"0100|07"[..]));
        let mut scratch = [0; 64];
        assert_eq!(
            crate::decode::<DelimitedMessage, DelimitedMessage>(b"0100|07", &mut scratch),
            Ok(value)
        );

        let value = BitmapMessage {
            mti: "0110".into(),
            body: Body::Response("42".into()),
        };
        let wire = encode::<BitmapMessage, _>(&value).unwrap();
        assert_eq!(wire, b"0110200000000000000042");
        let mut scratch = [0; 64];
        assert_eq!(crate::decode::<BitmapMessage, BitmapMessage>(&wire, &mut scratch), Ok(value));
    }

    #[test]
    fn records_keep_other_attributes() {
        // Serde attributes stay on the struct; `#[wire]` ones are gone.
        let json = serde_json::to_string(&record(None, None)).unwrap();
        assert!(json.starts_with(r#"{"type":"0100","inner":{"code":"07"}"#), "{json}");
    }
}
