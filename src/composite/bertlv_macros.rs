#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_encode_field {
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<Composite<$fmt:ty>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<Composite<$fmt:ty> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::encode(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> >) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode(value_out, scratch, inner)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Option<$fmt:ty>) => {{
        if let Some(inner) = $value.$field.as_ref() {
            $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
                $crate::composite::encode_serde_scalar::<_, $fmt>(inner, value_out, scratch).map_err($crate::CompositeError::from)
            })?;
        }
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : Composite<$fmt:ty>) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            $crate::composite::encode_nested_value::<_, $crate::composite::Composite<$fmt>>(&$value.$field, value_out, scratch)
        })?;
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?>) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::encode(value_out, scratch, &$value.$field)
        })?;
    }};
    ($value:expr, $output:expr, $scratch:expr, $tag:expr, $field:ident : $fmt:ty) => {{
        $crate::composite::encode_ber_tlv_field($output, $scratch, $tag, stringify!($field), |value_out, scratch| {
            $crate::composite::encode_serde_scalar::<_, $fmt>(&$value.$field, value_out, scratch).map_err($crate::CompositeError::from)
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
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<DirectScalar<$fmt $(, $value_ty)?>>);
        $crate::__finfmt_ber_tlv_encode_fields!($value, $output, $scratch; $($($rest)*)?);
    }};
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : Option<DirectScalar<$fmt $(, $value_ty)?> >);
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
    ($value:expr, $output:expr, $scratch:expr; $tag:expr => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_encode_field!($value, $output, $scratch, $tag, $field : DirectScalar<$fmt $(, $value_ty)?>);
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
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
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
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> >) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
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
                    $crate::composite::decode_serde_scalar::<_, $fmt>(value_input, scratch).map_err($crate::CompositeError::from)
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
                    <$crate::composite::Composite<$fmt> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
                },
            )?;
        }
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident, $tag:expr => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?>) => {{
        if !$matched {
            $matched = $crate::composite::decode_ber_tlv_field(
                $tag_bytes,
                $tag,
                $value_input,
                $scratch,
                &mut $field,
                stringify!($field),
                |value_input, scratch| {
                    <$crate::composite::DirectScalar<$fmt $(, $value_ty)?> as $crate::composite::CompositeFmt<_>>::decode(value_input, scratch)
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
                    $crate::composite::decode_serde_scalar::<_, $fmt>(value_input, scratch).map_err($crate::CompositeError::from)
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
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<DirectScalar<$fmt $(, $value_ty)?>>);
        $crate::__finfmt_ber_tlv_match_fields!($tag_bytes, $value_input, $scratch, $matched; $($($rest)*)?);
    }};
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : Option<DirectScalar<$fmt $(, $value_ty)?> >);
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
    ($tag_bytes:expr, $value_input:expr, $scratch:expr, $matched:ident; $tag:expr => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        $crate::__finfmt_ber_tlv_match_field!($tag_bytes, $value_input, $scratch, $matched, $tag => $field : DirectScalar<$fmt $(, $value_ty)?>);
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
        Ok::<$result_ty, $crate::CompositeError>({ $ctor { $($built)* } })
    };
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<Composite<$fmt:ty>> $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<Composite<$fmt:ty> > $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?>> $(, $($rest:tt)*)?) => {{
        let $field = $field;
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : Option<DirectScalar<$fmt:ty $(, $value_ty:ty)?> > $(, $($rest:tt)*)?) => {{
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
                return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : DirectScalar<$fmt:ty $(, $value_ty:ty)?> $(, $($rest:tt)*)?) => {{
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
    ($result_ty:ty, $ctor:path; [$($built:tt)*]; $tag:expr => $field:ident : $fmt:ty $(, $($rest:tt)*)?) => {{
        let $field = match $field {
            Some(value) => value,
            None => {
                $crate::__private::cold_path();
                return Err($crate::composite::wrap_composite_error($crate::Error::Invalid, stringify!($field)));
            }
        };
        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$($built)* $field: $field,]; $($($rest)*)?)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_decode_construct {
    ($input:expr, $scratch:expr, $padding:expr, $ty:path; extras: $extras:ident, $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_decode_construct_as!($input, $scratch, $padding, $ty, $ty; extras: $extras, $($fields)*)
    }};
    ($input:expr, $scratch:expr, $padding:expr, $ty:path; $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_decode_construct_as!($input, $scratch, $padding, $ty, $ty; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_decode_construct_as {
    ($input:expr, $scratch:expr, $padding:expr, $result_ty:ty, $ctor:path; extras: $extras:ident, $($fields:tt)*) => {{
        let mut $extras = ::core::default::Default::default();
        $crate::__finfmt_ber_tlv_init_fields!($($fields)*);

        while let Some(entry) = $crate::composite::decode_ber_tlv_collection_entry::<{ $padding }>($input).map_err($crate::CompositeError::from)? {
            let mut value_input = entry.value;
            let mut matched = false;
            let mut tag_hex = [0; $crate::primitive::bertlv::MAX_BER_TAG_HEX];
            let tag_hex = $crate::primitive::bertlv::format_ber_tag_hex(&mut tag_hex, entry.tag);
            $crate::__finfmt_ber_tlv_match_fields!(tag_hex, &mut value_input, $scratch, matched; $($fields)*);
            if !matched {
                $crate::composite::BerTlvExtras::decode_unknown(&mut $extras, entry.tag, value_input, $scratch)
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($extras)))?;
            }
        }

        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; [$extras: $extras,]; $($fields)*)
    }};
    ($input:expr, $scratch:expr, $padding:expr, $result_ty:ty, $ctor:path; $($fields:tt)*) => {{
        $crate::__finfmt_ber_tlv_init_fields!($($fields)*);

        while let Some(entry) = $crate::composite::decode_ber_tlv_collection_entry::<{ $padding }>($input).map_err($crate::CompositeError::from)? {
            let mut value_input = entry.value;
            let mut matched = false;
            let mut tag_hex = [0; $crate::primitive::bertlv::MAX_BER_TAG_HEX];
            let tag_hex = $crate::primitive::bertlv::format_ber_tag_hex(&mut tag_hex, entry.tag);
            $crate::__finfmt_ber_tlv_match_fields!(tag_hex, &mut value_input, $scratch, matched; $($fields)*);
            if !matched {
                $crate::__private::cold_path();
                return Err($crate::CompositeError::from($crate::Error::Invalid));
            }
        }

        $crate::__finfmt_ber_tlv_finish_fields_as!($result_ty, $ctor; []; $($fields)*)
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_known_tags {
    ($($tag:expr => $field:ident : $fmt:ty),* $(,)?) => { &[$($tag),*] };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_ber_tlv_assert_tags {
    ($($tag:expr => $field:ident : $fmt:ty),* $(,)?) => {
        const _: () = {
            let tags: &[&str] = &[$($tag),*];
            let mut i = 0;
            while i < tags.len() {
                let mut j = i + 1;
                while j < tags.len() {
                    let a = tags[i].as_bytes();
                    let b = tags[j].as_bytes();
                    if a.len() == b.len() {
                        let mut k = 0;
                        while k < a.len() && a[k] == b[k] {
                            k += 1;
                        }
                        assert!(k != a.len(), "duplicate declared BER tag");
                    }
                    j += 1;
                }
                i += 1;
            }
        };
    };
}

/// Define a named BER-TLV format. Decoding rejects padding by default.
///
/// Add `, allow_zero_padding = true` after the target type and before `{` to
/// accept `00` bytes between entries and at either end. Each nested format has
/// its own setting. Encoding never adds padding; value bytes are not trimmed.
///
/// Encoding stages each value in scratch before writing its tag and length, so
/// scratch must hold the largest value. The unwritten output is the value's own
/// workspace, as for `Frame`: with an output sized exactly to the message, a
/// value that needs workspace can report `BufferOverflow`.
///
/// Declared tags must be unique constant expressions. Repeated wire occurrences
/// of a known tag are also rejected, independently of this declaration check.
///
/// ```compile_fail
/// # use finfmt::*;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { first: String, second: String }
/// const TAG: &str = "5A";
/// ber_tlv_format! { struct Format for Record {
///     TAG => first: A2,
///     "5A" => second: A2,
/// } }
/// ```
///
/// ```compile_fail
/// # use finfmt::*;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record { first: String, second: String, extras: std::collections::BTreeMap<String, String> }
/// const TAG: &str = "5A";
/// ber_tlv_format! { struct Format for Record {
///     extras: extras,
///     TAG => first: A2,
///     "5A" => second: A2,
/// } }
/// ```
///
/// ```compile_fail
/// # use finfmt::*;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record<'a> { first: &'a str, second: &'a str }
/// const TAG: &str = "5A";
/// ber_tlv_format! { struct Format for<'a> Record<'a> {
///     TAG => first: A2,
///     "5A" => second: A2,
/// } }
/// ```
///
/// ```compile_fail
/// # use finfmt::*;
/// # type A2 = Field<Ascii<2, 2>, Fixed<2>>;
/// # struct Record<'a> { first: &'a str, second: &'a str, extras: std::collections::BTreeMap<String, String> }
/// const TAG: &str = "5A";
/// ber_tlv_format! { struct Format for<'a> Record<'a> {
///     extras: extras,
///     TAG => first: A2,
///     "5A" => second: A2,
/// } }
/// ```
#[macro_export]
macro_rules! ber_tlv_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > $(, allow_zero_padding = $padding:tt)? {
            extras: $extras:ident,
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_ber_tlv_assert_tags!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                $crate::composite::BerTlvExtras::encode_unknowns(&value.$extras, output, scratch, $crate::__finfmt_ber_tlv_known_tags!($($fields)*))
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($extras)))?;
                Ok(())
            }

            #[inline(always)]
            fn decode<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_decode_construct_as!(input, scratch, false $(|| $padding)?, $ty<'de>, $ty; extras: $extras, $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > $(, allow_zero_padding = $padding:tt)? {
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_ber_tlv_assert_tags!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_decode_construct_as!(input, scratch, false $(|| $padding)?, $ty<'de>, $ty; $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path $(, allow_zero_padding = $padding:tt)? {
            extras: $extras:ident,
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_ber_tlv_assert_tags!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                $crate::composite::BerTlvExtras::encode_unknowns(&value.$extras, output, scratch, $crate::__finfmt_ber_tlv_known_tags!($($fields)*))
                    .map_err(|error| $crate::composite::wrap_composite_error(error, stringify!($extras)))?;
                Ok(())
            }

            #[inline(always)]
            fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_decode_construct!(input, scratch, false $(|| $padding)?, $ty; extras: $extras, $($fields)*)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:path $(, allow_zero_padding = $padding:tt)? {
            $($fields:tt)*
        }
    ) => {
        $crate::__finfmt_ber_tlv_assert_tags!($($fields)*);

        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode(output: &mut &mut [u8], scratch: &mut [u8], value: &$ty) -> Result<(), $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_encode_fields!(value, output, scratch; $($fields)*);
                Ok(())
            }

            #[inline(always)]
            fn decode<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::CompositeError> {
                $crate::__finfmt_ber_tlv_decode_construct!(input, scratch, false $(|| $padding)?, $ty; $($fields)*)
            }
        }
    };
}
