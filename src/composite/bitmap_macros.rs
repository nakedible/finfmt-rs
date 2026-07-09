#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_assert_ascending_fields {
    () => {};
    ($id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {
        $crate::__finfmt_bitmap_assert_ascending_fields!(@prev $id; $($($rest)*)?);
    };
    (@prev $prev:literal;) => {};
    (@prev $prev:literal; $id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {
        const _: () = assert!(
            ($prev as usize) < ($id as usize),
            "bitmap fields must be declared in ascending order"
        );
        $crate::__finfmt_bitmap_assert_ascending_fields!(@prev $id; $($($rest)*)?);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_encode_field {
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<DirectScalar<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<DirectScalar<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_serde_scalar::<_, $fmt>(inner, $output, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            $crate::__private::cold_path();
            Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_nested_value::<_, $crate::composite::Composite<$fmt>>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : DirectScalar<$fmt:ty>) => {{
        <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_serde_scalar::<_, $fmt>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_bitmap_field_present {
    ($value:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<DirectScalar<$fmt:ty>>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<DirectScalar<$fmt:ty> >) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Option<$fmt:ty>) => {{ $value.$field.is_some() }};
    ($value:expr, $field:ident : Composite<$fmt:ty>) => {{ true }};
    ($value:expr, $field:ident : DirectScalar<$fmt:ty>) => {{ true }};
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
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt>>) {
            $bitmap.set($id, true);
        }
        $crate::__finfmt_bitmap_set_fields!($bitmap, $value; $($($rest)*)?);
    }};
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt> >) {
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
    ($bitmap:expr, $value:expr; $id:literal => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : DirectScalar<$fmt>) {
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
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt>>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<DirectScalar<$fmt>>)?;
        }
        $crate::__finfmt_bitmap_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : Option<DirectScalar<$fmt> >) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : Option<DirectScalar<$fmt> >)?;
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
    ($value:expr, $output:expr, $scratch:expr; $id:literal => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        if $crate::__finfmt_bitmap_field_present!($value, $field : DirectScalar<$fmt>) {
            $crate::__finfmt_bitmap_encode_field!($value, $output, $scratch, $field : DirectScalar<$fmt>)?;
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

#[macro_export]
macro_rules! bitmap_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime >, $layout:expr, $bitmap_word:ty {
            head: { $($head:tt)* }
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_bitmap_assert_ascending_fields!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($head)*);

                let mut bitmap = $crate::bitmap::Bitmap::new();
                $crate::__finfmt_bitmap_set_fields!(bitmap, value; $($fields)*);
                $crate::bitmap::encode_bitmap::<$bitmap_word>(output, &mut **scratch, &bitmap, $layout).map_err($crate::StructError::from)?;
                $crate::__finfmt_bitmap_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
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
        $crate::__finfmt_bitmap_assert_ascending_fields!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($head)*);

                let mut bitmap = $crate::bitmap::Bitmap::new();
                $crate::__finfmt_bitmap_set_fields!(bitmap, value; $($fields)*);
                $crate::bitmap::encode_bitmap::<$bitmap_word>(output, &mut **scratch, &bitmap, $layout).map_err($crate::StructError::from)?;
                $crate::__finfmt_bitmap_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
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
        let bitmap = $crate::bitmap::decode_bitmap::<$bitmap_word>($input, &mut **$scratch, $layout).map_err($crate::StructError::from)?;
        $crate::__finfmt_bitmap_decode_body_build!(bitmap, $input, $scratch, $result_ty, $ctor; [$($built)*]; $($fields)*)
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let expected: &[u8] = $bytes;
        $crate::composite::decode_literal::<$fmt>($input, $scratch, expected).map_err($crate::StructError::from)?;
        $crate::__finfmt_bitmap_decode_head_build!($input, $scratch, $layout, $bitmap_word, $result_ty, $ctor; [$($built)*]; { $($($rest)*)? } $($fields)*)
    }};
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = Some(
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
        );
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = Some(
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
        );
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = Some(
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
        );
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = Some(
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
        );
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = Some(
            $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
        );
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
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
    ($input:expr, $scratch:expr, $layout:expr, $bitmap_word:ty, $result_ty:ty, $ctor:path; [$($built:tt)*]; { $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)? } $($fields:tt)*) => {{
        let $field = <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
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
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
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
        Ok::<$result_ty, $crate::StructError>({ $ctor { $($built)* } })
    };
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            Some(
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
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
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        } else {
            None
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($bitmap:expr, $input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $id:literal => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = if $bitmap.get($id) {
            $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?
        } else {
            $crate::__private::cold_path();
            return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
        };
        $crate::__finfmt_bitmap_decode_body_build!($bitmap, $input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}
