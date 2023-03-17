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

//! More info:
//! - <https://stackoverflow.com/questions/43292357/how-can-one-detect-the-os-type-using-rust>
//! - <https://docs.rs/termcolor/1.2.0/src/termcolor/lib.rs.html#206-219>
//! - <https://github.com/microsoft/terminal/issues/11057>
//! - <https://github.com/termstandard/colors>
//! - <https://unix.stackexchange.com/a/67540/302646>

use std::env::{self, consts};

use crate::color_support_global_static::{clear_color_support_override,
                                         get_color_support_override,
                                         set_color_support_override};

#[derive(Debug, Copy, Clone)]
pub enum ColorSupport {
    Grayscale,
    /// ANSI 256 colors: <https://www.ditig.com/256-colors-cheat-sheet>
    Ansi256,
    Truecolor,
}

impl ColorSupport {
    pub fn set_color_support_override(color_support: ColorSupport) {
        set_color_support_override(color_support);
    }

    pub fn clear_color_support_override() { clear_color_support_override(); }

    /// This function is used to determine the color support of the current terminal. Some
    /// heuristics are used to determine what the highest color support can be. Once determined this
    /// value is cached in a global static variable. If you want to override this value please use
    /// [set_color_support_override] and [clear_color_support_override].
    pub fn detect() -> ColorSupport {
        // Override is set.
        if let Some(color_support) = get_color_support_override() {
            return color_support;
        }

        // Override is not set.
        match consts::OS {
            "macos" => {
                // Check for the TERM environment variable.
                if let Some(term_value) = env::var_os("TERM") {
                    return match term_value.to_str() {
                        Some("iterm") => ColorSupport::Truecolor,
                        Some("xterm-truecolor") => ColorSupport::Truecolor,
                        Some("tmux-truecolor") => ColorSupport::Truecolor,
                        _ => ColorSupport::Ansi256,
                    };
                }

                // Default to ANSI256.
                ColorSupport::Ansi256
            }
            "linux" => {
                // Check for the COLORTERM environment variable.
                if let Some(colorterm_value) = env::var_os("COLORTERM") {
                    return match colorterm_value.to_str() {
                        Some("truecolor") => ColorSupport::Truecolor,
                        Some("24bit") => ColorSupport::Truecolor,
                        _ => ColorSupport::Ansi256,
                    };
                }

                // Check for the TERM environment variable.
                if let Some(term_value) = env::var_os("TERM") {
                    return match term_value.to_str() {
                        Some("linux-truecolor") => ColorSupport::Truecolor,
                        Some("xterm-truecolor") => ColorSupport::Truecolor,
                        Some("tmux-truecolor") => ColorSupport::Truecolor,
                        Some("iterm") => ColorSupport::Truecolor,
                        _ => ColorSupport::Ansi256,
                    };
                }

                // Default to ANSI256.
                ColorSupport::Ansi256
            }
            "windows" => ColorSupport::Truecolor,
            _ => ColorSupport::Grayscale,
        }
    }
}

#[test]
fn test_color_support() {
    let color_support = ColorSupport::detect();
    println!("Color support: {:?}", color_support);
}

#[test]
fn test_override() {
    use r3bl_rs_utils_core::*;

    // Set the override.
    set_color_support_override(ColorSupport::Ansi256);

    // Test the override.
    let color_support = ColorSupport::detect();
    println!("Color support: {:?}", color_support);

    assert_eq2!(matches!(color_support, ColorSupport::Ansi256), true);
}
