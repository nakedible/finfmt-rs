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
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_value::<_, $crate::composite::Composite<$fmt>>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::StructError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_value::<_, $crate::composite::Composite<$fmt>>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::StructError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<DirectScalar<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::StructError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<DirectScalar<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::StructError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_delimited_serde_value::<_, $fmt>($output, $scratch, inner, $separator)
                .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
        } else {
            Ok::<(), $crate::StructError>(())
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Composite<$fmt:ty>::with($context:ident)) => {{
        $crate::composite::encode_delimited_context::<_, _, $fmt>($output, $scratch, &$value.$context, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_delimited_value::<_, $crate::composite::Composite<$fmt>>($output, $scratch, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : DirectScalar<$fmt:ty>) => {{
        $crate::composite::encode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>($output, $scratch, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_delimited_serde_value::<_, $fmt>($output, $scratch, &$value.$field, $separator)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
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
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, _: $fmt = $bytes)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Option<Composite<$fmt>>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Option<Composite<$fmt> >)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Option<DirectScalar<$fmt>>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Option<DirectScalar<$fmt> >)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Option<$fmt>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Composite<$fmt:ty>::with($context:ident) $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Composite<$fmt>::with($context))?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : Composite<$fmt>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : DirectScalar<$fmt>)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $separator:expr; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_delimited_encode_field!($value, $output, $scratch, $separator, $field : $fmt)?;
        $crate::__finfmt_delimited_encode_next!($value, $output, $scratch, $separator; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_decode_construct {
    ($input:expr, $scratch:expr, $separator:expr, $ty:path; $($fields:tt)*) => {{
        $crate::__finfmt_delimited_decode_construct_as!($input, $scratch, $separator, $ty, $ty; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_decode_construct_as {
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; $($fields:tt)*) => {{
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; []; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_delimited_decode_build {
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::StructError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let expected: &[u8] = $bytes;
        $crate::composite::decode_delimited_literal::<$fmt>(segment, $scratch, expected)?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)*]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $crate::composite::Composite<$fmt>>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $crate::composite::Composite<$fmt>>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = if segment.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_delimited_serde_value::<_, $fmt>(segment, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Composite<$fmt:ty>::with($context:ident) $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_context::<_, _, $fmt>(segment, $scratch, &$context)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_value::<_, $crate::composite::Composite<$fmt>>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_value::<_, $crate::composite::DirectScalar<$fmt>>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $separator:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let segment =
            $crate::primitive::bytes::split_delimited_bytes($input, $separator, $crate::__finfmt_delimited_has_rest!($($($rest)*)?))?;
        let $field = $crate::composite::decode_delimited_serde_value::<_, $fmt>(segment, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_delimited_decode_build!($input, $scratch, $separator, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

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

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                let _ = value;
                $crate::__finfmt_delimited_encode_fields!(value, output, scratch, $separator; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $crate::__finfmt_delimited_decode_construct_as!(input, scratch, $separator, $ty<'de>, $ty; $($fields)*)
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

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                let _ = value;
                $crate::__finfmt_delimited_encode_fields!(value, output, scratch, $separator; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                $crate::__finfmt_delimited_decode_construct!(input, scratch, $separator, $ty; $($fields)*)
            }
        }
    };
}
