// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[macro_export]
macro_rules! choose_apply_style {
    ($fg: expr => fg) => {
        SetForegroundColor($fg.into())
    };
    ($bg: expr => bg) => {
        SetBackgroundColor($bg.into())
    };
    ($style: expr => bold) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.bold.is_some() {
            true => SetAttribute(Attribute::Bold),
            false => SetAttribute(Attribute::NoBold),
        }
    }};
    ($style: expr => italic) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.italic.is_some() {
            true => SetAttribute(Attribute::Italic),
            false => SetAttribute(Attribute::NoItalic),
        }
    }};
    ($style: expr => dim) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.dim.is_some() {
            true => SetAttribute(Attribute::Dim),
            false => SetAttribute(Attribute::NormalIntensity),
        }
    }};
    ($style: expr => underline) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.underline.is_some() {
            true => SetAttribute(Attribute::Underlined),
            false => SetAttribute(Attribute::NoUnderline),
        }
    }};
    ($style: expr => reverse) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.reverse.is_some() {
            true => SetAttribute(Attribute::Reverse),
            false => SetAttribute(Attribute::NoReverse),
        }
    }};
    ($style: expr => hidden) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.hidden.is_some() {
            true => SetAttribute(Attribute::Hidden),
            false => SetAttribute(Attribute::NoHidden),
        }
    }};
    ($style: expr => strikethrough) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.attribs.strikethrough.is_some() {
            true => SetAttribute(Attribute::CrossedOut),
            false => SetAttribute(Attribute::NotCrossedOut),
        }
    }};
}
