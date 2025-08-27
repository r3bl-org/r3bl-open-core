// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use core::fmt::Debug;
use std::ops::{Add, AddAssign};

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

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Bold;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Italic;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Dim;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Underline;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Reverse;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Hidden;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Strikethrough;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Blink;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Computed;

    #[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
    pub struct Lolcat;
}

pub fn tui_style_attribs(arg: impl Into<TuiStyleAttribs>) -> TuiStyleAttribs {
    arg.into()
}

impl Add for TuiStyleAttribs {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            bold: self.bold.or(rhs.bold),
            italic: self.italic.or(rhs.italic),
            dim: self.dim.or(rhs.dim),
            underline: self.underline.or(rhs.underline),
            blink: self.blink.or(rhs.blink),
            reverse: self.reverse.or(rhs.reverse),
            hidden: self.hidden.or(rhs.hidden),
            strikethrough: self.strikethrough.or(rhs.strikethrough),
        }
    }
}

macro_rules! impl_from_and_add_for_attrib {
    ($type:ty, $field:ident) => {
        // From<$type> for TuiStyleAttribs
        impl From<$type> for TuiStyleAttribs {
            fn from(val: $type) -> Self {
                TuiStyleAttribs {
                    $field: Some(val),
                    ..Default::default()
                }
            }
        }

        // TuiStyleAttribs + $type
        impl Add<$type> for TuiStyleAttribs {
            type Output = TuiStyleAttribs;
            fn add(mut self, rhs: $type) -> Self::Output {
                self.$field = Some(rhs);
                self
            }
        }

        // $type + TuiStyleAttribs
        impl Add<TuiStyleAttribs> for $type {
            type Output = TuiStyleAttribs;
            fn add(self, mut rhs: TuiStyleAttribs) -> Self::Output {
                rhs.$field = Some(self);
                rhs
            }
        }
    };
}

#[allow(unused_macro_rules)]
macro_rules! define_attrib_operations {
    // Done
    () => {};

    // Just one element left
    (($type:ty, $field:ident)) => {
        impl_from_and_add_for_attrib!($type, $field);
    };

    // Multiple elements
    (($type:ty, $field:ident), $(($rest_type:ty, $rest_field:ident)),+) => {
        impl_from_and_add_for_attrib!($type, $field);

        // $type + $other_type
        $(
            impl Add<$rest_type> for $type {
                type Output = TuiStyleAttribs;
                fn add(self, rhs: $rest_type) -> Self::Output {
                    TuiStyleAttribs::from(self) + TuiStyleAttribs::from(rhs)
                }
            }

            impl Add<$type> for $rest_type {
                type Output = TuiStyleAttribs;
                fn add(self, rhs: $type) -> Self::Output {
                    TuiStyleAttribs::from(self) + TuiStyleAttribs::from(rhs)
                }
            }
        )*

        // Recurse.
        define_attrib_operations!($(($rest_type, $rest_field)),+);
    };
}

define_attrib_operations!(
    (tui_style_attrib::Bold, bold),
    (tui_style_attrib::Italic, italic),
    (tui_style_attrib::Dim, dim),
    (tui_style_attrib::Underline, underline),
    (tui_style_attrib::Blink, blink),
    (tui_style_attrib::Reverse, reverse),
    (tui_style_attrib::Hidden, hidden),
    (tui_style_attrib::Strikethrough, strikethrough)
);

macro_rules! impl_add_assign_for_attrib {
    ($type:ty, $field:ident) => {
        impl AddAssign<$type> for TuiStyleAttribs {
            fn add_assign(&mut self, rhs: $type) { self.$field = Some(rhs); }
        }
    };
}

impl_add_assign_for_attrib!(tui_style_attrib::Bold, bold);
impl_add_assign_for_attrib!(tui_style_attrib::Italic, italic);
impl_add_assign_for_attrib!(tui_style_attrib::Dim, dim);
impl_add_assign_for_attrib!(tui_style_attrib::Underline, underline);
impl_add_assign_for_attrib!(tui_style_attrib::Blink, blink);
impl_add_assign_for_attrib!(tui_style_attrib::Reverse, reverse);
impl_add_assign_for_attrib!(tui_style_attrib::Hidden, hidden);
impl_add_assign_for_attrib!(tui_style_attrib::Strikethrough, strikethrough);
