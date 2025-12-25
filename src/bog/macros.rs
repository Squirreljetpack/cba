
// todo:
// - ; for delimiters instead of ,
// - set log,bog,levels through params

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
    ($expr:expr; $none_expr:expr) => {
        match $expr {
            Some(v) => v,
            None => {
                $none_expr
                ;
                return Default::default();
            }
        }
    };
}