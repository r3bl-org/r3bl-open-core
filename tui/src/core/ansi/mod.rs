// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Why R3BL?
//!
//! <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">
//!
//! <!-- R3BL TUI library & suite of apps focused on developer productivity -->
//!
//! <span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span
//! style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span
//! style="color:#F92363"> </span><span style="color:#F82067">T</span><span
//! style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span
//! style="color:#F31874"> </span><span style="color:#F11678">l</span><span
//! style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span
//! style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span
//! style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span
//! style="color:#E10895"> </span><span style="color:#DE0799">&amp;</span><span
//! style="color:#DB069E"> </span><span style="color:#D804A2">s</span><span
//! style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span
//! style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span
//! style="color:#C801B6"> </span><span style="color:#C501B9">o</span><span
//! style="color:#C101BD">f</span><span style="color:#BD01C1"> </span><span
//! style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span
//! style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span
//! style="color:#AA03D2"> </span><span style="color:#A603D5">f</span><span
//! style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span
//! style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span
//! style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span
//! style="color:#890DE8"> </span><span style="color:#850FEB">o</span><span
//! style="color:#8111ED">n</span><span style="color:#7C13EF"> </span><span
//! style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span
//! style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span
//! style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span
//! style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span
//! style="color:#572CFC">r</span><span style="color:#532FFD"> </span><span
//! style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span
//! style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span
//! style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span
//! style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span
//! style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span
//! style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>
//!
//! We are working on building command line apps in Rust which have rich text user
//! interfaces (TUI). We want to lean into the terminal as a place of productivity, and
//! build all kinds of awesome apps for it.
//!
//! 1. 🔮 Instead of just building one app, we are building a library to enable any kind
//!    of rich TUI development w/ a twist: taking concepts that work really well for the
//!    frontend mobile and web development world and re-imagining them for TUI & Rust.
//!
//!    - Taking inspiration from things like [React](https://react.dev/), [SolidJS](https://www.solidjs.com/),
//!      [Elm](https://guide.elm-lang.org/architecture/), [iced-rs](https://docs.rs/iced/latest/iced/),
//!      [Jetpack Compose](https://developer.android.com/compose), [JSX](https://ui.dev/imperative-vs-declarative-programming),
//!      [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can be
//!      run in parallel & concurrent via [Tokio](https://crates.io/crates/tokio)).
//!    - Even the thread running the main event loop doesn't block since it is async.
//!    - Using proc macros to create DSLs to implement something inspired by [CSS](https://www.w3.org/TR/CSS/#css)
//!      & [JSX](https://ui.dev/imperative-vs-declarative-programming).
//!
//! 2. 🌎 We are building apps to enhance developer productivity & workflows.
//!
//!    - The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a
//!      single terminal window). Rather it is to build a set of integrated "apps" (or
//!      "tasks") that run in the same process that renders to one terminal window.
//!    - Inside of this terminal window, we can implement things like "app" switching,
//!      routing, tiling layout, stacking layout, etc. so that we can manage a lot of TUI
//!      apps (which are tightly integrated) that are running in the same process, in the
//!      same window. So you can imagine that all these "app"s have shared application
//!      state. Each "app" may also have its own local application state.
//!    - Here are some examples of the types of "app"s we plan to build (for which this
//!      infrastructure acts as the open source engine):
//!      1. Multi user text editors w/ syntax highlighting.
//!      2. Integrations w/ github issues.
//!      3. Integrations w/ calendar, email, contacts APIs.
//!
//! All the crates in the `r3bl-open-core`
//! [repo](https://github.com/r3bl-org/r3bl-open-core/) provide lots of useful
//! functionality to help you build TUI (text user interface) apps, along w/ general
//! niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉.
//!
//! # Table of contents
//!
//! <!-- TOC -->
//!
//! - [Introduction](#introduction)
//! - [Changelog](#changelog)
//! - [Learn how these crates are built, provide
//!   feedback](#learn-how-these-crates-are-built-provide-feedback)
//! - [How to use it](#how-to-use-it)
//! - [Build, run, test tasks](#build-run-test-tasks)
//!   - [Prerequisites](#prerequisites)
//!   - [Nushell commands](#nushell-commands)
//! - [References](#references)
//! - [Why make a new crate for this?](#why-make-a-new-crate-for-this)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! Rust crate to generate formatted ANSI 256 (8-bit) and truecolor (24-bit) color output
//! to stdout. On macOS, the default Terminal.app does not support truecolor, so ANSI 256
//! colors are used instead.
//!
//! This crate performs its own detection of terminal color capability heuristically. And
//! does not use other crates to perform this function.
//!
//! Here's a screenshot of running the `main` example on various operating systems:
//!
//! | ![Linux screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/ansi_color/docs/screenshot_linux.png?raw=true) |
//! |:--:|
//! | *Running on Linux Tilix* |
//!
//! | ![Windows screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/ansi_color/docs/screenshot_windows.png?raw=true) |
//! |:--:|
//! | *Running on Windows Terminal* |
//!
//! | ![macOS screenshot Terminal app](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/ansi_color/docs/screenshot_macos_terminal_app.png?raw=true) |
//! |:--:|
//! | *Running on macOS terminal app (note ANSI 256 runtime detection)* |
//!
//! | ![macOS screenshot iTerm app](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/ansi_color/docs/screenshot_macos_iterm_app.png?raw=true) |
//! |:--:|
//! | *Running on macOS iTerm app (note Truecolor runtime detection)* |
//!
//! # Changelog
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_ansi_color)
//! to see how the library has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom).
//!   Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer [site](https://developerlife.com/).
//! - If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).
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
//! #     fg_red, dim, fg_color, tui_color, new_style, ast,
//! #     RgbValue, ASTStyle, AnsiStyledText, ASTColor,
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
//!
//! Please a look at the
//! [`main` example](https://github.com/r3bl-org/r3bl_ansi_color/blob/main/examples/main.rs) to get a
//! better idea of how to use this crate.
//!
//! # Build, run, test tasks
//!
//! ## Prerequisites
//!
//! 🌠 In order for these to work you have to install the Rust toolchain and `nu` and
//! `cargo-watch`:
//!
//! 1. Install the Rust toolchain using `rustup` by following the instructions [here](https://rustup.rs/).
//! 1. Install `cargo-watch` using `cargo install cargo-watch`.
//! 1. Install `flamegraph` using `cargo install flamegraph`.
//! 1. Install [`nu`](https://www.nushell.sh/) on your system using `cargo install nu`. It
//!    is available for Linux, macOS, and Windows. And it is written in Rust.
//!
//! ## Nushell commands
//!
//! | Command                         | Description                                                                                              |
//! |---------------------------------|------------------------------------------------------------------------------------------------------|
//! | `fish run.fish build`           | Build the project                                                                                        |
//! | `fish run.fish clean`           | Clean the project                                                                                        |
//! | `fish run.fish run`             | Run examples                                                                                             |
//! | `fish run.fish run-release`     | Run examples with release flag                                                                           |
//! | `fish run.fish run-flamegraph`  | Run examples with flamegraph profiling                                                                   |
//! | `fish run.fish test`            | Run tests                                                                                                |
//! | `fish run.fish clippy`          | Run clippy                                                                                               |
//! | `fish run.fish docs`            | Build docs                                                                                               |
//! | `fish run.fish serve-docs`      | Serve docs. Useful if you SSH into a remote machine via `VSCode` and want to view the docs locally      |
//! | `fish run.fish upgrade-deps`    | Upgrade dependencies                                                                                     |
//! | `fish run.fish rustfmt`         | Run rustfmt                                                                                              |
//!
//! The following commands will watch for changes in the source folder and re-run:
//!
//! | Command                                             | Description                         |
//! |-----------------------------------------------------|-------------------------------------|
//! | `fish run.fish watch-run`                          | Watch run                           |
//! | `fish run.fish watch-all-tests`                    | Watch all tests                     |
//! | `fish run.fish watch-one-test <test_name>`         | Watch one test                      |
//! | `fish run.fish watch-clippy`                       | Watch clippy                        |
//! | `fish run.fish watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
//!
//! # References
//!
//! - [ANSI Escape Codes](https://notes.burke.libbey.me/ansi-escape-codes/)
//! - [ASCII Table](https://www.asciitable.com/)
//! - [Xterm 256color Chart](https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg)
//! - [256 Colors Cheat Sheet](https://www.ditig.com/256-colors-cheat-sheet)
//! - [List of ANSI Color Escape Sequences](https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences)
//! - [Color Metric](https://www.compuphase.com/cmetric.htm)
//!
//! # Why make a new crate for this?
//!
//! - There are a few crates on crates.io that do similar things but they don't amenable
//!   licenses.
//! - Other crates simply ignore ANSI 256 colors and only support truecolor, even when
//!   they claim that they support it.
//! - And there are other crates which don't correctly report that macOS Terminal.app does
//!   not support truecolor and only supports ANSI 256 color.
//!
//! Here are some relevant links:
//! <!-- cspell:disable -->
//! 1. [Issue 47: `concolor`](https://github.com/rust-cli/concolor/issues/47)
//! 1. [`anstream` documentation](https://docs.rs/anstream/latest/anstream/)
//! 1. [`colorchoice` documentation](https://docs.rs/colorchoice/latest/colorchoice/)
//! 1. [`colorchoice-clap` documentation](https://docs.rs/colorchoice-clap/latest/colorchoice_clap/)
//! 1. [`term_supports_ansi_color` function](https://docs.rs/anstyle-query/latest/anstyle_query/fn.term_supports_ansi_color.html)
//! 1. [`anstyle-query` crate](https://crates.io/crates/anstyle-query)
//! 1. [`supports-color` documentation](https://docs.rs/supports-color/2.0.0/supports_color/)
//! 1. [`r3bl_ansi_color` crate](https://crates.io/crates/r3bl_ansi_color) (the source in
//!    `ansi_color` folder is this crate)
//! 1. [`colored` crate](https://crates.io/crates/colored)
//! <!-- cspell:enable -->

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

// Attach.
pub mod ansi_escape_codes;
pub mod ansi_styled_text;
pub mod ast_color;
pub mod convert;
pub mod detect_color_support;
pub mod transform_color;

pub use ansi_escape_codes::*;
pub use ansi_styled_text::*;
pub use ast_color::*;
pub use convert::*;
pub use detect_color_support::*;
pub use transform_color::*;
