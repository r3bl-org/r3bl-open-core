use crossterm::style::Stylize;

use crate::UnicodeString;

type US = UnicodeString;

/// Marker trait to "remember" which types can be printed to the console w/ color.
pub trait ConsoleLogInColor {
    fn console_log_fg(&self);
    fn console_log_bg(&self);
}

fn console_log_fg(this: &str) {
    if this.is_empty() {
        println!("\n{}", "← empty →".yellow());
    } else {
        println!("\n{}", this.yellow());
    }
}

fn console_log_bg(this: &str) {
    if this.is_empty() {
        println!("\n{}", "← empty →".red().on_white());
    } else {
        println!("\n{}", this.red().on_white());
    }
}

impl<T: PrettyPrintDebug> ConsoleLogInColor for T {
    fn console_log_fg(&self) { console_log_fg(&self.pretty_print_debug()); }

    fn console_log_bg(&self) { console_log_bg(&self.pretty_print_debug()); }
}

impl ConsoleLogInColor for &str {
    fn console_log_fg(&self) { console_log_fg(self); }

    fn console_log_bg(&self) { console_log_bg(self); }
}

impl ConsoleLogInColor for String {
    fn console_log_fg(&self) { console_log_fg(self); }

    fn console_log_bg(&self) { console_log_bg(self); }
}

/// Marker trait to "remember" which types support pretty printing for debugging.
pub trait PrettyPrintDebug {
    fn pretty_print_debug(&self) -> String;
}

/// Marker trait to "remember" which types can be converted to plain text.
pub trait ConvertToPlainText {
    fn to_plain_text_us(&self) -> US;
}
