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

use crossterm::style::Stylize;

use crate::string_storage;

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

impl<T: Debug> ConsoleLogInColor for T {
    fn console_log_fg(&self) {
        let it = string_storage!("{self:?}");
        console_log_fg(it.as_str());
    }

    fn console_log_bg(&self) {
        let it = string_storage!("{self:?}");
        console_log_bg(it.as_str());
    }
}
