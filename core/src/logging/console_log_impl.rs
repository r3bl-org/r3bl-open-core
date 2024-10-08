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

//! ANSI colorized text <https://github.com/ogham/rust-ansi-term> helper methods. This is
//! used by the [`crate::console_log!`] macro (in `decl_macro.rs`).

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
