#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_has_rest {
    () => {
        false
    };
    ($($rest:tt)+) => {
        true
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_encode_field {
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, _: $fmt:ty = $bytes:expr) => {{
        let expected: &[u8] = $bytes;
        $crate::composite::encode_delimited_literal::<$fmt>($output, $scratch, expected, $separator)
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_value::<_, $fmt>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::CompositeError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident ($context:ident) : $fmt:ty) => {{
        $crate::composite::encode_delimited_context::<_, _, $fmt>($output, $scratch, &$value.$context, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_delimited_value::<_, $fmt>($output, $scratch, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_encode_next {
    ($value:expr, $output:expr, $scratch:expr, $separator:expr;) => {};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $($rest:tt)+) => {{
        $crate::composite::encode_delimiter($output, $separator)?;
        $crate::__finfmt_delimited_encode_fields!($value, $output, $scratch, $separator; $($rest)+);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_encode_fields {
    ($value:expr, $output:expr, $scratch:expr, $separator:expr;) => {};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, if $crate::__finfmt_delimited_has_rest!($($($rest)*)?) { Some($separator) } else { None }, _: $fmt = $bytes)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, if $crate::__finfmt_delimited_has_rest!($($($rest)*)?) { Some($separator) } else { None }, $field : Option<$fmt>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident ($context:ident) : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, if $crate::__finfmt_delimited_has_rest!($($($rest)*)?) { Some($separator) } else { None }, $field ($context) : $fmt)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, if $crate::__finfmt_delimited_has_rest!($($($rest)*)?) { Some($separator) } else { None }, $field : $fmt)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_decode_build {
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::CompositeError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::composite::decode_delimited_field($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let expected: &[u8] = $bytes;
        $crate::composite::decode_delimited_literal::<$fmt>(segment, $scratch, expected)?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)*]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::composite::decode_delimited_field($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $fmt>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident ($context:ident) : $fmt:ty $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::composite::decode_delimited_field($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_context::<_, _, $fmt>(segment, $scratch, &$context)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::composite::decode_delimited_field($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_value::<_, $fmt>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

/// Define a record separated by a byte. The final field consumes the remainder
/// of the bounded input and may itself contain the separator. Earlier fields
/// cannot encode that byte. An empty optional segment decodes as `None`, even
/// when it was encoded from a present value that produced no bytes.
#[macro_export]
macro_rules! delimited_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime >, $separator:tt {
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::FieldEncode<$ty<$lt>> for $name {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::CompositeError> {
                let _ = value;
                $crate::__finfmt_delimited_encode_fields!(value, output, scratch, $separator; $($fields)*);
                Ok(())
            }
        }

        impl<'de> $crate::composite::FieldDecode<'de, $ty<'de>> for $name {
            #[inline(always)]
            fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<$ty<'de>, $crate::CompositeError> {
                $crate::__finfmt_delimited_decode_build!(input, scratch, $separator, $ty<'de>, $ty; []; $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path, $separator:tt {
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::FieldEncode<$ty> for $name {
            #[inline(always)]
            fn encode_field(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty) -> Result<(), $crate::CompositeError> {
                let _ = value;
                $crate::__finfmt_delimited_encode_fields!(value, output, scratch, $separator; $($fields)*);
                Ok(())
            }
        }

        impl<'de> $crate::composite::FieldDecode<'de, $ty> for $name {
            #[inline(always)]
            fn decode_field(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<$ty, $crate::CompositeError> {
                $crate::__finfmt_delimited_decode_build!(input, scratch, $separator, $ty, $ty; []; $($fields)*)
            }
        }
    };
}
