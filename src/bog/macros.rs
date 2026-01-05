// todo:
// - ; for delimiters instead of ,
// - set log,bog,levels through params

// purposefully limited more complicated, prefer let else
#[macro_export]
macro_rules! else_default {
    ($expr:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                return Default::default();
            }
        }
    };
    ($expr:expr; !) => {
        match $expr {
            Some(v) => v,
            None => continue,
        }
    };
    ($expr:expr; ?) => {
        match $expr {
            Some(v) => v,
            None => {
                return Err(Default::default());
            }
        }
    };
}
