use serde::{Deserialize, Deserializer, Serialize, Serializer, de::IntoDeserializer};

pub mod one_or_many {
    use super::*;

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Helper<I> {
        Single(I),
        Multiple(Vec<I>),
    }

    pub fn deserialize<'de, D, T, I>(deserializer: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: FromIterator<I>,
        I: Deserialize<'de>,
    {
        match Helper::deserialize(deserializer)? {
            Helper::Single(item) => Ok(std::iter::once(item).collect()),
            Helper::Multiple(items) => Ok(items.into_iter().collect()),
        }
    }

    pub fn serialize<S, T, I>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
        for<'a> &'a T: IntoIterator<Item = &'a I>,
        I: Serialize,
    {
        let mut iter = value.into_iter();

        match iter.next() {
            None => serializer.collect_seq(std::iter::empty::<&I>()),
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

pub mod through_string {
    use serde::{Deserialize, Deserializer, Serializer, de};
    use std::fmt::Display;
    use std::str::FromStr;

    pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
    where
        T: Display,
        S: Serializer,
    {
        serializer.serialize_str(&value.to_string())
    }

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

pub mod empty_string_as_none {
    use serde::{Deserialize, Deserializer, Serialize, Serializer, de::IntoDeserializer};

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
        camelcase_normalized,
        deserialize_with_transform,
        |s: String| camel_case(s)
    );
    defn_transform!(
        as_option,
        deserialize_option_with_transform => Option<T>,
        |s: String| s
    );
    defn_transform!(
        uppercase_normalized_option,
        deserialize_option_with_transform => Option<T>,
        |s: String| s.to_ascii_uppercase()
    );
    defn_transform!(
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
