// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use core::fmt::Debug;
use std::ops::Deref;

use crate::TinyInlineString;

/// Contains the visual style attributes that can be applied to text.
/// This struct is shared between `TuiStyle` (which adds id, computed, padding, lolcat)
/// and `AnsiToBufferProcessor` (which uses these for ANSI sequence processing).
#[derive(Copy, Clone, PartialEq, Eq, Hash, Default, Debug)]
pub struct TuiStyleAttribs {
    // XMARK: Use of newtype pattern `Option<T>` instead of `bool`.
    pub bold: Option<tui_style_attrib::Bold>,
    pub italic: Option<tui_style_attrib::Italic>,
    pub dim: Option<tui_style_attrib::Dim>,
    pub underline: Option<tui_style_attrib::Underline>,
    pub blink: Option<tui_style_attrib::Blink>,
    pub reverse: Option<tui_style_attrib::Reverse>,
    pub hidden: Option<tui_style_attrib::Hidden>,
    pub strikethrough: Option<tui_style_attrib::Strikethrough>,
}

pub mod tui_style_attrib {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Id(pub u8);

    impl Id {
        #[must_use]
        pub fn eq(maybe_id: Option<Id>, other: u8) -> bool {
            match maybe_id {
                None => false,
                Some(id) => id.0 == other,
            }
        }

        #[must_use]
        pub fn fmt_id(maybe_id: Option<Id>) -> TinyInlineString {
            use std::fmt::Write;
            let mut acc = TinyInlineString::new();
            match maybe_id {
                None => {
                    // We don't care about the result of this operation.
                    write!(acc, "id: N/A").ok();
                }
                Some(id) => {
                    // We don't care about the result of this operation.
                    write!(acc, "id: {}", id.0).ok();
                }
            }
            acc
        }
    }

    pub fn id(arg_val: impl Into<u8>) -> Option<Id> { Some(Id(arg_val.into())) }

    impl From<u8> for Id {
        fn from(id: u8) -> Self { Id(id) }
    }

    impl Deref for Id {
        type Target = u8;
        fn deref(&self) -> &Self::Target { &self.0 }
    }

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Bold;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Italic;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Dim;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Underline;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Reverse;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Hidden;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Strikethrough;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Blink;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Computed;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
    pub struct Lolcat;
}
