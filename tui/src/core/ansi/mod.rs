// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module allows you to generate formatted ANSI 256 (8-bit) and truecolor (24-bit)
//! color output to stdout. On macOS, the default Terminal.app does not support truecolor,
//! so ANSI 256 colors are used instead.
//!
//! This crate performs its own detection of terminal color capability heuristically. And
//! does not use other crates to perform this function.
//!
//! Here's a screenshot of running the `main` example on various operating systems:
//!
//! | ![Linux screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/refs/heads/main/docs/image/screenshot_linux.png) |
//! |:--:|
//! | *Running on Linux Tilix* |
//!
//! | ![Windows screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/refs/heads/main/docs/image/screenshot_windows.png) |
//! |:--:|
//! | *Running on Windows Terminal* |
//!
//! | ![macOS screenshot Terminal app](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/refs/heads/main/docs/image/screenshot_macos_terminal_app.png) |
//! |:--:|
//! | *Running on macOS terminal app (note ANSI 256 runtime detection)* |
//!
//! | ![macOS screenshot iTerm app](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/refs/heads/main/docs/image/screenshot_macos_iterm_app.png) |
//! |:--:|
//! | *Running on macOS iTerm app (note Truecolor runtime detection)* |
//!
//! # How to use it
//!
//! The main struct that we have to consider is `AnsiStyledText`. It has two fields:
//!
//! - `text` - the text to print.
//! - `style` - a list of styles to apply to the text.
//!
//! Here's an example.
//!
//! ```
//! # use r3bl_tui::{
//! #     fg_red, size, fg_color, tui_color, new_style, ast,
//! #     RgbValue, ASTStyle, AnsiStyledText,
//! # };
//!
//! // Use ast() to create a styled text.
//! let styled_text = ast("Hello", new_style!(bold));
//! println!("{styled_text}");
//! styled_text.println();
//! ```
//!
//! For more examples, please read the documentation for [`AnsiStyledText`]. Please don't
//! create this struct directly, use [`crate::ast()`], [`crate::ast_line!`],
//! [`crate::ast_lines`!] or the constructor functions like [`fg_red()`], [`fg_green()`],
//! [`fg_blue()`], etc.
//!
//! # References
//!
//! - [ANSI Escape Codes](https://notes.burke.libbey.me/ansi-escape-codes/)
//! - [ASCII Table](https://www.asciitable.com/)
//! - [Xterm 256color Chart](https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg)
//! - [256 Colors Cheat Sheet](https://www.ditig.com/256-colors-cheat-sheet)
//! - [List of ANSI Color Escape Sequences](https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences)
//! - [Color Metric](https://www.compuphase.com/cmetric.htm)

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

// Attach.
pub mod ansi_escape_codes;
pub mod ansi_styled_text;
pub mod color;
pub mod detect_color_support;
pub mod terminal_output;

pub use ansi_escape_codes::*;
pub use ansi_styled_text::*;
pub use color::*;
pub use detect_color_support::*;
pub use terminal_output::*;
