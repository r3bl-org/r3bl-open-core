/*
 *   Copyright (c) 2025 R3BL LLC
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

use crate::InlineString;

/// This macro is used to format an option. If the option is [Some], it will return the
/// value. It is meant for use with [`std::fmt::Formatter::debug_struct`].
///
/// When using this, make sure to import [`FormatOptionMsg`] as well, like this:
///
/// ```
/// use r3bl_tui::{fmt_option, FormatOptionMsg};
///
/// struct FooStruct {
///    pub insertion_pos_for_next_box: Option<r3bl_tui::Pos>,
/// }
///
/// impl std::fmt::Debug for FooStruct {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         f.debug_struct("FlexBox")
///             .field(
///                 "insertion_pos_for_next_box",
///                 fmt_option!(&self.insertion_pos_for_next_box),
///              )
///             .finish()
///     }
/// }
#[macro_export]
macro_rules! fmt_option {
    ($opt:expr) => {
        match ($opt) {
            Some(v) => v,
            None => &$crate::FormatOptionMsg::None,
        }
    };
}

#[derive(Clone, Copy, Debug)]
pub enum FormatOptionMsg {
    None,
}

/// Marker trait to "remember" which types can be converted to plain text.
pub trait ConvertToPlainText {
    fn to_plain_text(&self) -> InlineString;
}

/// Marker trait to "remember" which types support pretty printing for debugging.
pub trait PrettyPrintDebug {
    fn pretty_print_debug(&self) -> InlineString;
}
