/*
 *   Copyright (c) 2023 R3BL LLC
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

use crossterm::style::{Attribute, Color, SetAttribute};
use r3bl_ansi_color::{global_color_support, ColorSupport, TransformColor};

pub fn get_crossterm_color_based_on_terminal_capabilities(
    color: r3bl_ansi_color::Color,
) -> Color {
    let detect_color_support = global_color_support::detect();
    match detect_color_support {
        ColorSupport::Truecolor => {
            let rgb_color = color.as_rgb();
            Color::Rgb {
                r: rgb_color.red,
                g: rgb_color.green,
                b: rgb_color.blue,
            }
        }
        _ => Color::AnsiValue(color.as_ansi256().index),
    }
}

#[macro_export]
macro_rules! apply_style {
    ($style: expr => bg_color) => {
        SetBackgroundColor(get_crossterm_color_based_on_terminal_capabilities(
            $style.bg_color,
        ))
    };
    ($style: expr => fg_color) => {
        SetForegroundColor(get_crossterm_color_based_on_terminal_capabilities(
            $style.fg_color,
        ))
    };
    ($style: expr => bold) => {
        set_attribute($style.bold, Attribute::Bold, Attribute::NoBold)
    };
    ($style: expr => italic) => {
        set_attribute($style.italic, Attribute::Italic, Attribute::NoItalic)
    };
    ($style: expr => dim) => {
        set_attribute($style.dim, Attribute::Dim, Attribute::NormalIntensity)
    };
    ($style: expr => underline) => {
        set_attribute(
            $style.underline,
            Attribute::Underlined,
            Attribute::NoUnderline,
        )
    };
    ($style: expr => reverse) => {
        set_attribute($style.reverse, Attribute::Reverse, Attribute::NoReverse)
    };
    ($style: expr => hidden) => {
        set_attribute($style.hidden, Attribute::Hidden, Attribute::NoHidden)
    };
    ($style: expr => strikethrough) => {
        set_attribute(
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
