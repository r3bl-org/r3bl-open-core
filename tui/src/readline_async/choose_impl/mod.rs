// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Choose implementation module
//!
//! This module provides the implementation for the [`crate::choose()`] function, which
//! creates an interactive selection UI in the terminal. It allows users to select one
//! or multiple items from a list using keyboard navigation.
//!
//! The implementation is cross-platform (macOS, Linux, Windows) and automatically adapts
//! to terminal capabilities (color support, input methods).
//!
//! # Quick example
//!
//! ```no_run
//! # use r3bl_tui::*;
//! # use r3bl_tui::readline_async::*;
//! # async fn example() -> miette::Result<()> {
//! let mut io_devices = DefaultIoDevices::default();
//! let selection = choose(
//!     "Select an item",
//!     &["option 1", "option 2", "option 3"],
//!     None,  // default height
//!     None,  // default width
//!     HowToChoose::Single,
//!     StyleSheet::default(),
//!     io_devices.as_mut_tuple(),
//! ).await?;
//! # Ok(())
//! # }
//! ```
//!
//! For complete examples, see `tui/examples/choose.rs`.
//!
//! # Styling
//!
//! The choose UI supports customizable styling through the [`StyleSheet`] struct.
//! Built-in styles include:
//! - [`StyleSheet::default()`] - Default styling
//! - [`StyleSheet::sea_foam_style()`] - Sea foam color theme
//! - [`StyleSheet::hot_pink_style()`] - Hot pink color theme
//!
//! You can also create custom styles by constructing a `StyleSheet` with your own
//! [`TuiStyle`] settings. See the examples for detailed styling demonstrations.

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
