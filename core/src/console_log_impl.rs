/*
 * Copyright (c) 2022 R3BL LLC. All rights reserved.
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! ANSI colorized text <https://github.com/ogham/rust-ansi-term> helper methods.

use r3bl_ansi_color::{AnsiStyledText, Color, Style};

fn purple(text: &str) -> AnsiStyledText {
    AnsiStyledText {
        text,
        style: &[Style::Foreground(Color::Rgb(147, 112, 219))],
    }
}

#[macro_export]
macro_rules! print_header {
    (error $msg: expr) => {
        let hamburger = "☰";
        let msg = format!("{0} {1} {0}", hamburger, $msg);
        eprintln!("{}", purple(&msg));
    };
    (normal $msg: expr) => {
        let hamburger = "☰";
        let msg = format!("{0} {1} {0}", hamburger, $msg);
        println!("{}", purple(&msg));
    };
}

///
/// Equivalent for template string literal. One way to do this using `format!`
/// 1. <https://doc.rust-lang.org/std/fmt/>
/// 2. <https://internals.rust-lang.org/t/string-interpolation-template-literals-like-js/9082/3>
pub fn print_header(msg: &str) {
    print_header!(normal msg);
}

pub fn eprint_header(msg: &str) {
    print_header!(error msg);
}

pub mod color_text_default_styles {
    use super::*;

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
}
