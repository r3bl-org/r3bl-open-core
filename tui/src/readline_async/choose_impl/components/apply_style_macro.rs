/*
 *   Copyright (c) 2023-2025 R3BL LLC
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

#[macro_export]
macro_rules! apply_style {
    ($fg: expr => fg) => {
        SetForegroundColor(::r3bl_core::convert_from_tui_color_to_crossterm_color($fg))
    };
    ($bg: expr => bg) => {
        SetBackgroundColor(::r3bl_core::convert_from_tui_color_to_crossterm_color($bg))
    };
    ($style: expr => bold) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.bold.is_some() {
            true => SetAttribute(Attribute::Bold),
            false => SetAttribute(Attribute::NoBold),
        }
    }};
    ($style: expr => italic) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.italic.is_some() {
            true => SetAttribute(Attribute::Italic),
            false => SetAttribute(Attribute::NoItalic),
        }
    }};
    ($style: expr => dim) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.dim.is_some() {
            true => SetAttribute(Attribute::Dim),
            false => SetAttribute(Attribute::NormalIntensity),
        }
    }};
    ($style: expr => underline) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.underline.is_some() {
            true => SetAttribute(Attribute::Underlined),
            false => SetAttribute(Attribute::NoUnderline),
        }
    }};
    ($style: expr => reverse) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.reverse.is_some() {
            true => SetAttribute(Attribute::Reverse),
            false => SetAttribute(Attribute::NoReverse),
        }
    }};
    ($style: expr => hidden) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.hidden.is_some() {
            true => SetAttribute(Attribute::Hidden),
            false => SetAttribute(Attribute::NoHidden),
        }
    }};
    ($style: expr => strikethrough) => {{
        use crossterm::style::{Attribute, SetAttribute};
        match $style.strikethrough.is_some() {
            true => SetAttribute(Attribute::CrossedOut),
            false => SetAttribute(Attribute::NotCrossedOut),
        }
    }};
}
