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

/// This macro is used to format an option. If the option is [Some], it will return the
/// value. It is meant for use with [std::fmt::Formatter::debug_struct].
///
/// When using this, make sure to import [FormatMsg] as well, like this:
/// ```rust
/// use r3bl_tui::{format_option, FormatMsg};
/// ```
#[macro_export]
macro_rules! format_option {
    ($opt:expr) => {
        match ($opt) {
            Some(v) => v,
            None => &$crate::FormatMsg::None,
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub enum FormatMsg {
    None,
}
