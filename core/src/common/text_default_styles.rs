/*
 *   Copyright (c) 2024-2025 R3BL LLC
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

use r3bl_ansi_color::{AnsiStyledText, Color, Style};

pub fn style_primary(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Foreground(Color::Rgb(50, 200, 50))],
    }
}

pub fn style_prompt(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Foreground(Color::Rgb(100, 100, 200))],
    }
}

pub fn style_error(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Foreground(Color::Rgb(200, 0, 50))],
    }
}

pub fn style_underline(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Underline],
    }
}

pub fn style_dim(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Dim],
    }
}

pub fn style_dim_underline(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Dim, Style::Underline],
    }
}
