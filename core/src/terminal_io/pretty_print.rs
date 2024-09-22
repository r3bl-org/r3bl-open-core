/*
 *   Copyright (c) 2024 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

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
