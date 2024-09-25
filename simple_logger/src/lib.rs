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
//! - [How to customize or change logging implementation](#how-to-customize-or-change-logging-implementation)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! The simplest way to use this crate to log things and simply use the logging
//! facilities, is to use the
//! [`r3bl_rs_utils_core`](https://crates.io/crates/r3bl_rs_utils_core) crate, and not
//! this crate directly.
//! - Look at the
//!   [`r3bl_rs_utils_core::try_to_set_log_level`](https://docs.rs/r3bl_rs_utils_core/latest/r3bl_rs_utils_core/utils/file_logging/fn.try_to_set_log_level.html)
//!   function in the `r3bl_rs_utils_core` crate as the main entry point.
//! - By default, logging is disabled even if you call all the functions in the
//!   `file_logger` module in the `r3bl_rs_utils_core` crate: `log_debug`, `log_info`,
//!   `log_trace`, etc.
//!
//! # Changelog
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_simple_logger)
//! to see how the library has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom). Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer [site](https://developerlife.com/). Please consider subscribing to our [newsletter](https://developerlife.com/subscribe.html).
//! - If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).
//!
//! # How to customize or change logging implementation
//!
//! Under the hood the [`simplelog`](https://crates.io/crates/simplelog) crate is forked
//! and modified for use here.
//!
//! The following are details for people who want to work on changing the underlying
//! behavior of the logging engine itself, and *not* for folks who just want to use this
//! crate.
//!
//! `r3bl_simple_logger` provides a series of logging facilities, that can be easily
//! combined.
//!
//! - `SimpleLogger` (very basic logger that logs to stdout)
//! - `TermLogger` (advanced terminal logger, that splits to stdout/err and has color
//!   support) (can be excluded on unsupported platforms)
//! - `WriteLogger` (logs to a given struct implementing `Write`, e.g. a file)
//! - `CombinedLogger` (can be used to form combinations of the above loggers)
//! - `TestLogger` (specialized logger for tests. Uses print!() / println!() for tests to
//!   be able to capture the output)
//!
//! Only one Logger should be initialized of the start of your program through the
//! `Logger::init(...)` method. For the actual calling syntax take a look at the
//! documentation of the specific implementation(s) you want to use.

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

mod config;
mod loggers;

pub use log::{Level, LevelFilter, Log, Metadata, Record, SetLoggerError};
pub use termcolor::{Color, ColorChoice};

pub use self::{config::{format_description,
                        Config,
                        ConfigBuilder,
                        FormatItem,
                        LevelPadding,
                        TargetPadding,
                        ThreadLogMode,
                        ThreadPadding},
               loggers::{CombinedLogger,
                         SimpleLogger,
                         TermLogger,
                         TerminalMode,
                         TestLogger,
                         WriteLogger}};

/// Trait to have a common interface to obtain the Level of Loggers
///
/// Necessary for CombinedLogger to calculate
/// the lowest used Level.
///
pub trait SharedLogger: Log {
    /// Returns the set Level for this Logger
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    /// let logger = SimpleLogger::new(LevelFilter::Info, Config::default());
    /// println!("{}", logger.level());
    /// # }
    /// ```
    fn level(&self) -> LevelFilter;

    /// Inspect the config of a running Logger
    ///
    /// An Option is returned, because some Logger may not contain a Config
    ///
    /// # Examples
    ///
    /// ```
    /// # extern crate r3bl_simple_logger;
    /// # use r3bl_simple_logger::*;
    /// # fn main() {
    /// let logger = SimpleLogger::new(LevelFilter::Info, Config::default());
    /// println!("{:?}", logger.config());
    /// # }
    /// ```
    fn config(&self) -> Option<&Config>;

    /// Returns the logger as a Log trait object
    fn as_log(self: Box<Self>) -> Box<dyn Log>;
}

#[cfg(test)]
mod tests {
    use std::{fs::File, io::Read};

    use log::*;

    use super::*;

    #[test]
    fn test() {
        let mut i = 0;

        CombinedLogger::init({
            let mut vec = Vec::new();
            let mut conf_builder = ConfigBuilder::new();

            let conf_thread_name = ConfigBuilder::new()
                .set_time_level(LevelFilter::Off)
                .set_thread_level(LevelFilter::Error)
                .set_thread_mode(ThreadLogMode::Names)
                .build();

            vec.push(WriteLogger::new(
                LevelFilter::Error,
                conf_thread_name,
                File::create("thread_naming.log").unwrap(),
            ) as Box<dyn SharedLogger>);

            for elem in [
                LevelFilter::Off,
                LevelFilter::Trace,
                LevelFilter::Debug,
                LevelFilter::Info,
                LevelFilter::Warn,
                LevelFilter::Error,
            ] {
                let conf = conf_builder
                    .set_location_level(elem)
                    .set_target_level(elem)
                    .set_max_level(elem)
                    .set_time_level(elem)
                    .build();
                i += 1;

                //error
                vec.push(SimpleLogger::new(LevelFilter::Error, conf.clone())
                    as Box<dyn SharedLogger>);
                vec.push(TermLogger::new(
                    LevelFilter::Error,
                    conf.clone(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ) as Box<dyn SharedLogger>);
                vec.push(WriteLogger::new(
                    LevelFilter::Error,
                    conf.clone(),
                    File::create(format!("error_{}.log", i)).unwrap(),
                ) as Box<dyn SharedLogger>);
                vec.push(TestLogger::new(LevelFilter::Error, conf.clone()));

                //warn
                vec.push(SimpleLogger::new(LevelFilter::Warn, conf.clone())
                    as Box<dyn SharedLogger>);
                vec.push(TermLogger::new(
                    LevelFilter::Warn,
                    conf.clone(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ) as Box<dyn SharedLogger>);
                vec.push(WriteLogger::new(
                    LevelFilter::Warn,
                    conf.clone(),
                    File::create(format!("warn_{}.log", i)).unwrap(),
                ) as Box<dyn SharedLogger>);
                vec.push(TestLogger::new(LevelFilter::Warn, conf.clone()));

                //info
                vec.push(SimpleLogger::new(LevelFilter::Info, conf.clone())
                    as Box<dyn SharedLogger>);
                vec.push(TermLogger::new(
                    LevelFilter::Info,
                    conf.clone(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ) as Box<dyn SharedLogger>);
                vec.push(WriteLogger::new(
                    LevelFilter::Info,
                    conf.clone(),
                    File::create(format!("info_{}.log", i)).unwrap(),
                ) as Box<dyn SharedLogger>);
                vec.push(TestLogger::new(LevelFilter::Info, conf.clone()));

                //debug
                vec.push(SimpleLogger::new(LevelFilter::Debug, conf.clone())
                    as Box<dyn SharedLogger>);
                vec.push(TermLogger::new(
                    LevelFilter::Debug,
                    conf.clone(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ) as Box<dyn SharedLogger>);
                vec.push(WriteLogger::new(
                    LevelFilter::Debug,
                    conf.clone(),
                    File::create(format!("debug_{}.log", i)).unwrap(),
                ) as Box<dyn SharedLogger>);
                vec.push(TestLogger::new(LevelFilter::Debug, conf.clone()));

                //trace
                vec.push(SimpleLogger::new(LevelFilter::Trace, conf.clone())
                    as Box<dyn SharedLogger>);
                vec.push(TermLogger::new(
                    LevelFilter::Trace,
                    conf.clone(),
                    TerminalMode::Mixed,
                    ColorChoice::Auto,
                ) as Box<dyn SharedLogger>);
                vec.push(WriteLogger::new(
                    LevelFilter::Trace,
                    conf.clone(),
                    File::create(format!("trace_{}.log", i)).unwrap(),
                ) as Box<dyn SharedLogger>);
                vec.push(TestLogger::new(LevelFilter::Trace, conf.clone()));
            }

            vec
        })
        .unwrap();

        error!("Test Error");
        warn!("Test Warning");
        info!("Test Information");
        debug!("Test Debug");
        trace!("Test Trace");

        let mut thread_naming = String::new();
        File::open("thread_naming.log")
            .unwrap()
            .read_to_string(&mut thread_naming)
            .unwrap();

        if let Some(name) = std::thread::current().name() {
            assert!(thread_naming.contains(&format!("({})", name)));
        }

        for j in 1..i {
            let mut error = String::new();
            File::open(format!("error_{}.log", j))
                .unwrap()
                .read_to_string(&mut error)
                .unwrap();
            let mut warn = String::new();
            File::open(format!("warn_{}.log", j))
                .unwrap()
                .read_to_string(&mut warn)
                .unwrap();
            let mut info = String::new();
            File::open(format!("info_{}.log", j))
                .unwrap()
                .read_to_string(&mut info)
                .unwrap();
            let mut debug = String::new();
            File::open(format!("debug_{}.log", j))
                .unwrap()
                .read_to_string(&mut debug)
                .unwrap();
            let mut trace = String::new();
            File::open(format!("trace_{}.log", j))
                .unwrap()
                .read_to_string(&mut trace)
                .unwrap();

            assert!(error.contains("Test Error"));
            assert!(!error.contains("Test Warning"));
            assert!(!error.contains("Test Information"));
            assert!(!error.contains("Test Debug"));
            assert!(!error.contains("Test Trace"));

            assert!(warn.contains("Test Error"));
            assert!(warn.contains("Test Warning"));
            assert!(!warn.contains("Test Information"));
            assert!(!warn.contains("Test Debug"));
            assert!(!warn.contains("Test Trace"));

            assert!(info.contains("Test Error"));
            assert!(info.contains("Test Warning"));
            assert!(info.contains("Test Information"));
            assert!(!info.contains("Test Debug"));
            assert!(!info.contains("Test Trace"));

            assert!(debug.contains("Test Error"));
            assert!(debug.contains("Test Warning"));
            assert!(debug.contains("Test Information"));
            assert!(debug.contains("Test Debug"));
            assert!(!debug.contains("Test Trace"));

            assert!(trace.contains("Test Error"));
            assert!(trace.contains("Test Warning"));
            assert!(trace.contains("Test Information"));
            assert!(trace.contains("Test Debug"));
            assert!(trace.contains("Test Trace"));
        }
    }
}
