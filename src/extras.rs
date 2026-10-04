//! Optional helpers outside the core format interfaces.

/// Serde `with` module that writes the unknown tags of a TLV record's
/// `extras` map beside the record's named fields in JSON, each key spelled
/// `t{tag}_unknown`, such as `t9F03_unknown`:
///
/// ```
/// use std::collections::BTreeMap;
///
/// #[derive(serde::Serialize, serde::Deserialize)]
/// struct Emv {
///     amount: String,
///     #[serde(flatten, with = "finfmt::extras::unknown_tag_keys")]
///     extras: BTreeMap<String, String>,
/// }
///
/// let emv = Emv { amount: "12".into(), extras: [("9F03".into(), "00".into())].into() };
/// let json = serde_json::to_string(&emv).unwrap();
/// assert_eq!(json, r#"{"amount":"12","t9F03_unknown":"00"}"#);
/// let back: Emv = serde_json::from_str(&json).unwrap();
/// assert_eq!(back.extras, emv.extras);
/// // Any other leftover key is an unknown field.
/// assert!(serde_json::from_str::<Emv>(r#"{"amount":"12","typo":"x"}"#).is_err());
/// ```
///
/// The map itself, and so the wire and any Rust code using it, keeps the
/// plain tags.
#[cfg(feature = "serde")]
pub mod unknown_tag_keys {
    use core::fmt::{self, Display};
    use core::marker::PhantomData;
    use core::str::FromStr;

    use serde::de::{Error, MapAccess, Visitor};
    use serde::ser::SerializeMap;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    struct Key<'a, K>(&'a K);

    impl<K: Display> Serialize for Key<'_, K> {
        fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            serializer.collect_str(&format_args!("t{}_unknown", self.0))
        }
    }

    pub fn serialize<S, M, K, V>(map: &M, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        for<'a> &'a M: IntoIterator<Item = (&'a K, &'a V)>,
        K: Display + 'static,
        V: Serialize + 'static,
    {
        let mut out = serializer.serialize_map(None)?;
        for (key, value) in map {
            out.serialize_entry(&Key(key), value)?;
        }
        out.end()
    }

    pub fn deserialize<'de, D, M, K, V>(deserializer: D) -> Result<M, D::Error>
    where
        D: Deserializer<'de>,
        M: Default + Extend<(K, V)>,
        K: FromStr,
        V: Deserialize<'de>,
    {
        struct Entries<M, K, V>(PhantomData<(M, K, V)>);

        impl<'de, M, K, V> Visitor<'de> for Entries<M, K, V>
        where
            M: Default + Extend<(K, V)>,
            K: FromStr,
            V: Deserialize<'de>,
        {
            type Value = M;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("unknown tags keyed like t9F03_unknown")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<M, A::Error> {
                let mut map = M::default();
                while let Some(key) = access.next_key::<String>()? {
                    let tag = key
                        .strip_prefix('t')
                        .and_then(|rest| rest.strip_suffix("_unknown"))
                        .and_then(|tag| tag.parse::<K>().ok())
                        .ok_or_else(|| A::Error::custom(format_args!("unknown field `{key}`")))?;
                    map.extend(core::iter::once((tag, access.next_value::<V>()?)));
                }
                Ok(map)
            }
        }

        deserializer.deserialize_map(Entries(PhantomData))
    }
}
