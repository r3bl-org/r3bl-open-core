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

use crossterm::style::{Attribute, SetAttribute};

#[macro_export]
macro_rules! apply_style {
    ($fg: expr => fg) => {
        SetForegroundColor(::r3bl_core::convert_from_tui_color_to_crossterm_color($fg))
    };
    ($bg: expr => bg) => {
        SetBackgroundColor(::r3bl_core::convert_from_tui_color_to_crossterm_color($bg))
    };
    ($style: expr => bold) => {
        $crate::set_attribute($style.bold, Attribute::Bold, Attribute::NoBold)
    };
    ($style: expr => italic) => {
        $crate::set_attribute($style.italic, Attribute::Italic, Attribute::NoItalic)
    };
    ($style: expr => dim) => {
        $crate::set_attribute($style.dim, Attribute::Dim, Attribute::NormalIntensity)
    };
    ($style: expr => underline) => {
        $crate::set_attribute(
            $style.underline,
            Attribute::Underlined,
            Attribute::NoUnderline,
        )
    };
    ($style: expr => reverse) => {
        $crate::set_attribute($style.reverse, Attribute::Reverse, Attribute::NoReverse)
    };
    ($style: expr => hidden) => {
        $crate::set_attribute($style.hidden, Attribute::Hidden, Attribute::NoHidden)
    };
    ($style: expr => strikethrough) => {
        $crate::set_attribute(
            $style.strikethrough,
            Attribute::CrossedOut,
            Attribute::NotCrossedOut,
        )
    };
}

pub fn set_attribute(
    enable: bool,
    enable_attribute: Attribute,
    disable_attribute: Attribute,
) -> SetAttribute {
    match enable {
        true => SetAttribute(enable_attribute),
        false => SetAttribute(disable_attribute),
    }
}
