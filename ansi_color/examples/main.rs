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

use r3bl_ansi_color::{global_color_support, AnsiStyledText, Color, ColorSupport, Style};

fn main() {
    // Print a string w/ ANSI color codes.
    {
        AnsiStyledText {
            text:
                "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
            style: &[
                Style::Bold,
                Style::Italic,
                Style::Underline,
                Style::Foreground(Color::Rgb(50, 50, 50)),
                Style::Background(Color::Rgb(100, 200, 1)),
            ],
        }
        .println();

        AnsiStyledText {
            text: "Dim, Overline and Strikethrough line.",
            style: &[
                Style::Dim,
                Style::Strikethrough,
                Style::Overline,
                Style::Foreground(Color::Rgb(200, 50, 50)),
                Style::Background(Color::Rgb(200, 200, 1)),
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
        style: &[
            Style::Underline,
            Style::Foreground(Color::Rgb(200, 200, 1)),
            Style::Background(Color::Rgb(100, 60, 150)),
        ],
    }
    .println();

    let eg_1 = AnsiStyledText {
        text: "Hello",
        style: &[
            Style::Foreground(Color::Rgb(100, 60, 150)),
            Style::Background(Color::Rgb(100, 200, 50)),
        ],
    };
    println!("eg_1: {0}", eg_1);

    let eg_2 = AnsiStyledText {
        text: "World",
        style: &[
            Style::Foreground(Color::Ansi256(150)),
            Style::Background(Color::Rgb(50, 50, 100)),
        ],
    };
    println!("eg_2: {0}", eg_2);
}
