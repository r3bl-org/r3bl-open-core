// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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
