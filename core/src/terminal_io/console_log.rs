/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use std::fmt::Debug;

use crate::{InlineString, fg_rgb_color, inline_string, tui_color};

/// Marker trait to "remember" which types can be printed to the console w/ color. Any
/// type that implements `Debug` can be printed to the console using this trait.
pub trait ConsoleLogInColor {
    fn console_log_fg(&self);
    fn prepare_console_log_fg_output(&self) -> InlineString;
    fn console_log_bg(&self);
    fn prepare_console_log_bg_output(&self) -> InlineString;
}

fn prepare_console_log_fg_output(this: &str) -> InlineString {
    let msg = if this.is_empty() {
        "← empty →"
    } else {
        &inline_string!("{this}")
    };
    let msg_fmt = fg_rgb_color(tui_color!(lizard_green), msg);
    inline_string!("{}", msg_fmt)
}

fn console_log_fg(this: &str) {
    println!("\n{}", prepare_console_log_fg_output(this));
}

fn prepare_console_log_bg(this: &str) -> InlineString {
    let msg = if this.is_empty() {
        "← empty →"
    } else {
        &inline_string!("{this}")
    };
    let msg_fmt =
        fg_rgb_color(tui_color!(cyan), msg).bg_rgb_color(tui_color!(slate_grey));
    inline_string!("{}", msg_fmt)
}

fn console_log_bg(this: &str) {
    println!("\n{}", prepare_console_log_bg(this));
}

impl<T: Debug> ConsoleLogInColor for T {
    fn console_log_fg(&self) {
        let it = self.prepare_console_log_fg_output();
        console_log_fg(it.as_str());
    }

    fn prepare_console_log_fg_output(&self) -> InlineString {
        prepare_console_log_fg_output(&inline_string!("{self:?}"))
    }

    fn console_log_bg(&self) {
        let it = self.prepare_console_log_bg_output();
        console_log_bg(it.as_str());
    }

    fn prepare_console_log_bg_output(&self) -> InlineString {
        prepare_console_log_bg(&inline_string!("{self:?}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_console_log_in_color() {
        let it = "Hello, World!";
        it.console_log_fg();
        it.console_log_bg();

        let str = "Hello, World!";
        str.console_log_fg();
        str.console_log_bg();
    }

    #[test]
    fn test_prepare_console_log_fg_output() {
        let it = "Hello, World!";
        let output = prepare_console_log_fg_output(it);
        // Assert that output contains "Hello, World!" and some ANSI escape sequences.
        assert!(output.contains("Hello, World!"));
        assert!(output.contains("\u{1b}"));
    }

    #[test]
    fn test_prepare_console_log_bg_output() {
        let it = "Hello, World!";
        let output = prepare_console_log_bg(it);
        // Assert that output contains "Hello, World!" and some ANSI escape sequences.
        assert!(output.contains("Hello, World!"));
        assert!(output.contains("\u{1b}"));
    }
}
