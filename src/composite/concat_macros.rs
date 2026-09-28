#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_encode_field {
    ($value:expr, $output:expr, $scratch:expr, _: $fmt:ty = $bytes:expr) => {{
        let expected: &[u8] = $bytes;
        <$fmt as $crate::ScalarFmt>::encode($output, $scratch, expected).map_err($crate::CompositeError::from)
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident ($context:ident) : $fmt:ty) => {{
        <$fmt as $crate::composite::ContextEncode<_, _>>::encode_with($output, $scratch, &$value.$context, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : $fmt:ty) => {{
        <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
}

/// Define a record of fields written back to back. `Option` fields may only
/// follow the required ones: they are omitted at the end of the record, and an
/// omitted field cannot be followed by a present one.
#[macro_export]
macro_rules! concat_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > {
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::FieldEncode<$ty<$lt>> for $name {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::CompositeError> {
                let _ = value;
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }
        }

        impl<'de> $crate::composite::FieldDecode<'de, $ty<'de>> for $name {
            #[inline(always)]
            fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<$ty<'de>, $crate::CompositeError> {
                $crate::__finfmt_concat_decode_build!(input, scratch, $ty<'de>, $ty; []; $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path {
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::FieldEncode<$ty> for $name {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty) -> Result<(), $crate::CompositeError> {
                let _ = value;
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }
        }

        impl<'de> $crate::composite::FieldDecode<'de, $ty> for $name {
            #[inline(always)]
            fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<$ty, $crate::CompositeError> {
                $crate::__finfmt_concat_decode_build!(input, scratch, $ty, $ty; []; $($fields)*)
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_encode_fields {
    ($value:expr, $output:expr, $scratch:expr;) => {};
    ($value:expr, $output:expr, $scratch:expr; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, _: $fmt = $bytes)?;
        $crate::__finfmt_concat_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let mut __finfmt_omitted_tail = false;
        $crate::__finfmt_concat_encode_tail_fields!(
            $value,
            $output,
            $scratch,
            __finfmt_omitted_tail;
            $field : Option<$fmt> $(, $($rest)*)?
        );
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident ($context:ident) : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, $field ($context) : $fmt)?;
        $crate::__finfmt_concat_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, $field : $fmt)?;
        $crate::__finfmt_concat_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_encode_tail_fields {
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident;) => {};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                <$fmt as $crate::composite::FieldEncode<_>>::encode_field($output, $scratch, inner)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $($unexpected:tt)+) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_build {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::CompositeError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        let expected: &[u8] = $bytes;
        $crate::composite::decode_literal::<$fmt>($input, $scratch, expected).map_err($crate::CompositeError::from)?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)*]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_decode_tail_build!(
            $input,
            $scratch,
            $result_ty,
            $ctor;
            [$($built)*];
            $field : Option<$fmt> $(, $($rest)*)?
        )
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident ($context:ident) : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = <$fmt as $crate::composite::ContextDecode<'_, _, _>>::decode_with($input, $scratch, &$context)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_tail_build {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::CompositeError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$fmt as $crate::composite::FieldDecode<'_, _>>::decode_field($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $($unexpected:tt)+) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
}
