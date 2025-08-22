// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use core::fmt::Debug;

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
