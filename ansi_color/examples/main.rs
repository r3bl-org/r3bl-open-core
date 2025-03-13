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

use r3bl_ansi_color::{ASTColor,
                      ASTStyle,
                      AnsiStyledText,
                      ColorSupport,
                      global_color_support};

fn main() {
    // Print a string w/ ANSI color codes.
    {
        AnsiStyledText {
            text:
                "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
            style: smallvec::smallvec![
                ASTStyle::Bold,
                ASTStyle::Italic,
                ASTStyle::Underline,
                ASTStyle::Foreground(ASTColor::Rgb(50, 50, 50)),
                ASTStyle::Background(ASTColor::Rgb(100, 200, 1)),
            ],
        }
        .println();

        AnsiStyledText {
            text: "Dim, Overline and Strikethrough line.",
            style: smallvec::smallvec![
                ASTStyle::Dim,
                ASTStyle::Strikethrough,
                ASTStyle::Overline,
                ASTStyle::Foreground(ASTColor::Rgb(200, 50, 50)),
                ASTStyle::Background(ASTColor::Rgb(200, 200, 1)),
            ],
        }
        .println();
    }

    // Set the color support override to ANSI 256 color mode.
    {
        global_color_support::set_override(ColorSupport::Ansi256);
        let msg: String = format!(
            "> Force ANSI 256 color mode ({:?})",
            global_color_support::detect()
        );
        print_text(&msg);
    }

    // Set the color support override to truecolor mode.
    {
        global_color_support::set_override(ColorSupport::Truecolor);
        let msg: String = format!(
            "> Force True color mode ({:?})",
            global_color_support::detect()
        );
        print_text(&msg);
    }

    // Set the color support override to grayscale mode.
    {
        global_color_support::set_override(ColorSupport::Grayscale);
        let msg: String = format!(
            "> Force Grayscale color mode ({:?})",
            global_color_support::detect()
        );
        print_text(&msg);
    }

    // Use runtime detection to determine the color support.
    {
        global_color_support::clear_override();
        let msg = format!(
            "> Runtime detection of color support ({:?})",
            global_color_support::detect()
        );
        print_text(&msg);
    }
}

fn print_text(msg: &str) {
    AnsiStyledText {
        text: msg,
        style: smallvec::smallvec![
            ASTStyle::Underline,
            ASTStyle::Foreground(ASTColor::Rgb(200, 200, 1)),
            ASTStyle::Background(ASTColor::Rgb(100, 60, 150)),
        ],
    }
    .println();

    let eg_1 = AnsiStyledText {
        text: "Hello",
        style: smallvec::smallvec![
            ASTStyle::Foreground(ASTColor::Rgb(100, 60, 150)),
            ASTStyle::Background(ASTColor::Rgb(100, 200, 50)),
        ],
    };
    println!("eg_1: {0}", eg_1);

    let eg_2 = AnsiStyledText {
        text: "World",
        style: smallvec::smallvec![
            ASTStyle::Foreground(ASTColor::Ansi256(150)),
            ASTStyle::Background(ASTColor::Rgb(50, 50, 100)),
        ],
    };
    println!("eg_2: {0}", eg_2);
}
