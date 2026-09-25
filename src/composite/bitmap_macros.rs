#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_assert_fields {
    ($layout:expr, $word:ty; $($id:literal => $field:ident : $fmt:ty),* $(,)?) => {
        const _: () = {
            let layout: $crate::bitmap::BitmapLayout = $layout;
            let width = layout.word_bits as usize / 8;
            let max_words = layout.max_words as usize;
            let [second_flag, third_flag] = layout.word_flags;
            assert!(layout.word_bits > 0 && layout.word_bits <= 64 && layout.word_bits % 8 == 0, "bitmap word width must be a multiple of 8 bits up to 64");
            assert!(layout.min_words > 0 && layout.min_words <= layout.max_words && layout.max_words <= 3, "invalid bitmap word counts");
            assert!(width == 8 || max_words == 1, "a narrow bitmap must be a single word");
            let mut flags: [u16; 2] = [0; 2];
            if max_words > 1 && let Some(flag) = second_flag {
                assert!(flag > 0 && flag <= 64, "a bitmap word flag must be in an earlier word");
                flags[0] = flag as u16;
            }
            if max_words > 2 && let Some(flag) = third_flag {
                assert!(flag > 0 && flag <= 128, "a bitmap word flag must be in an earlier word");
                assert!(flag as u16 != flags[0], "bitmap word flags must be distinct");
                flags[1] = flag as u16;
            }
            let fields: &[u16] = &[$($id),*];
            let mut index = 0;
            while index < fields.len() {
                let id = fields[index] as usize;
                assert!(id > 0 && id <= 192, "bitmap field must be in 1..=192");
                assert!(index == 0 || fields[index - 1] < fields[index], "bitmap fields must be declared in ascending order");
                let word = (id - 1) / 64;
                let bit = (id - 1) % 64 + 1;
                assert!(word < max_words, "bitmap field exceeds configured word count");
                assert!(bit <= width * 8, "bitmap field exceeds decoded word width");
                assert!(fields[index] != flags[0] && fields[index] != flags[1], "bitmap field uses a word flag");
                index += 1;
            }
        };
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_reject_unknown {
    ($bitmap:expr; $($id:literal => $field:ident : $fmt:ty),* $(,)?) => {{
        #[allow(unused_mut)]
        let mut unknown = $bitmap;
        $(unknown.set($id, false);)*
        if unknown != $crate::bitmap::Bitmap::new() {
            $crate::__private::cold_path();
            return Err($crate::Error::Invalid.into());
        }
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_encode_field {
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_serde_scalar::<_, $fmt>(inner, $output, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_nested_value::<_, $crate::composite::Composite<$fmt>>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?>) => {{
        <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_serde_scalar::<_, $fmt>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_field_present {
    ($value:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> >) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<$fmt:ty>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Composite<$fmt:ty>) => {{ true }};
    ($value:expr, $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?>) => {{ true }};
    ($value:expr, $field:ident : $fmt:ty) => {{ true }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_set_fields {
    ($bitmap:expr, $value:expr;) => {};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<Composite<$fmt>>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<Composite<$fmt> >) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt $(, $value_ty)?>>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt $(, $value_ty)?> >) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<$fmt>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Composite<$fmt>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : DirectScalar<$fmt $(, $value_ty)?>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : $fmt) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_encode_fields {
    ($value:expr, $output:expr, $scratch:expr;) => {};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<Composite<$fmt>>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<Composite<$fmt>>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<Composite<$fmt> >) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<Composite<$fmt> >)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt $(, $value_ty)?>>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<DirectScalar<$fmt $(, $value_ty)?>>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt $(, $value_ty)?> >) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<DirectScalar<$fmt $(, $value_ty)?> >)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<$fmt>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<$fmt>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Composite<$fmt>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Composite<$fmt>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : DirectScalar<$fmt $(, $value_ty)?>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : DirectScalar<$fmt $(, $value_ty)?>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : $fmt) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : $fmt)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
}

/// Define a bitmap record with a constant layout and ascending field numbers.
///
/// Fields must fit the configured words and their decoded width, and must not
/// be word flags. Unknown incoming fields are rejected before
/// any body field is decoded. Header slots are always physically present; use
/// an explicit `OptionalAbsent` format for a header absence pattern.
///
/// An out-of-range field is a declaration error.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: String }
/// bitmap_format! { struct Format for Record, BitmapLayout::fixed(3), Identity {
///     193 => value: A2,
/// } }
/// ```
///
/// Fields must be strictly ascending.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { first: String, second: String }
/// bitmap_format! { struct Format for Record, BitmapLayout::fixed(1), Identity {
///     3 => first: A2, 2 => second: A2,
/// } }
/// ```
///
/// Fields must fit the word count.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: String }
/// bitmap_format! { struct Format for Record, BitmapLayout::fixed(1), Identity {
///     65 => value: A2,
/// } }
/// ```
///
/// Fields must fit a narrow word.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: String }
/// bitmap_format! { struct Format for Record, BitmapLayout::bits(32), Identity {
///     33 => value: A2,
/// } }
/// ```
///
/// A narrow bitmap is a single word.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: String }
/// bitmap_format! { struct Format for Record, BitmapLayout { max_words: 2, ..BitmapLayout::bits(32) }, Identity {
///     2 => value: A2,
/// } }
/// ```
///
/// Word flags are not fields.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: String }
/// bitmap_format! { struct Format for Record, BitmapLayout::iso(1, 2), Identity {
///     1 => value: A2,
/// } }
/// ```
///
/// A header cannot be omitted through container presence syntax.
///
/// ```compile_fail
/// # use finfmt::*;
/// # use finfmt::bitmap::BitmapLayout;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { value: Option<String> }
/// bitmap_format! { struct Format for Record, BitmapLayout::fixed(1), Identity {
///     head: { value: Option<A2>, }
/// } }
/// ```
#[macro_export]
macro_rules! bitmap_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime >, $layout:expr, $bitmap_word:ty {
            head: { $($head:tt)* }
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_bitmap_assert_fields!($layout, $bitmap_word; $($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($head)*);

                let mut bitmap = $crate::bitmap::Bitmap::new();
                $crate::__finfmt_bitmap_set_fields!(bitmap, value; $($fields)*);
                $crate::bitmap::encode_bitmap::<$bitmap_word>(output, &mut *scratch, &bitmap, $layout).map_err($crate::CompositeError::from)?;
                $crate::__finfmt_bitmap_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::CompositeError> {
                $crate::__finfmt_bitmap_decode_construct_as!(input, scratch, $layout, $bitmap_word, $ty<'de>, $ty; { $($head)* } $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime >, $layout:expr, $bitmap_word:ty {
            $($fields:tt)*
        }
    ) => {
        $crate::bitmap_format! {
            $(#[$attr])*
            $vis struct $name for<$lt> $ty<$ty_lt>, $layout, $bitmap_word {
                head: {}
                $($fields)*
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path, $layout:expr, $bitmap_word:ty {
            head: { $($head:tt)* }
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_bitmap_assert_fields!($layout, $bitmap_word; $($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($head)*);

                let mut bitmap = $crate::bitmap::Bitmap::new();
                $crate::__finfmt_bitmap_set_fields!(bitmap, value; $($fields)*);
                $crate::bitmap::encode_bitmap::<$bitmap_word>(output, &mut *scratch, &bitmap, $layout).map_err($crate::CompositeError::from)?;
                $crate::__finfmt_bitmap_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::CompositeError> {
                $crate::__finfmt_bitmap_decode_construct!(input, scratch, $layout, $bitmap_word, $ty; { $($head)* } $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path, $layout:expr, $bitmap_word:ty {
            $($fields:tt)*
        }
    ) => {
        $crate::bitmap_format! {
            $(#[$attr])*
            $vis struct $name for $ty, $layout, $bitmap_word {
                head: {}
                $($fields)*
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_decode_construct {
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $ty:path; { $($head:tt)* } $($fields:tt)*) => {{
        $crate::__finfmt_bitmap_decode_construct_as!($input, $scratch, $layout, $bitmap_word, $ty, $ty; { $($head)* } $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_decode_construct_as {
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; { $($head:tt)* } $($fields:tt)*) => {{
        $crate::__finfmt_bitmap_decode_head_build!($input, $scratch, $layout, $bitmap_word, $result_ty, $ctor; []; { $($head)* } $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_decode_head_build {
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { } $($fields:tt)*) => {{
        let bitmap = $crate::bitmap::decode_bitmap::<$bitmap_word>($input, &mut **$scratch, $layout).map_err($crate::CompositeError::from)?;
        $crate::__finfmt_bitmap_reject_unknown!(bitmap; $($fields)*);
        $crate::__finfmt_bitmap_decode_body_build!(bitmap, $input, $scratch, $result_ty, $ctor; [$($built)*]; $($fields)*)
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let expected: &[u8] = $bytes;
        $crate::composite::decode_literal::<$fmt>($input, $scratch, expected).map_err($crate::CompositeError::from)?;
        $crate::__finfmt_bitmap_decode_head_build!($input, $scratch, $layout, $bitmap_word, $result_ty, $ctor; [$($built)*]; { $($($rest)*)? } $($fields)*)
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        compile_error!("bitmap head fields cannot use container Option; use an explicit OptionalAbsent format");
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_bitmap_decode_head_build!(
            $input,
            $scratch,
            $layout,
            $bitmap_word,
            $result_ty,
            $ctor;
            [$($built)* $field: $field,];
            { $($($rest)*)? }
            $($fields)*
        )
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_bitmap_decode_head_build!(
            $input,
            $scratch,
            $layout,
            $bitmap_word,
            $result_ty,
            $ctor;
            [$($built)* $field: $field,];
            { $($($rest)*)? }
            $($fields)*
        )
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : $fmt:ty $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_bitmap_decode_head_build!(
            $input,
            $scratch,
            $layout,
            $bitmap_word,
            $result_ty,
            $ctor;
            [$($built)* $field: $field,];
            { $($($rest)*)? }
            $($fields)*
        )
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_decode_body_build {
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::CompositeError>({ $ctor { $($built)* } })
    };
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode($input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}
