// todo:
// - ; for delimiters instead of ,
// - set log,bog,levels through params

// purposefully limited more complicated, prefer let else
#[macro_export]
macro_rules! unwrap {
    ($expr:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return Default::default();
            }
        }
    };
    ($expr:expr; continue) => {
        match $expr {
            Some(v) => v,
            None => continue,
        }
    };

    ($expr:expr; |$i:ident| $body:expr) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                return (|$i| $body)(e);
            }
        }
    };

    ($expr:expr; |$i:ident| $body:expr; continue) => {
        match $expr {
            Ok(v) => v,
            Err(e) => {
                (|$i| $body)(e);
                continue;
            }
        }
    };

    ($expr:expr; $err:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return Err($err);
            }
        }
    };
}
