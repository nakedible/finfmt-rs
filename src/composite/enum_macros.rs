#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_tagged_const_decode_arm {
    ($input:expr, $scratch:expr, $ty:ident, $variant:ident, $fmt:ty, $literal_fmt:ty, $literal_bytes:expr) => {{
        let expected: &[u8] = $literal_bytes;
        if $crate::composite::match_literal::<$literal_fmt>($input, $scratch, expected).map_err($crate::StructError::from)? {
            return $crate::__private::decode_variant::<_, _, $fmt, _>($input, $scratch, $ty::$variant);
        }
    }};
    ($input:expr, $scratch:expr, $ty:ident, $variant:ident, $fmt:ty, $literal_fmt:ty, $literal_bytes:expr, if |$remaining_len:ident| $pred:expr) => {{
        let source = *$input;
        let expected: &[u8] = $literal_bytes;
        if $crate::composite::match_literal::<$literal_fmt>($input, $scratch, expected).map_err($crate::StructError::from)? {
            let $remaining_len = $input.len();
            if $pred {
                return $crate::__private::decode_variant::<_, _, $fmt, _>($input, $scratch, $ty::$variant);
            }
            *$input = source;
        }
    }};
}

#[macro_export]
macro_rules! tagged_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > {
            $(
                _: $literal_fmt:ty = $literal_bytes:expr => $variant:ident($fmt:ty) $(if |$remaining_len:ident| $pred:expr)?
            ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => {
                            let expected: &[u8] = $literal_bytes;
                            <$literal_fmt as $crate::ScalarFmt>::encode(output, scratch, expected).map_err($crate::StructError::from)?;
                            <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner)
                        }
                    )+
                }
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $(
                    $crate::__finfmt_tagged_const_decode_arm!(
                        input,
                        scratch,
                        $ty,
                        $variant,
                        $fmt,
                        $literal_fmt,
                        $literal_bytes
                        $(, if |$remaining_len| $pred)?
                    );
                )+
                $crate::__private::cold_path();
                Err($crate::Error::Invalid.into())
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:ident {
            $(
                _: $literal_fmt:ty = $literal_bytes:expr => $variant:ident($fmt:ty) $(if |$remaining_len:ident| $pred:expr)?
            ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => {
                            let expected: &[u8] = $literal_bytes;
                            <$literal_fmt as $crate::ScalarFmt>::encode(output, scratch, expected).map_err($crate::StructError::from)?;
                            <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner)
                        }
                    )+
                }
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                $(
                    $crate::__finfmt_tagged_const_decode_arm!(
                        input,
                        scratch,
                        $ty,
                        $variant,
                        $fmt,
                        $literal_fmt,
                        $literal_bytes
                        $(, if |$remaining_len| $pred)?
                    );
                )+
                $crate::__private::cold_path();
                Err($crate::Error::Invalid.into())
            }
        }
    };
}

#[macro_export]
macro_rules! choice_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime >, $selector_ty:ty {
            $(
                $variant:ident($fmt:ty) if |$selector:ident| $pred:expr
            ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::ContextFmt<$ty<$lt>, $selector_ty> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_with(
                output: &mut &mut [u8],
                scratch: &mut &mut [u8],
                context: &$selector_ty,
                value: &$ty<$lt>,
            ) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => {
                            let $selector = context;
                            if !($pred) {
                                $crate::__private::cold_path();
                                return Err($crate::Error::Invalid.into());
                            }
                            <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner)
                        }
                    )+
                }
            }

            #[inline(always)]
            fn decode_with<'de>(
                input: &mut &'de [u8],
                scratch: &mut &'de mut [u8],
                context: &$selector_ty,
            ) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $(
                    {
                        let $selector = context;
                        if $pred {
                            return $crate::__private::decode_variant::<_, _, $fmt, _>(input, scratch, $ty::$variant);
                        }
                    }
                )+
                $crate::__private::cold_path();
                Err($crate::Error::Invalid.into())
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:ident, $selector_ty:ty {
            $(
                $variant:ident($fmt:ty) if |$selector:ident| $pred:expr
            ),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::ContextFmt<$ty, $selector_ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_with(
                output: &mut &mut [u8],
                scratch: &mut &mut [u8],
                context: &$selector_ty,
                value: &$ty,
            ) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => {
                            let $selector = context;
                            if !($pred) {
                                $crate::__private::cold_path();
                                return Err($crate::Error::Invalid.into());
                            }
                            <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner)
                        }
                    )+
                }
            }

            #[inline(always)]
            fn decode_with<'a>(
                input: &mut &'a [u8],
                scratch: &mut &'a mut [u8],
                context: &$selector_ty,
            ) -> Result<$ty, $crate::StructError> {
                $(
                    {
                        let $selector = context;
                        if $pred {
                            return $crate::__private::decode_variant::<_, _, $fmt, _>(input, scratch, $ty::$variant);
                        }
                    }
                )+
                $crate::__private::cold_path();
                Err($crate::Error::Invalid.into())
            }
        }
    };
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_union_decode_arms {
    (
        $input:expr,
        $scratch:expr,
        $ty:ident;
        $variant:ident($fmt:ty)
    ) => {{
        let source = *$input;
        match <$fmt as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch) {
            Ok(inner) => Ok($ty::$variant(inner)),
            Err(error) => {
                *$input = source;
                Err(error)
            }
        }
    }};
    (
        $input:expr,
        $scratch:expr,
        $ty:ident;
        $variant:ident($fmt:ty) $(, $($rest:tt)*)?
    ) => {{
        let source = *$input;
        match <$fmt as $crate::composite::CompositeFmt<_>>::decode_cursor($input, $scratch) {
            Ok(inner) => Ok($ty::$variant(inner)),
            Err(error) if $crate::composite::should_retry_union(error.kind) => {
                *$input = source;
                $crate::__finfmt_union_decode_arms!($input, $scratch, $ty; $($($rest)*)?)
            }
            Err(error) => {
                *$input = source;
                Err(error)
            }
        }
    }};
}

#[macro_export]
#[doc(hidden)]
macro_rules! __finfmt_union_decode_owned_arms {
    (
        $input:expr,
        $scratch:expr,
        $scratch_source:ident,
        $source:ident,
        $ty:ident;
        $variant:ident($fmt:ty)
    ) => {{
        let mut arm_input = $source;
        let decoded = {
            let arm_scratch = &mut *$scratch_source;
            $crate::composite::decode_owned_struct::<_, $fmt>(&mut arm_input, arm_scratch)
        };
        match decoded {
            Ok(inner) => {
                $crate::composite::advance_input($input, $source.len() - arm_input.len()).map_err($crate::StructError::from)?;
                *$scratch = $scratch_source;
                Ok($ty::$variant(inner))
            }
            Err(error) => {
                *$scratch = $scratch_source;
                Err(error)
            }
        }
    }};
    (
        $input:expr,
        $scratch:expr,
        $scratch_source:ident,
        $source:ident,
        $ty:ident;
        $variant:ident($fmt:ty) $(, $($rest:tt)*)?
    ) => {{
        let mut arm_input = $source;
        let decoded = {
            let arm_scratch = &mut *$scratch_source;
            $crate::composite::decode_owned_struct::<_, $fmt>(&mut arm_input, arm_scratch)
        };
        match decoded {
            Ok(inner) => {
                $crate::composite::advance_input($input, $source.len() - arm_input.len()).map_err($crate::StructError::from)?;
                *$scratch = $scratch_source;
                Ok($ty::$variant(inner))
            }
            Err(error) if $crate::composite::should_retry_union(error.kind) => {
                $crate::__finfmt_union_decode_owned_arms!(
                    $input,
                    $scratch,
                    $scratch_source,
                    $source,
                    $ty;
                    $($($rest)*)?
                )
            }
            Err(error) => {
                *$scratch = $scratch_source;
                Err(error)
            }
        }
    }};
}

#[macro_export]
macro_rules! union_format {
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for<$lt:lifetime> $ty:ident < $ty_lt:lifetime > {
            $($variant:ident($fmt:ty)),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl<$lt> $crate::composite::CompositeFmt<$ty<$lt>> for $name {
            type Decoded<'de> = $ty<'de>;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty<$lt>) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner),
                    )+
                }
            }

            #[inline(always)]
            fn decode_cursor<'de>(input: &mut &'de [u8], scratch: &mut &'de mut [u8]) -> Result<Self::Decoded<'de>, $crate::StructError> {
                $crate::__finfmt_union_decode_arms!(input, scratch, $ty; $($variant($fmt)),+)
            }
        }
    };
    (
        $(#[$attr:meta])*
        $vis:vis struct $name:ident for $ty:ident {
            $($variant:ident($fmt:ty)),+ $(,)?
        }
    ) => {
        $(#[$attr])*
        $vis struct $name;

        impl $crate::composite::CompositeFmt<$ty> for $name {
            type Decoded<'de> = $ty;

            #[inline(always)]
            fn encode_cursor(output: &mut &mut [u8], scratch: &mut &mut [u8], value: &$ty) -> Result<(), $crate::StructError> {
                match value {
                    $(
                        $ty::$variant(inner) => <$fmt as $crate::composite::CompositeFmt<_>>::encode_cursor(output, scratch, inner),
                    )+
                }
            }

            #[inline(always)]
            fn decode_cursor<'a>(input: &mut &'a [u8], scratch: &mut &'a mut [u8]) -> Result<$ty, $crate::StructError> {
                let source = *input;
                let scratch_source = core::mem::take(scratch);
                $crate::__finfmt_union_decode_owned_arms!(input, scratch, scratch_source, source, $ty; $($variant($fmt)),+)
            }
        }
    };
}
