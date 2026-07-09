#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_encode_field {
    ($value:expr, $output:expr, $scratch:expr, _: $fmt:ty = $bytes:expr) => {{
        let expected: &[u8] = $bytes;
        <$fmt as $crate::ScalarFmt>::encode($output, $scratch, expected).map_err($crate::StructError::from)
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_nested_value::<_, $crate::composite::Composite<$fmt>>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : DirectScalar<$fmt:ty>) => {{
        <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, &$value.$field)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : Composite<$fmt:ty>::with($context:ident)) => {{
        <$fmt as $crate::composite::ContextFmt<_, _>>::encode_with($output, $scratch, &$value.$context, &$value.$field)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
    ($value:expr, $output:expr, $scratch:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_serde_scalar::<_, $fmt>(&$value.$field, $output, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))
    }};
}

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

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                let _ = value;
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $crate::__finfmt_concat_decode_construct_as!(input, scratch, $ty<'de>, $ty; $($fields)*)
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

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                let _ = value;
                $crate::__finfmt_concat_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                $crate::__finfmt_concat_decode_construct!(input, scratch, $ty; $($fields)*)
            }
        }
    };
}

#[macro_export]
macro_rules! absent_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident {
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::AbsentFmt for $name {
            #[inline(always)]
            fn encode_absent(output: &mut &mut [u8], scratch: &mut &mut [u8]) -> Result<(), $crate::Error> {
                $crate::absent_format!(@encode output, scratch; $($fields)*);
                Ok(())
            }
        }
    };
    (@encode $output:expr, $scratch:expr;) => {};
    (@encode $output:expr, $scratch:expr; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        let expected: &[u8] = $bytes;
        <$fmt as $crate::ScalarFmt>::encode($output, $scratch, expected)?;
        $crate::absent_format!(@encode $output, $scratch; $($($rest)*)?);
    }};
    (@encode $output:expr, $scratch:expr; $($unexpected:tt)+) => {
        compile_error!("absent_format! only supports `_ : Fmt = bytes` entries")
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
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let mut __finfmt_omitted_tail = false;
        $crate::__finfmt_concat_encode_tail_fields!(
            $value,
            $output,
            $scratch,
            __finfmt_omitted_tail;
            $field : Option<Composite<$fmt>> $(, $($rest)*)?
        );
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let mut __finfmt_omitted_tail = false;
        $crate::__finfmt_concat_encode_tail_fields!(
            $value,
            $output,
            $scratch,
            __finfmt_omitted_tail;
            $field : Option<Composite<$fmt> > $(, $($rest)*)?
        );
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let mut __finfmt_omitted_tail = false;
        $crate::__finfmt_concat_encode_tail_fields!(
            $value,
            $output,
            $scratch,
            __finfmt_omitted_tail;
            $field : Option<DirectScalar<$fmt>> $(, $($rest)*)?
        );
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let mut __finfmt_omitted_tail = false;
        $crate::__finfmt_concat_encode_tail_fields!(
            $value,
            $output,
            $scratch,
            __finfmt_omitted_tail;
            $field : Option<DirectScalar<$fmt> > $(, $($rest)*)?
        );
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
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, $field : Composite<$fmt>)?;
        $crate::__finfmt_concat_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : Composite<$fmt:ty>::with($context:ident) $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, $field : Composite<$fmt>::with($context))?;
        $crate::__finfmt_concat_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_encode_field!($value, $output, $scratch, $field : DirectScalar<$fmt>)?;
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
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor($output, $scratch, inner)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        match ($omitted, $value.$field.as_ref()) {
            (true, Some(_)) => {
                $crate::__private::cold_path();
                Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)))?;
            }
            (false, Some(inner)) => {
                $crate::composite::encode_serde_scalar::<_, $fmt>(inner, $output, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
            }
            (false, None) => {
                $omitted = true;
            }
            (true, None) => {}
        }
        $crate::__finfmt_concat_encode_tail_fields!($value, $output, $scratch, $omitted; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
    ($value:expr, $output:expr, $scratch:expr, $omitted:ident; $field:ident : $($unexpected:tt)+) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_construct {
    ($input:expr, $scratch:expr, $ty:path; $($fields:tt)*) => {{
        $crate::__finfmt_concat_decode_construct_as!($input, $scratch, $ty, $ty; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_construct_as {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; $($fields:tt)*) => {{
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; []; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_build {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::StructError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {{
        let expected: &[u8] = $bytes;
        $crate::composite::decode_literal::<$fmt>($input, $scratch, expected).map_err($crate::StructError::from)?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)*]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_decode_tail_build!(
            $input,
            $scratch,
            $result_ty,
            $ctor;
            [$($built)*];
            $field : Option<Composite<$fmt>> $(, $($rest)*)?
        )
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_decode_tail_build!(
            $input,
            $scratch,
            $result_ty,
            $ctor;
            [$($built)*];
            $field : Option<Composite<$fmt> > $(, $($rest)*)?
        )
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_decode_tail_build!(
            $input,
            $scratch,
            $result_ty,
            $ctor;
            [$($built)*];
            $field : Option<DirectScalar<$fmt>> $(, $($rest)*)?
        )
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_concat_decode_tail_build!(
            $input,
            $scratch,
            $result_ty,
            $ctor;
            [$($built)*];
            $field : Option<DirectScalar<$fmt> > $(, $($rest)*)?
        )
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
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Composite<$fmt:ty>::with($context:ident) $(, $($rest:tt)*)?) => {{
        let $field = <$fmt as $crate::composite::ContextFmt<_, _>>::decode_with($input, $scratch, &$context)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
            .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?;
        $crate::__finfmt_concat_decode_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_concat_decode_tail_build {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::StructError>({ $ctor { $($built)* } })
    };
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = if $input.is_empty() {
            None
        } else {
            Some(
                $crate::composite::decode_serde_scalar::<_, $fmt>($input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($field)))?,
            )
        };
        $crate::__finfmt_concat_decode_tail_build!($input, $scratch, $result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; _: $fmt:ty = $bytes:expr $(, $($rest:tt)*)?) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; [$($built:tt)*]; $field:ident : $($unexpected:tt)+) => {
        compile_error!("concat_format! only supports Option<...> fields after the first optional field")
    };
}
