/// Macro to define a `When` enum with exactly three variants:
/// Never, Auto, Always. Preserves enum and variant metadata.
///
/// # Example
/// ```
/// use cli_boilerplate_automation::define_when;
///
/// define_when! {
///    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
///    #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
///    pub enum When {
///        #[cfg_attr(feature = "serde", serde(alias = "false", alias = "never"))]
///        Never,
///        #[default]
///        #[cfg_attr(feature = "serde", serde(alias = "auto"))]
///        Auto,
///        #[cfg_attr(feature = "serde", serde(alias = "true", alias = "always"))]
///        Always
///   }
/// }
/// ```
#[macro_export]
macro_rules! define_when {
        (
            $(#[$enum_meta:meta])*
            $vis:vis enum $name:ident {
                $(#[$never_meta:meta])* $never:ident,
                $(#[$auto_meta:meta])* $auto:ident,
                $(#[$always_meta:meta])* $always:ident$(,)?
            }
        ) => {
            $(#[$enum_meta])*
            $vis enum $name {
                $(#[$never_meta])* $never,
                $(#[$auto_meta])* $auto,
                $(#[$always_meta])* $always,
            }

            impl From<$name> for Option<bool> {
                fn from(w: $name) -> Self {
                    match w {
                        $name::$never => Some(false),
                        $name::$always => Some(true),
                        $name::$auto => None,
                    }
                }
            }

            impl From<Option<bool>> for $name {
                fn from(opt: Option<bool>) -> Self {
                    match opt {
                        Some(true) => $name::$always,
                        Some(false) => $name::$never,
                        None => $name::$auto,
                    }
                }
            }

            impl From<bool> for $name {
                fn from(b: bool) -> Self {
                    if b { $name::$always } else { $name::$never }
                }
            }

            impl $name {
                pub fn unwrap_or(self, default: bool) -> bool {
                    match self {
                        $name::$never => false,
                        $name::$always => true,
                        $name::$auto => default,
                    }
                }

                pub fn unwrap_or_else<F>(self, f: F) -> bool
                where F: FnOnce() -> bool
                {
                    match self {
                        $name::$never => false,
                        $name::$always => true,
                        $name::$auto => f(),
                    }
                }

                pub fn is_default(&self) -> bool { matches!(self, $name::$auto) }
                pub fn is_always(&self) -> bool { matches!(self, $name::$always) }
                pub fn is_never(&self) -> bool { matches!(self, $name::$never) }
                pub fn is_none(&self) -> bool { matches!(self, $name::$auto) }
            }
        }
    }

/// Macro to define an `Either` enum with exactly two variants: Left, Right.
///```rust
/// use cli_boilerplate_automation::define_either;
///
/// define_either! {
///     #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
///     #[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
///     pub enum Either<L, R = L> {
///         Left,
///         Right
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_either {
        (
            $(#[$enum_meta:meta])*
            $vis:vis enum $name:ident<$l:ident, $r:ident = $default_r:ident> {
                $(#[$left_meta:meta])* $left:ident,
                $(#[$right_meta:meta])* $right:ident$(,)?
            }
        ) => {
            $(#[$enum_meta])*
            $vis enum $name<$l, $r = $default_r> {
                $(#[$left_meta])* $left($l),
                $(#[$right_meta])* $right($r),
            }

            impl<$l, $r> $name<$l, $r> {
                pub fn right(self) -> Option<$r> {
                    match self {
                        $name::$right(r) => Some(r),
                        $name::$left(_) => None,
                    }
                }

                pub fn left(self) -> Option<$l> {
                    match self {
                        $name::$left(l) => Some(l),
                        $name::$right(_) => None,
                    }
                }

                pub fn _left(self) -> $l { self.left().unwrap() }
                pub fn _right(self) -> $r { self.right().unwrap() }

                pub fn as_ref(&self) -> $name<&$l, &$r> {
                    match self {
                        $name::$left(l) => $name::$left(l),
                        $name::$right(r) => $name::$right(r),
                    }
                }

                pub fn as_mut(&mut self) -> $name<&mut $l, &mut $r> {
                    match self {
                        $name::$left(l) => $name::$left(l),
                        $name::$right(r) => $name::$right(r),
                    }
                }

                pub fn map_left<F, LL>(self, f: F) -> $name<LL, $r>
                where F: FnOnce($l) -> LL
                {
                    match self {
                        $name::$left(l) => $name::$left(f(l)),
                        $name::$right(r) => $name::$right(r),
                    }
                }

                pub fn map_right<F, RR>(self, f: F) -> $name<$l, RR>
                where F: FnOnce($r) -> RR
                {
                    match self {
                        $name::$left(l) => $name::$left(l),
                        $name::$right(r) => $name::$right(f(r)),
                    }
                }

                pub fn _map_right<F>(self, f: F) -> $l
                where F: FnOnce($r) -> $l
                {
                    match self {
                        $name::$left(l) => l,
                        $name::$right(r) => f(r),
                    }
                }

                pub fn is_left(&self) -> bool { matches!(self, $name::$left(_)) }

                pub fn swap(self) -> $name<$r, $l> {
                    match self {
                        $name::$left(x) => $name::$right(x),
                        $name::$right(x) => $name::$left(x),
                    }
                }

                pub fn into_result(self) -> Result<$l, $r> {
                    match self {
                        $name::$left(l) => Ok(l),
                        $name::$right(r) => Err(r),
                    }
                }
            }
        }
    }
