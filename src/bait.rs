use std::fmt::Display;

use crate::StringError;

#[easy_ext::ext(MaybeExt)]
pub impl<T> T
where
    T: Sized,
{
    /// Merge from maybe by taking.
    fn take_from(&mut self, maybe: Option<T>) {
        if let Some(v) = maybe {
            *self = v;
        }
    }

    /// Merge from maybe by cloning.
    fn clone_from(&mut self, maybe: &Option<T>)
    where
        T: Clone,
    {
        if let Some(v) = maybe {
            *self = v.clone();
        }
    }
}

// this would be more useful if try blocks exposed their "other" type so we could call into on e
#[easy_ext::ext(ResultExt)]
pub impl<T, E> Result<T, E> {
    /// cast result types.
    ///
    /// # Note
    /// Difficult to used with ?, more useful in return statements.
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
    // debated between prefix and prefix_err, chose the first because anyhow just calls it context
    /// Convert Err(e) to the string '{prefix}: {e}'
    fn prefix(self, prefix: impl Display) -> Result<T, StringError>
    where
        E: std::fmt::Display,
    {
        match self {
            Ok(val) => Ok(val),
            Err(e) => Err(format!("{prefix}: {e}").into()),
        }
    }

    fn context(self, prefix: impl Display) -> anyhow::Result<T>
    where
        E: std::fmt::Display,
    {
        match self {
            Ok(val) => Ok(val),
            Err(e) => Err(anyhow::anyhow!("{prefix}: {e}")),
        }
    }

    // logging

    /// Log the error.
    /// See also: [`ResultExt::prefix`].
    fn elog(self) -> Result<T, E>
    where
        E: Display,
    {
        self.map_err(|e| {
            log::error!("{e}");
            e
        })
    }

    /// [`elog`], then consume the error.
    /// See also: [`crate::bog::BogOkExt`].
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

    fn wlog(self) -> Result<T, E>
    where
        E: Display,
    {
        self.map_err(|e| {
            log::warn!("{e}");
            e
        })
    }

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

    fn elog<E: Display>(self, err: E) -> Result<T, E> {
        match self {
            Some(v) => Ok(v),
            None => {
                log::error!("{err}");
                Err(err)
            }
        }
    }

    fn context<E: Display>(self, err: E) -> anyhow::Result<T> {
        match self {
            Some(v) => Ok(v),
            None => Err(anyhow::anyhow!("{err}")),
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
    fn then_modify<T>(&self, base: T, modification: impl FnOnce(T) -> T) -> T {
        if *self { modification(base) } else { base }
    }

    #[inline]
    fn neg(&self) -> Self {
        !*self
    }

    #[inline]
    fn change(&mut self, new: Self) -> bool {
        let ret = *self != new;
        *self = new;
        ret
    }

    #[inline]
    fn or_exit(&self) {
        if !self {
            std::process::exit(1)
        }
    }
}

#[easy_ext::ext(TransformExt)]
pub impl<T> T {
    fn transform<Q>(self, transform: impl FnOnce(Self) -> Q) -> Q
    where
        T: Sized,
    {
        transform(self)
    }
}
