//! Display colored log-style messages for CLI tools
//! Performance is no concern (hence `bog`), only convenience and style.

mod fmt;
mod global;
mod macros;

pub use fmt::*;
pub use global::*;
pub use macros::*;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn show_fg_bogger() {
        init_bogger(true, false);
        // DEBUG messages
        dbog!("DEBUG message: {}", 3.14159);
        dbog!("val"; "DEBUG values: x={}, y={}", 10, 20);

        // INFO messages
        ibog!("INFO message: {}", 42);
        ibog!("Created Directory"; "~/archr/Desktop");

        // WARN messages
        wbog!("WARN message: {}", "disk almost full");
        wbog!("NoSpace"; "WARN message: {} attempts left", 3);

        // ERROR messages
        ebog!("ERROR message: {}", "file not found");
        ebog!("404"; "Not found");

        // NOTE messages
        nbog!("justification");
        nbog!("NOTE"; "ancillary");
        mbog!("justification");
        mbog!("FULL TAG"; "ancillary");

        // CUSTOM / NOTE-like messages using cbog
        cbog!("NOTE"; "Custom note message: {}", "all good");
        cbog!("NOTE"; ""; "Custom note with tag: {}", 123);
        cbog!("CUSTOM"; "Custom discriminant"; "Message with both tag and content");
    }

    #[test]
    fn show_bg_bogger() {
        init_bogger(false, true);
        // DEBUG messages
        dbog!("DEBUG message: {}", 3.14159);
        dbog!("val"; "DEBUG values: x={}, y={}", 10, 20);

        // INFO messages
        ibog!("INFO message: {}", 42);
        ibog!("Urgent"; "INFO message number {}", 7);

        // WARN messages
        wbog!("WARN message: {}", "disk almost full");
        wbog!("NoSpace"; "WARN message: {} attempts left", 3);

        // ERROR messages
        ebog!("ERROR message: {}", "file not found");
        ebog!("404"; "Not found");

        // NOTE messages
        nbog!("justification");
        nbog!("NOTE"; "ancillary");
        mbog!("justification");
        mbog!("FULL"; "ancillary");

        // CUSTOM
        cbog!("NOTE"; "Custom note message: {}", "all good");
        cbog!("NOTE"; ""; "Custom note with tag: {}", 123);
        cbog!("CUSTOM"; "Custom discriminant"; "Message with both tag and content");
    }

    #[test]
    fn min_level_and_downcast_combined() {
        init_bogger(true, false);

        // drop DEBUG/INFO entirely
        BOGGER::filter_below(/* WARN priority */ BogLevel::INFO);
        // downcast ERROR to WARN
        BOGGER::downcast_above(BogLevel::WARN);

        dbog!("debug filtered");
        ibog!("info normal");
        ebog!("error shown as warn");
    }
}
