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

//! # Why R3BL?
//!
//! <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">
//!
//! <!-- R3BL TUI library and suite of apps focused on developer productivity -->
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
//! interfaces (TUI). We want to lean into the terminal as a place of productivity and
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
//! 2. 🌎 We are building apps to enhance developer productivity and workflows.
//!
//!    - The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a
//!      single terminal window). Rather, it is to build a set of integrated "apps" (or
//!      "tasks") that run in the same process that renders to one terminal window.
//!    - Inside of this terminal window, we can implement things like "app" switching,
//!      routing, tiling layout, stacking layout, etc. so that we can manage a lot of TUI
//!      "apps" (which are tightly integrated) that are running in the same process, in
//!      the same window. So you can imagine that all these "apps" have shared application
//!      state. Each "app" may also have its own local application state.
//!    - Here are some example "apps" we plan to build (for which this infrastructure acts
//!      as the open source engine):
//!      1. Multi-user text editors w/ syntax highlighting.
//!      2. Integrations w/ GitHub issues.
//!      3. Integrations w/ calendar, email, contacts APIs.
//!
//! All the crates in the `r3bl-open-core`
//! [repo](https://github.com/r3bl-org/r3bl-open-core/) provide lots of useful
//! functionality to help you build TUI (text user interface) apps, along with general
//! niceties and ergonomics that all Rustaceans 🦀 can enjoy 🎉.
//!
//! # Table of contents
//!
//! <!-- TOC -->
//!
//! - [Introduction](#introduction)
//! - [Examples](#examples)
//! - [Changelog](#changelog)
//! - [Learn how these crates are built, provide
//!   feedback](#learn-how-these-crates-are-built-provide-feedback)
//! - [How to use it as a library?](#how-to-use-it-as-a-library)
//! - [APIs](#apis)
//!     - [choose](#choose)
//! - [Component styling](#component-styling)
//!     - [Choose one of the 3 built-in styles](#choose-one-of-the-3-built-in-styles)
//!     - [Create your style](#create-your-style)
//! - [Build, run, test tasks](#build-run-test-tasks)
//!     - [Prerequisites](#prerequisites)
//!     - [Nushell scripts to build, run, test,
//!       etc.](#nu-shell-scripts-to-build-run-test-etc)
//! - [References](#references)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! `choose_impl` allows you to add simple interactivity to your CLI app. It is not a full
//! TUI, neither is it like [`crate::ReadlineAsyncContext`]. It simply allows you to provide
//! a list of items and ask the user to choose one or more of them.
//!
//! # Examples
//!
//! To run the examples, you can run `nu run.nu examples` in the `terminal_async` folder.
//!
//! # Changelog
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_tui) to
//! see how the library has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom).
//!   Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer [site](https://developerlife.com/).
//! - If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).
//!
//! # How to use it as a library?
//!
//! Here's a demo of this library in action.
//!
//! <video width="100%" controls>
//!   <source src="https://github.com/r3bl-org/r3bl-open-core/assets/22040032/46850043-4973-49fa-9824-58f32f21e96e" type="video/mp4"/>
//! </video>
//!
//! To install the crate as a library, add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! r3bl_core = "*" # Get the latest version at the time you get this.
//! r3bl_tui = "*" # Get the latest version at the time you get this.
//! ```
//!
//! The following example illustrates how you can use this as a library. The function that
//! does the work of rendering the UI is called [`crate::choose()`]. It takes a list of
//! items and returns the selected item or items (depending on the selection mode). If the
//! user does not select anything, it returns `None`. The function also takes the maximum
//! height and width of the display, and the selection mode (single select or multiple
//! select).
//!
//! It works on macOS, Linux, and Windows. And is aware of the terminal color output
//! limitations of each. For e.g., it uses Windows API on Windows for keyboard input. And
//! on macOS Terminal.app it restricts color output to a 256-color palette.
//!
//! ```no_run
//! // This example requires an interactive terminal for user selection
//! # use r3bl_tui::*;
//! # use r3bl_tui::readline_async::*;
//! # use std::io::Result;
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     // Get display size.
//!     let max_width_col_count: usize = (get_size().map(|it| *it.col_width).unwrap_or(ch(80))).into();
//!     let max_height_row_count: usize = 5;
//!
//!     let mut default_io_devices = DefaultIoDevices::default();
//!     let user_input = choose(
//!         "Select an item",
//!         &[
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ],
//!         Some(height(max_height_row_count)),
//!         Some(width(max_width_col_count)),
//!         HowToChoose::Single,
//!         StyleSheet::default(),
//!         default_io_devices.as_mut_tuple(),
//!     ).await?;
//!
//!     match user_input.first() {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # APIs
//!
//! We provide a single function:
//!
//! - [`crate::choose()`]: Use this API if you want to display a list of items in an async
//!   context, with a single or multi-line header.
//!
//! ## choose
//!
//! Use this async API if you want to display a list of items with a single or multi-line
//! header.
//!
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/0ae722bb-8cd1-47b1-a293-1a96e84d24d0)
//!
//! [`crate::choose()`] code example:
//!
//! ```no_run
//! // This example requires an interactive terminal for user selection
//! # use r3bl_tui::*;
//! # use r3bl_tui::readline_async::*;
//! # use std::io::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Get display size.
//!     let max_height_row_count: usize = 5;
//!     let user_input = choose(
//!         "Select an item",
//!         &[
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ],
//!         None,
//!         None,
//!         HowToChoose::Single,
//!         StyleSheet::default(),
//!              (&mut OutputDevice::new_stdout(), &mut InputDevice::new_event_stream(), None),
//!     ).await;
//!
//!     match user_input {
//!         Ok(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         _ => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! # Component styling
//!
//! ## Choose one of the 3 built-in styles
//!
//! Built-in styles are called [`default`](StyleSheet::default()),
//! [`sea_foam_style`](StyleSheet::sea_foam_style()), and
//! [`hot_pink_style`](StyleSheet::hot_pink_style()).
//!
//! Default style:
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/eaf990a4-1c33-4783-9f39-82af42568183)
//!
//! `sea_foam_style`:
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/fc414f56-2f72-4d3a-86eb-bfd732b66bd1)
//!
//! `hot_pink_style`:
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/06c155f9-11a9-416d-8056-cb4c741ac3d7)
//!
//! To use one of the built-in styles, pass it as an argument to the `choose` function.
//!
//! ```no_run
//! // This example requires an interactive terminal for user selection
//! # use r3bl_tui::*;
//! # use r3bl_tui::readline_async::*;
//! # use std::io::Result;
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     // 🎨 Uncomment the lines below to choose the other 2 built-in styles.
//!     // let default_style = StyleSheet::default();
//!     // let hot_pink_style = StyleSheet::hot_pink_style();
//!     let sea_foam_style = StyleSheet::sea_foam_style();
//!
//!     let max_width_col_count: usize = get_size().map(|it| *it.col_width).unwrap_or(ch(80)).into();
//!     let max_height_row_count: usize = 5;
//!
//!     let mut default_io_devices = DefaultIoDevices::default();
//!     let user_input = choose(
//!         "Select an item",
//!         &[
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ],
//!         Some(height(max_height_row_count)),
//!         Some(width(max_width_col_count)),
//!         HowToChoose::Single,
//!         sea_foam_style,  // 🖌️ or default_style or hot_pink_style
//!         default_io_devices.as_mut_tuple(),
//!     ).await?;
//!
//!     match user_input.first() {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Create your style
//!
//! To create your style, you need to create a `StyleSheet` struct and pass it as an
//! argument to the `choose` function.
//!
//! ```no_run
//! // This example requires an interactive terminal for user selection
//! use std::io::Result;
//! use r3bl_tui::{
//!     AnsiStyledText, ASTColor, tui_color, TuiStyle,
//!     height, width
//! };
//! use r3bl_tui::readline_async::{
//!     State, choose_impl::style::StyleSheet,
//!     choose, HowToChoose, DefaultIoDevices
//! };
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!    // This is how you can define your custom style.
//!    // For each Style struct, you can define different style overrides.
//!    // Please take a look at the Style struct to see what you can override.
//!    let my_custom_style = StyleSheet {
//!       focused_and_selected_style: TuiStyle {
//!             color_fg: tui_color!(255, 244, 0).into(),
//!             color_bg: tui_color!(15, 32, 66).into(),
//!             ..TuiStyle::default()
//!       },
//!       focused_style: TuiStyle {
//!             color_fg: tui_color!(255, 244, 0).into(),
//!             ..TuiStyle::default()
//!       },
//!       unselected_style: TuiStyle { ..TuiStyle::default() },
//!       selected_style: TuiStyle {
//!             color_fg: tui_color!(203, 170, 250).into(),
//!             color_bg: tui_color!(15, 32, 66).into(),
//!             ..TuiStyle::default()
//!       },
//!       header_style: TuiStyle {
//!             color_fg: tui_color!(171, 204, 242).into(),
//!             color_bg: tui_color!(31, 36, 46).into(),
//!             ..TuiStyle::default()
//!       },
//!    };
//!
//!    // Then pass `my_custom_style` as the last argument to the `choose` function.
//!    let mut default_io_devices = DefaultIoDevices::default();
//!    let user_input = choose(
//!       "Multiple select",
//!       &["item 1 of 3", "item 2 of 3", "item 3 of 3"],
//!       Some(height(6)), // max_height_row_count
//!       Some(width(80)), // max_width_col_count
//!       HowToChoose::Multiple,
//!       my_custom_style,
//!       default_io_devices.as_mut_tuple()
//!    ).await?;
//!
//!    match user_input.first() {
//!       Some(it) => {
//!          println!("User selected: {:?}", it);
//!       }
//!       None => println!("User did not select anything"),
//!    }
//!
//!    Ok(())
//! }
//! ```
//!
//! # Build, run, test tasks
//!
//! ## Prerequisites
//!
//! 🌠 For these to work, you have to install the Rust toolchain, `nu`, `cargo-watch`,
//! `bat`, and `flamegraph` on your system. Here are the instructions:
//!
//! 1. Install the Rust toolchain using `rustup` by following the instructions [here](https://rustup.rs/).
//! 1. Install `cargo-watch` using `cargo install cargo-watch`.
//! 1. Install `flamegraph` using `cargo install flamegraph`.
//! 1. Install `bat` using `cargo install bat`.
//! 1. Install [`nu`](https://crates.io/crates/nu) shell on your system using `cargo
//!    install nu`. It is available for Linux, macOS, and Windows.
//!
//! ## Nushell scripts to build, run, test, etc.
//!
//! Go to the `tui` folder and run the commands below. These commands are defined in the
//! `./run` folder.
//!
//! | Command                                | Description                                |
//! | -------------------------------------- | ------------------------------------------ |
//! | `nu run.nu examples`                      | Run examples in the `./examples` folder    |
//! | `nu run.nu piped`                         | Run binary with piped input                |
//! | `nu run.nu build`                         | Build                                      |
//! | `nu run.nu clean`                         | Clean                                      |
//! | `nu run.nu all`                           | All                                        |
//! | `nu run.nu examples-with-flamegraph-profiling` | Run examples with flamegraph profiling |
//! | `nu run.nu test`                          | Run tests                                  |
//! | `nu run.nu clippy`                        | Run clippy                                 |
//! | `nu run.nu docs`                          | Build docs                                 |
//! | `nu run.nu serve-docs`                    | Serve docs over `VSCode` Remote SSH session. |
//! | `nu run.nu upgrade-deps`                  | Upgrade deps                               |
//! | `nu run.nu rustfmt`                       | Run rustfmt                                |
//!
//! The following commands will watch for changes in the source folder and re-run:
//!
//! | Command                                             | Description                        |
//! | --------------------------------------------------- | ---------------------------------- |
//! | `nu run.nu watch-examples`                             | Watch run examples                 |
//! | `nu run.nu watch-all-tests`                            | Watch all test                     |
//! | `nu run.nu watch-one-test <test_name>`                 | Watch one test                     |
//! | `nu run.nu watch-clippy`                               | Watch clippy                       |
//! | `nu run.nu watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
//!
//! There's also a `run` script at the **top level folder** of the repo. It is intended to
//! be used in a CI/CD environment w/ all the required arguments supplied or in
//! interactive mode, where the user will be prompted for input.
//!
//! | Command                       | Description                        |
//! | ----------------------------- | ---------------------------------- |
//! | `nu run.nu all`                  | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD |
//! | `nu run.nu build-full`           | This will build all the crates in the Rust workspace. It will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
//! | `nu run.nu install-cargo-tools`  | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`,and `flamegraph` will all be installed in one go) |
//! | `nu run.nu check-licenses`       | Use `cargo-deny` to audit all licenses used in the Rust workspace |
//!
//! # References
//!
//! CLI UX guidelines:
//!
//! - [Handling Arguments](https://rust-cli-recommendations.sunshowers.io/handling-arguments.html)
//! - [Configuration](https://rust-cli-recommendations.sunshowers.io/configuration.html)
//! - [Hierarchical Config](https://rust-cli-recommendations.sunshowers.io/hierarchical-config.html)
//! - [Hierarchical Config](https://rust-cli-recommendations.sunshowers.io/hierarchical-config.html)
//! - [Clap Derive Overview](https://docs.rs/clap/latest/clap/_derive/#overview)
//! - [Command Line Interface Guidelines](https://clig.dev/#foreword)
//!
//! ANSI escape codes:
//!
//! - [ANSI Escape Codes Notes](https://notes.burke.libbey.me/ansi-escape-codes/)
//! - [ANSI Escape Code - Wikipedia](https://en.wikipedia.org/wiki/ANSI_escape_code)
//! - [ASCII Table](https://www.asciitable.com/)
//! - [Xterm 256 Color Chart](https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg)
//! - [256 Colors Cheat Sheet](https://www.ditig.com/256-colors-cheat-sheet)
//! - [List of ANSI Color Escape Sequences - Stack Overflow](https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences)
//! - [Color Metric](https://www.compuphase.com/cmetric.htm)

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

pub mod components;
pub mod crossterm_macros;
pub mod event_loop;
pub mod function_component;
pub mod keypress_reader_sync;
pub mod scroll;
pub mod state;
pub mod style;

pub use components::*;
pub use event_loop::*;
pub use function_component::*;
pub use keypress_reader_sync::*;
pub use scroll::*;
pub use state::*;
pub use style::*;

/// Enable file logging. You can use `tail -f log.txt` to watch the logs.
pub const DEVELOPMENT_MODE: bool = false;
