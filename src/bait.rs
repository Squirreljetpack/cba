use std::fmt::Display;

use crate::StringError;

#[easy_ext::ext(MaybeExt)]
pub impl<T> T
where
    T: Sized,
{
    /// Merge from maybe by taking.
    fn _take(&mut self, maybe: Option<T>) {
        if let Some(v) = maybe {
            *self = v;
        }
    }

    /// Merge from maybe by cloning.
    fn _clone(&mut self, maybe: &Option<T>) -> T
    where
        T: Clone,
    {
        if let Some(v) = maybe {
            *self = v.clone();
        }
        self.clone()
    }
}

// this would be more useful if try blocks exposed their "other" type so we could call into on e
#[easy_ext::ext(ResultExt)]
pub impl<T, E> Result<T, E> {
    /// cast result types.
    ///
    /// # Note
    /// Difficult to used with ?, more useful in return statements.
    #[inline]
    fn cast<F, S>(self) -> Result<S, F>
    where
        F: From<E>,
        S: From<T>,
    {
        match self {
            Ok(s) => Ok(s.into()),
            Err(e) => Err(e.into()),
        }
    }

    /// cast result to StringError.
    #[inline]
    fn cast_(self) -> Result<T, StringError>
    where
        E: Display,
    {
        match self {
            Ok(s) => Ok(s),
            Err(e) => Err(e.to_string().into()),
        }
    }

    // debated between prefix and prefix_err, chose the first because anyhow just calls it context.
    /// Convert Err(e) to the string '{prefix}: {e}'
    #[inline]
    fn prefix(self, prefix: impl Display) -> Result<T, StringError>
    where
        E: std::fmt::Display,
    {
        match self {
            Ok(val) => Ok(val),
            Err(e) => Err(format!("{prefix}: {e}").into()),
        }
    }

    // logging

    /// Log the error.
    ///
    /// # Notes
    /// Can be used in conjunction with [`prefix`](ResultExt::prefix) to add context.
    /// See also: [`crate::bog::BogOkExt`] to bog instead of log.
    #[inline]
    fn elog(self) -> Result<T, E>
    where
        E: Display,
    {
        self.map_err(|e| {
            log::error!("{e}");
            e
        })
    }

    /// [`elog`](ResultExt::elog), then consume the error.
    ///
    /// # Notes
    /// Can be used in conjunction with [`prefix`](ResultExt::prefix) to add context.
    /// See also: [`crate::bog::BogOkExt`] to bog instead of log.
    #[inline]
    fn _elog(self) -> Option<T>
    where
        E: Display,
    {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                log::error!("{e}");
                None
            }
        }
    }

    /// Log the error as a warning.
    ///
    /// # Notes
    /// Can be used in conjunction with [`prefix`](ResultExt::prefix) to add context.
    /// See also: [`crate::bog::BogOkExt`] to bog instead of log.
    #[inline]
    fn wlog(self) -> Result<T, E>
    where
        E: Display,
    {
        self.map_err(|e| {
            log::warn!("{e}");
            e
        })
    }

    /// [`wlog`](ResultExt::wlog), then consume the error.
    ///
    /// # Notes
    /// Can be used in conjunction with [`prefix`](ResultExt::prefix) to add context.
    /// See also: [`crate::bog::BogOkExt`] to bog instead of log.
    #[inline]
    fn _wlog(self) -> Option<T>
    where
        E: Display,
    {
        match self {
            Ok(x) => Some(x),
            Err(e) => {
                log::warn!("{e}");
                None
            }
        }
    }
}

#[easy_ext::ext(OptionExt)]
pub impl<T> Option<T> {
    /// Unwrap or exit
    fn or_exit(self) -> T {
        match self {
            Some(val) => val,
            None => {
                std::process::exit(1);
            }
        }
    }

    /// Unwrap or log and exit
    fn _elog(self, s: &str) -> T {
        if self.is_none() {
            log::error!("{s}");
        }
        self.or_exit()
    }

    /// Log the error if None, then transform to a Result.
    fn elog<E: Display>(self, err: E) -> Result<T, E> {
        match self {
            Some(v) => Ok(v),
            None => {
                log::error!("{err}");
                Err(err)
            }
        }
    }

    /// Log the error as a warning if None, then transform to a Result.
    fn wlog<E: Display>(self, err: E) -> Result<T, E> {
        match self {
            Some(v) => Ok(v),
            None => {
                log::warn!("{err}");
                Err(err)
            }
        }
    }
}

#[easy_ext::ext(BoolExt)]
pub impl bool {
    #[inline]
    fn ternary<U>(&self, and: U, or: U) -> U {
        self.then_some(and).unwrap_or(or)
    }

    #[inline]
    fn and_then<U>(&self, f: impl FnOnce() -> Option<U>) -> Option<U> {
        if *self { f() } else { None }
    }

    #[inline]
    fn neg(&self) -> Self {
        !*self
    }

    #[inline]
    fn or_exit(&self) {
        if !self {
            std::process::exit(1)
        }
    }
}

// ---------------------------------

use std::sync::{Mutex, MutexGuard};
#[easy_ext::ext(MutexExt)]
pub impl<T> Mutex<T> {
    fn _lock(&self) -> MutexGuard<'_, T> {
        match self.lock() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

// ---------------------------------

#[easy_ext::ext(TransformExt)]
pub impl<T> T {
    fn transform<Q>(self, transform: impl FnOnce(Self) -> Q) -> Q
    where
        T: Sized,
    {
        transform(self)
    }

    /// # Example
    ///
    /// ```rust,ignore
    /// Table::new(rows, widths.to_vec())
    ///     .block(block)
    ///     .transform_if(
    ///         true,
    ///         |t| t.style(style),
    ///     )
    ///```
    fn transform_if(self, condition: bool, transform: impl FnOnce(Self) -> Self) -> Self
    where
        T: Sized,
    {
        if condition { transform(self) } else { self }
    }

    fn modify<Q>(mut self, modify: impl FnOnce(&mut Self) -> Q) -> Self
    where
        T: Sized,
    {
        modify(&mut self);
        self
    }

    /// # Example
    ///
    /// ```rust
    /// use cli_boilerplate_automation::bait::TransformExt;
    ///
    /// true.modify_if(cfg!(debug_assertions), |x| *dbg!(x));
    ///```
    fn modify_if<Q>(mut self, condition: bool, modify: impl FnOnce(&mut Self) -> Q) -> Self
    where
        T: Sized,
    {
        if condition {
            modify(&mut self);
        }
        self
    }

    /// # Example
    ///
    /// ```rust
    /// use cli_boilerplate_automation::bait::TransformExt;
    ///
    /// let mut v = 0usize;
    /// if !v.cmp_exch(&mut 0, 1) {
    ///     unreachable!();
    /// }
    /// assert_eq!(v, 1);
    ///```
    fn cmp_exch<'a, E>(&'a mut self, expected: E, new: T) -> bool
    where
        &'a mut T: PartialEq<E>,
    {
        if self == expected {
            *self = new;
            true
        } else {
            false
        }
    }

    /// # Example
    ///
    /// ```rust
    /// use cli_boilerplate_automation::bait::TransformExt;
    ///
    /// true.modify_if(cfg!(debug_assertions), |x| *dbg!(x));
    ///```
    fn cmp_replace(&mut self, new: T) -> bool
    where
        T: PartialEq,
    {
        let changed = *self != new;
        if changed {
            *self = new;
        }
        changed
    }

    fn dbg(self) -> Self
    where
        T: std::fmt::Debug,
    {
        dbg!(&self);
        self
    }
}
