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

//! # Why R3BL?
//!
//! <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true" height="256px">
//!
//! <!-- R3BL TUI library & suite of apps focused on developer productivity -->
//!
//! <span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span style="color:#F92363">
//! </span><span style="color:#F82067">T</span><span style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span style="color:#F31874">
//! </span><span style="color:#F11678">l</span><span style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span style="color:#E10895">
//! </span><span style="color:#DE0799">&amp;</span><span style="color:#DB069E">
//! </span><span style="color:#D804A2">s</span><span style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span style="color:#C801B6">
//! </span><span style="color:#C501B9">o</span><span style="color:#C101BD">f</span><span style="color:#BD01C1">
//! </span><span style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span style="color:#AA03D2">
//! </span><span style="color:#A603D5">f</span><span style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span style="color:#890DE8">
//! </span><span style="color:#850FEB">o</span><span style="color:#8111ED">n</span><span style="color:#7C13EF">
//! </span><span style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span style="color:#572CFC">r</span><span style="color:#532FFD">
//! </span><span style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>
//!
//! We are working on building command line apps in Rust which have rich text user interfaces (TUI).
//! We want to lean into the terminal as a place of productivity, and build all kinds of awesome
//! apps for it.
//!
//! 1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
//!    development w/ a twist: taking concepts that work really well for the frontend mobile and web
//!    development world and re-imagining them for TUI & Rust.
//!
//!    - Taking inspiration from things like [React](https://react.dev/),
//!      [SolidJS](https://www.solidjs.com/),
//!      [Elm](https://guide.elm-lang.org/architecture/),
//!      [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack
//!      Compose](https://developer.android.com/compose),
//!      [JSX](https://ui.dev/imperative-vs-declarative-programming),
//!      [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can
//!      be run in parallel & concurrent via [Tokio](https://crates.io/crates/tokio)).
//!    - Even the thread running the main event loop doesn't block since it is async.
//!    - Using proc macros to create DSLs to implement something inspired by
//!      [CSS](https://www.w3.org/TR/CSS/#css) &
//!      [JSX](https://ui.dev/imperative-vs-declarative-programming).
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
//! - [Learn how these crates are built, provide feedback](#learn-how-these-crates-are-built-provide-feedback)
//! - [async_stream_fixtures](#async_stream_fixtures)
//! - [stdout_fixtures](#stdout_fixtures)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! This is a test fixtures library that provides reusable components for testing. It is
//! meant to be used by all the crates in the `r3bl-open-core` monorepo. This crate is
//! intended to be a
//! [`dev-dependency`](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#dev-dependencies)
//! for other creates in the monorepo.
//!
//! It provides fixtures to test async streams and stdout. This allows TUI frameworks to
//! be tested "end to end".
//! 1. The async stream fixtures are used to test the input stream of a TUI framework.
//! 2. The stdout fixtures are used to test the output of a TUI framework.
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_test_fixtures)
//! to see how the library has evolved over time.
//!
//! # Changelog
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_test_fixtures) to
//! see how the library has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom). Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer [site](https://developerlife.com/). Please consider subscribing to our [newsletter](https://developerlife.com/subscribe.html).
//! - If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).
//!
//! # async_stream_fixtures
//!
//! Here's an example of how create a stream of `T` from a `Vec<T>`.
//!
//! ```
//! #[tokio::test]
//! async fn test_gen_input_stream() {
//!     use futures_util::StreamExt;
//!     use r3bl_test_fixtures::gen_input_stream;
//!
//!     let mut input_stream = gen_input_stream(vec![1, 2, 3]);
//!     for _ in 1..=3 {
//!         input_stream.next().await;
//!     }
//!     pretty_assertions::assert_eq!(input_stream.next().await, None);
//! }
//! ```
//!
//! Here's another example of how to use this with a delay.
//!
//! ```
//! #[tokio::test]
//! async fn test_gen_input_stream_with_delay() {
//!     use futures_util::StreamExt;
//!     use r3bl_test_fixtures::gen_input_stream_with_delay;
//!
//!     let delay = 100;
//!
//!     // Start timer.
//!     let start_time = std::time::Instant::now();
//!
//!     let mut input_stream = gen_input_stream_with_delay(vec![1, 2, 3], Duration::from_millis(delay));
//!     for _ in 1..=3 {
//!         input_stream.next().await;
//!     }
//!
//!     // End timer.
//!     let end_time = std::time::Instant::now();
//!
//!     pretty_assertions::assert_eq!(input_stream.next().await, None);
//!
//!     assert!(end_time - start_time >= Duration::from_millis(delay * 3));
//! }
//! ```
//!
//! # stdout_fixtures
//!
//! Here's an example of how to use this.
//!
//! ```
//! #[tokio::test]
//! async fn test_stdout_mock_no_strip_ansi() {
//!     use strip_ansi_escapes::strip;
//!
//!     use super::*;
//!     use std::{
//!         io::{Result, Write},
//!         sync::Arc,
//!     };
//!
//!     let mut stdout_mock = StdoutMock::default();
//!     let stdout_mock_clone = stdout_mock.clone(); // Points to the same inner value as `stdout_mock`.
//!
//!     let normal_text = "hello world";
//!
//!     stdout_mock.write_all(normal_text.as_bytes()).unwrap();
//!     stdout_mock.flush().unwrap();
//!
//!     pretty_assertions::assert_eq!(stdout_mock.get_copy_of_buffer_as_string(), normal_text);
//!     pretty_assertions::assert_eq!(
//!         stdout_mock_clone.get_copy_of_buffer_as_string(),
//!         normal_text
//!     );
//! }
//! ```

// Attach sources.
pub mod input_device_fixtures;
pub mod output_device_fixtures;
pub mod tcp_stream_fixtures;

// Re-export.
pub use input_device_fixtures::*;
pub use output_device_fixtures::*;
pub use tcp_stream_fixtures::*;
