#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_encode_field {
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<DirectScalar<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<DirectScalar<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                $crate::composite::encode_serde_scalar::<_, $fmt>(inner, value_out, scratch).map_err($crate::StructError::from)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            $crate::composite::encode_nested_value::<_, $crate::composite::Composite<$fmt>>(&$value.$field, value_out, scratch)
        })?;
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : DirectScalar<$fmt:ty>) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::encode_cursor(value_out, scratch, &$value.$field)
        })?;
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            $crate::composite::encode_serde_scalar::<_, $fmt>(&$value.$field, value_out, scratch).map_err($crate::StructError::from)
        })?;
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_init_fields {
    () => {};
    ($tag:expr => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {
        let mut $field = None;
        $crate::__finfmt_ber_tlv_init_fields!($($($rest)*)?);
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_encode_fields {
    ($value:expr, $output:expr, $scratch:expr;) => {};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<Composite<$fmt>>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<Composite<$fmt> >);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<DirectScalar<$fmt>>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<DirectScalar<$fmt> >);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<$fmt>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Composite<$fmt>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : DirectScalar<$fmt>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : $fmt);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_match_field {
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<Composite<$fmt:ty>>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<Composite<$fmt:ty> >) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty>>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty> >) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<$fmt:ty>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    $crate::composite::decode_serde_scalar::<_, $fmt>(value_input, scratch).map_err($crate::StructError::from)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Composite<$fmt:ty>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : DirectScalar<$fmt:ty>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt> as $crate::composite::CompositeFmt<_>>::decode_cursor(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : $fmt:ty) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    $crate::composite::decode_serde_scalar::<_, $fmt>(value_input, scratch).map_err($crate::StructError::from)
                },
            )?;
        }
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_match_fields {
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident;) => {};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<Composite<$fmt>>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<Composite<$fmt> >);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<DirectScalar<$fmt>>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<DirectScalar<$fmt> >);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<$fmt>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Composite<$fmt>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : DirectScalar<$fmt>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : $fmt);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_finish_fields_as {
    ($result_ty:ty, $ctor:path; [$($built:tt)*];) => {
        Ok::<$result_ty, $crate::StructError>({ $ctor { $($built)* } })
    };
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Composite<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : DirectScalar<$fmt:ty> $(, $($rest:tt)*)?) => {{
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_struct_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_decode_construct {
    ($input:expr, $scratch:expr, $ty:path; extras: $extras:ident, $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_decode_construct_as!($input, $scratch, $ty, $ty; extras: $extras, $($fields)*)
    }};
    ($input:expr, $scratch:expr, $ty:path; $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_decode_construct_as!($input, $scratch, $ty, $ty; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_decode_construct_as {
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; extras: $extras:ident, $($fields:tt)*) => {{
        let mut $extras = ::core::default::Default::default();
        $crate::__finfmt_ber_tlv_init_fields!($($fields)*);

        while let Some(entry) = $crate::primitive::bertlv::decode_ber_tlv_entry($input).map_err($crate::StructError::from)? {
            let mut value_input = entry.value;
            let mut matched = false;
            $crate::__finfmt_ber_tlv_match_fields!(entry.tag, &mut value_input, $scratch, matched; $($fields)*);
            if !matched {
                $crate::composite::BerTlvExtras::decode_unknown(&mut $extras, entry.tag, value_input, $scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($extras)))?;
            }
        }

        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$extras: $extras,]; $($fields)*)
    }};
    ($input:expr, $scratch:expr, $result_ty:ty, $ctor:path; $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_init_fields!($($fields)*);

        while let Some(entry) = $crate::primitive::bertlv::decode_ber_tlv_entry($input).map_err($crate::StructError::from)? {
            let mut value_input = entry.value;
            let mut matched = false;
            $crate::__finfmt_ber_tlv_match_fields!(entry.tag, &mut value_input, $scratch, matched; $($fields)*);
            if !matched {
                $crate::__private::cold_path();
                return Err($crate::StructError::from($crate::Error::Invalid));
            }
        }

        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; []; $($fields)*)
    }};
}

#[macro_export]
macro_rules! ber_tlv_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > {
            extras: $extras:ident,
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                $crate::composite::BerTlvExtras::encode_unknowns(&value.$extras, output, scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($extras)))?;
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $crate::__finfmt_ber_tlv_decode_construct_as!(input, scratch, $ty<'de>, $ty; extras: $extras, $($fields)*)
            }
        }
    };
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
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $crate::__finfmt_ber_tlv_decode_construct_as!(input, scratch, $ty<'de>, $ty; $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path {
            extras: $extras:ident,
            $($fields:tt)*
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                $crate::composite::BerTlvExtras::encode_unknowns(&value.$extras, output, scratch)
                    .map_err(|error| $crate::composite::wrap_struct_error(error, stringify!($extras)))?;
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                $crate::__finfmt_ber_tlv_decode_construct!(input, scratch, $ty; extras: $extras, $($fields)*)
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
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                $crate::__finfmt_ber_tlv_decode_construct!(input, scratch, $ty; $($fields)*)
            }
        }
    };
}
