#[macro_export]
/// Implement a transparent wrapper around an inner type:
///
/// Implements Deref, DerefMut, FromStr, Display, Debug, PartialEq, Serialize, Deserialize.
///
/// # Example
/// ```rust
/// use cli_boilerplate_automation::define_transparent_wrapper;
///
/// #[cfg(feature = "serde")]
/// define_transparent_wrapper!(
///     #[derive(Copy)]
///     Count: u16 = 1
/// );
/// ```
macro_rules! define_transparent_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path $(= $default:expr)?) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq)]
        pub struct $name($(#[$inner_meta])* pub $inner);

        $(
            impl Default for $name {
                fn default() -> Self {
                    $name($default)
                }
            }
        )?

        // Conversions
        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }
        impl From<$inner> for $name {
            fn from(c: $inner) -> Self {
                Self(c)
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
        }
    };
}

#[macro_export]
/// Implement a restricted wrapper around an inner type:
///
/// Implements Deref, PartialEq, Clone, Debug, Serialize.
///
/// # Example
/// ```rust
/// use cli_boilerplate_automation::define_restricted_wrapper;
///
/// #[cfg(feature = "serde")] {
///     define_restricted_wrapper!(Percentage: u16 = 100);
///     impl Percentage {
///         pub fn new(value: u16) -> Self {
///             if value <= 100 { Self(value) } else { Self(100) }
///         }
///     }
/// }
///
/// ```
macro_rules! define_restricted_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path $(= $default:expr)?) => {
        $(#[$meta])*
        #[derive(Debug, PartialEq)]
        pub struct $name($(#[$inner_meta])* $inner);

        impl $name {
            pub fn inner(&self) -> $inner {
                self.0.clone()
            }
        }

        $(
            impl Default for $name {
                fn default() -> Self {
                    $name($default)
                }
            }
        )?

        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;
            fn deref(&self) -> &Self::Target { &self.0 }
        }
    };
}

#[macro_export]
/// Implement a wrapper around a container type (i.e. HashMap).
/// Implements the Deref, DerefMut, Default and IntoIterator/FromIterator traits and the new function.
///
/// ```rust
/// use cli_boilerplate_automation::define_collection_wrapper;
/// pub struct Module {};
/// define_collection_wrapper!(
///     #[cfg_attr(feature = "serde", derive(Debug, serde::Serialize, serde::Deserialize))]
///     Modules: std::collections::HashMap<String, Module>
/// );
/// ```
macro_rules! define_collection_wrapper {
    ($(#[$meta:meta])* $name:ident: $(#[$inner_meta:meta])* $inner:path) => {
        $(#[$meta])*
        pub struct $name($(#[$inner_meta])* $inner);

        impl $name {
            pub fn new() -> Self {
                Self(<$inner>::new())
            }
        }

        impl std::ops::Deref for $name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self(<$inner>::new())
            }
        }

        impl From<$name> for $inner {
            fn from(c: $name) -> Self {
                c.0
            }
        }
        impl From<$inner> for $name {
            fn from(c: $inner) -> Self {
                Self(c)
            }
        }

        impl IntoIterator for $name {
            type Item = <$inner as IntoIterator>::Item;
            type IntoIter = <$inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $name {
            type Item = <&'a $inner as IntoIterator>::Item;
            type IntoIter = <&'a $inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                (&self.0).into_iter()
            }
        }

        // impl<'a> IntoIterator for &'a mut $name {
        //     type Item = <&'a mut $inner as IntoIterator>::Item;
        //     type IntoIter = <&'a mut $inner as IntoIterator>::IntoIter;

        //     fn into_iter(self) -> Self::IntoIter {
        //         (&mut self.0).into_iter()
        //     }
        // }

        impl FromIterator<<$inner as IntoIterator>::Item> for $name {
            fn from_iter<I: IntoIterator<Item = <$inner as IntoIterator>::Item>>(iter: I) -> Self {
                Self(iter.into_iter().collect())
            }
        }
    };
}

// would be kinda neat if rust supported pub ident: value, and value defines the type as well as sets the default.
#[macro_export]
macro_rules! define_const_default {
    (
        $(#[$meta:meta])*
        $vis:vis struct $Name:ident {
            $(
                $(#[$field_meta:meta])*
                $ivis:vis $field:ident : $ty:ty $(= $default:expr)?
            ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        $vis struct $Name {
            $(
                $(#[$field_meta])*
                $ivis $field: $ty,
            )*
        }

        impl $Name {
            pub const DEFAULT: Self = Self {
                $(
                    $field: define_const_default!(@default $ty $(, $default)?),
                )*
            };
        }

        impl Default for $Name {
            fn default() -> Self {
                Self::DEFAULT
            }
        }
    };

    (@default $ty:ty, $default:expr) => {
        $default
    };

    (@default $ty:ty) => {
        <$ty>::DEFAULT
    };
}

#[macro_export]
macro_rules! auto_impl {
    ($name:ty : $($trait:ident $(=> $target:ty)? $(= $val:expr)?);+ $(;)?) => {
        $(
            auto_impl!(@dispatch $name, $trait $(, $target)? $(, $val)?);
        )+
    };

    // ===== dispatch =====

    (@dispatch $name:ty, Default, $val:expr) => {
        impl Default for $name {
            fn default() -> Self {
                Self($val)
            }
        }
    };

    (@dispatch $name:ty, $trait:ident) => {
        auto_impl!(@impl $name, $trait, <Self as std::ops::Deref>::Target);
    };

    // explicit target
    (@dispatch $name:ty, $trait:ident, $target:ty) => {
        auto_impl!(@impl $name, $trait, $target);
    };

    // ===== impls (single inner path) =====

    (@impl $name:ty, Deref, $target:ty) => {
        impl std::ops::Deref for $name {
            type Target = $target;
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
    };

    (@impl $name:ty, DerefMut, $target:ty) => {
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut $target {
                &mut self.0
            }
        }
    };

    (@impl $name:ty, DerefMut) => {
        impl std::ops::DerefMut for $name {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.0
            }
        }
    };


    (@impl $name:ty, From, $inner:ty) => {
        impl From<$inner> for $name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }
    };

    (@impl $name:ty, Into, $inner:ty) => {
        impl Into<$inner> for $name {
            fn into(self) -> $inner {
                self.0
            }
        }
    };

    (@impl $name:ty, Display, $inner:ty) => {
        impl std::fmt::Display for $name
        where
        $inner: std::fmt::Display,
        {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }
    };

    (@impl $name:ty, FromStr, $inner:ty) => {
        impl std::str::FromStr for $name
        where
        $inner: std::str::FromStr,
        {
            type Err = <$inner as std::str::FromStr>::Err;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                <$inner as std::str::FromStr>::from_str(s).map(Self)
            }
        }
    };

    (@impl $name:ty, IntoIterator, $inner:ty) => {
        impl IntoIterator for $name
        where
        $inner: IntoIterator,
        {
            type Item = <$inner as IntoIterator>::Item;
            type IntoIter = <$inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        impl<'a> IntoIterator for &'a $name
        where
        &'a $inner: IntoIterator,
        {
            type Item = <&'a $inner as IntoIterator>::Item;
            type IntoIter = <&'a $inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                (&self.0).into_iter()
            }
        }

        impl<'a> IntoIterator for &'a mut $name
        where
        &'a mut $inner: IntoIterator,
        {
            type Item = <&'a mut $inner as IntoIterator>::Item;
            type IntoIter = <&'a mut $inner as IntoIterator>::IntoIter;

            fn into_iter(self) -> Self::IntoIter {
                (&mut self.0).into_iter()
            }
        }
    };

    (@impl $name:ty, FromIterator, $inner:ty) => {
        impl<T> std::iter::FromIterator<T> for $name
        where
        $inner: std::iter::FromIterator<T>,
        {
            fn from_iter<I: IntoIterator<Item = T>>(iter: I) -> Self {
                Self(<$inner as std::iter::FromIterator<T>>::from_iter(iter))
            }
        }
    };
}
