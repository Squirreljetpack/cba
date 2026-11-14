use serde::{Deserialize, Deserializer, Serialize, Serializer, de::IntoDeserializer};

/// Accept a single value or a list of values for the same field.
///
/// Sequences always deserialize as a collection of `I`, so `I` must
/// deserialize from a non-sequence value.
pub mod one_or_many {
    use std::marker::PhantomData;

    use serde::de::{
        self, EnumAccess, MapAccess, SeqAccess, Visitor,
        value::{
            BytesDeserializer, EnumAccessDeserializer, MapAccessDeserializer, SeqAccessDeserializer,
        },
    };

    use super::*;

    /// Deserialize one value or many into a collection.
    pub fn deserialize<'de, D, T, I>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromIterator<I>,
        I: Deserialize<'de>,
    {
        fn single<I, T: FromIterator<I>, E>(res: Result<I, E>) -> Result<T, E> {
            res.map(|item| std::iter::once(item).collect())
        }

        struct OneOrManyVisitor<T, I>(PhantomData<(T, I)>);

        macro_rules! forward_single {
            ($($method:ident($ty:ty)),* $(,)?) => {
                $(
                    fn $method<E>(self, v: $ty) -> Result<Self::Value, E>
                    where
                        E: de::Error,
                    {
                        single(I::deserialize(v.into_deserializer()))
                    }
                )*
            };
        }

        impl<'de, T, I> Visitor<'de> for OneOrManyVisitor<T, I>
        where
            T: FromIterator<I>,
            I: Deserialize<'de>,
        {
            type Value = T;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a single value or a sequence of values")
            }

            forward_single!(
                visit_bool(bool),
                visit_i64(i64),
                visit_u64(u64),
                visit_f64(f64),
                visit_char(char),
                visit_str(&str),
                visit_string(String),
            );

            fn visit_bytes<E>(self, v: &[u8]) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                single(I::deserialize(BytesDeserializer::new(v)))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                single(I::deserialize(().into_deserializer()))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                single(I::deserialize(().into_deserializer()))
            }

            fn visit_some<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
            where
                D2: Deserializer<'de>,
            {
                single(I::deserialize(deserializer))
            }

            fn visit_newtype_struct<D2>(self, deserializer: D2) -> Result<Self::Value, D2::Error>
            where
                D2: Deserializer<'de>,
            {
                single(I::deserialize(deserializer))
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                single(I::deserialize(MapAccessDeserializer::new(map)))
            }

            fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
            where
                A: EnumAccess<'de>,
            {
                single(I::deserialize(EnumAccessDeserializer::new(data)))
            }

            fn visit_seq<A>(self, seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                Vec::<I>::deserialize(SeqAccessDeserializer::new(seq))
                    .map(|items| items.into_iter().collect())
            }
        }

        deserializer.deserialize_any(OneOrManyVisitor(PhantomData))
    }

    /// Serialize a collection holding one element as that element.
    pub fn serialize<T, S>(value: T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        T: IntoIterator + Copy,
        T::Item: Serialize,
    {
        let mut iter = value.into_iter();

        match iter.next() {
            None => serializer.collect_seq(value),
            Some(first) => {
                if iter.next().is_none() {
                    // single element → serialize as scalar
                    first.serialize(serializer)
                } else {
                    // multiple → serialize as array
                    serializer.collect_seq(value)
                }
            }
        }
    }
}

/// Convert through the string form of a type.
pub mod through_string {
    use serde::{Deserialize, Deserializer, Serializer, de};
    use std::fmt::Display;
    use std::str::FromStr;

    /// Serialize as the `Display` string.
    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

    /// Deserialize by parsing the string with `FromStr`.
    pub fn deserialize<'de, T, D>(deserializer: D) -> Result<T, D::Error>
    where
        T: FromStr,
        T::Err: Display,
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        T::from_str(&s).map_err(de::Error::custom)
    }
}

/// Read `""` as `None` for optional fields.
pub mod empty_string_as_none {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::IntoDeserializer};

    /// `Some` as its own value, `None` as `""`.
    pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Serialize,
        S: Serializer,
    {
        match value {
            Some(value) => value.serialize(serializer),

            None => serializer.serialize_str(""),
        }
    }

    /// Missing or empty input gives `None`.
    pub fn deserialize<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
    {
        let s = Option::<String>::deserialize(deserializer)?;

        match s {
            None => Ok(None),

            Some(s) if s.is_empty() => Ok(None),

            Some(s) => T::deserialize(s.into_deserializer()).map(Some),
        }
    }
}

/// Read any value into `Err`, for fields typed as `Result`.
pub mod result_as_err {
    use serde::{Deserialize, Deserializer};

    /// Wrap the deserialized value in `Err`.
    pub fn deserialize<'de, D, E>(d: D) -> Result<Result<(), E>, D::Error>
    where
        D: Deserializer<'de>,
        E: Deserialize<'de>,
    {
        Ok(Err(E::deserialize(d)?))
    }
}

/// Deserialize through an intermediate type and a transform.
pub mod transform {
    pub use super::*;

    /// Applying a transformation before deserializing.
    pub fn deserialize_with_transform<'de, D, T, F, I, S>(
        deserializer: D,
        transform: F,
    ) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
        F: FnOnce(I) -> S,
        I: Deserialize<'de>,
        S: IntoDeserializer<'de, D::Error>,
    {
        let s = I::deserialize(deserializer)?;
        let s = transform(s);
        T::deserialize(serde::de::IntoDeserializer::into_deserializer(s))
    }
    /// [`deserialize_with_transform`] for optional fields.
    pub fn deserialize_option_with_transform<'de, D, T, F, I, S>(
        deserializer: D,
        transform: F,
    ) -> Result<Option<T>, D::Error>
    where
        D: Deserializer<'de>,
        T: Deserialize<'de>,
        F: FnOnce(I) -> S,
        I: Deserialize<'de>,
        S: IntoDeserializer<'de, D::Error>,
    {
        let s = Option::<I>::deserialize(deserializer)?.map(transform);
        if let Some(s) = s {
            let ret = T::deserialize(serde::de::IntoDeserializer::into_deserializer(s))?;
            Ok(Some(ret))
        } else {
            Ok(None)
        }
    }

    macro_rules! defn_transform {
        ($(#[$meta:meta])* $fn_name:ident, $deserialize_fn:ident, $transform:expr) => {
            $(#[$meta])*
            pub fn $fn_name<'de, D, T>(deserializer: D) -> Result<T, D::Error>
            where
            D: serde::Deserializer<'de>,
            T: serde::Deserialize<'de>,
            {
                $deserialize_fn(deserializer, $transform)
            }
        };

        ($(#[$meta:meta])* $fn_name:ident, $deserialize_fn:ident => $type:ty, $transform:expr) => {
            $(#[$meta])*
            pub fn $fn_name<'de, D, T>(deserializer: D) -> Result<$type, D::Error>
            where
            D: serde::Deserializer<'de>,
            T: serde::Deserialize<'de>,
            {
                $deserialize_fn(deserializer, $transform)
            }
        };
    }

    defn_transform!(
        /// Deserialize a type `T` from a string after applying uppercase.
        /// This is useful for deserializing bitflags.
        /// Mote that this requires the T able to be deserialize from string, which is commonly not the case. In particular, the bitflags must be annotated with #[serde(transparent)].
        uppercase_normalized,
        deserialize_with_transform,
        |s: String| s.to_ascii_uppercase()
    );
    defn_transform!(
        /// Deserialize from a string after camel-case normalization.
        camelcase_normalized,
        deserialize_with_transform,
        |s: String| camel_case(s)
    );
    defn_transform!(
        /// Deserialize `Option<T>` from a plain string.
        as_option,
        deserialize_option_with_transform => Option<T>,
        |s: String| s
    );
    defn_transform!(
        /// Deserialize `Option<T>` from a string after applying uppercase.
        uppercase_normalized_option,
        deserialize_option_with_transform => Option<T>,
        |s: String| s.to_ascii_uppercase()
    );
    defn_transform!(
        /// Deserialize `Option<T>` from a string after camel-case normalization.
        camelcase_normalized_option,
        deserialize_option_with_transform => Option<T>,
        |s: String| camel_case(s)
    );

    #[cfg(feature = "bring")]
    use crate::bring::camel_case;
    #[cfg(not(feature = "bring"))]
    pub fn camel_case(s: String) -> String {
        s.split(|c: char| c == '_' || c.is_whitespace())
            .filter(|p| !p.is_empty())
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    None => String::new(),
                    Some(c) => c.to_ascii_uppercase().to_string() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// implement serde for types with FromStr or Display.
/// # Caveats
/// Doesn't support generics
#[macro_export]
macro_rules! impl_serde {
    ($type:path : $($trait:ident)+) => {
        $(
            impl_serde!(@expand $type, $trait);
        )+
    };

    // ----- dispatch arms -----

    (@expand $type:path, FromStr) => {
        impl<'de> serde::Deserialize<'de> for $type {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
            D: serde::Deserializer<'de>,
            {
                let s: std::borrow::Cow<'de, str> =
                serde::Deserialize::deserialize(deserializer)?;
                <$type as std::str::FromStr>::from_str(&s)
                .map_err(serde::de::Error::custom)
            }
        }
    };

    (@expand $type:path, Display) => {
        impl serde::Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
            S: serde::Serializer,
            {
                serializer.serialize_str(&self.to_string())
            }
        }
    };
}
