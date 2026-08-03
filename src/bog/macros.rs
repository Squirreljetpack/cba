pub use crate::{_ibog, _wbog, cbog, dbog, ebog, ibog, mbog, nbog, wbog};

#[macro_export]
macro_rules! ibog {
    // With tag expressions
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::INFO,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    // Without tag
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::INFO,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! dbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::DEBUG,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::DEBUG,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! ebog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::ERROR,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::ERROR,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! wbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::WARN,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::WARN,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! nbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::NOTE,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::NOTE,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! mbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::EMPTY,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::EMPTY,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! cbog {
    ($discriminant:literal ; $($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($discriminant:literal ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::CUSTOM($discriminant),
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! _wbog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_WRN,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_WRN,
            "",
            &format!($($arg),*),
        );
    }};
}

#[macro_export]
macro_rules! _ibog {
    ($($harg:expr),* ; $($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_NFO,
            &format!($($harg),*),
            &format!($($arg),*),
        );
    }};
    ($($arg:expr),*) => {{
        $crate::BOGGER::bog(
            $crate::bog::BogLevel::_NFO,
            "",
            &format!($($arg),*),
        );
    }};
}
