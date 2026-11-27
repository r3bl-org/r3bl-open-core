// Copyright (c) 2023-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module provides the implementation for the [`try_choose()`] function, which
//! creates an interactive selection UI in the terminal. It allows users to select one or
//! multiple items from a list using keyboard navigation.
//!
//! The implementation is cross-platform (macOS, Linux, Windows) and automatically adapts
//! to terminal capabilities (color support, input methods).
//!
//! # Examples
//!
//! ```no_run
//! # use r3bl_tui::*;
//! # use r3bl_tui::readline_async::*;
//! # async fn example() -> miette::Result<()> {
//! let mut io_devices = DefaultIoDevices::default();
//! match try_choose(
//!     "Select an item",
//!     &["option 1", "option 2", "option 3"],
//!     None,  // default height
//!     None,  // default width
//!     HowToChoose::Single,
//!     StyleSheet::default(),
//!     io_devices.as_mut_tuple(),
//! ) {
//!     TuiAvailability::Available(choice_future) => {
//!         let selection = choice_future.await?;
//!     }
//!     it => return it.into_err(),
//! }
//! # ok!()
//! # }
//! ```
//!
//! For complete examples, see [`tui/examples/choose_interactive.rs`].
//!
//! # Styling
//!
//! The choose UI supports customizable styling through the [`StyleSheet`] struct.
//! Built-in styles include:
//! - [`StyleSheet::default()`] - Default styling
//! - [`StyleSheet::sea_foam_style()`] - Sea foam color theme
//! - [`StyleSheet::hot_pink_style()`] - Hot pink color theme
//!
//! You can also create custom styles by constructing a [`StyleSheet`] with your own
//! [`TuiStyle`] settings. See the examples for detailed styling demonstrations.
//!
//! [`StyleSheet`]: crate::StyleSheet
//! [`try_choose()`]: crate::try_choose
//! [`tui/examples/choose_interactive.rs`]:
//!     https://github.com/r3bl-org/r3bl-open-core/tree/main/tui/examples/choose_interactive.rs
//! [`TuiStyle`]: crate::TuiStyle

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

// Attach. Upward #[macro_use] propagation (must precede modules below that invoke their
// macros).
#[macro_use]
mod crossterm_macros;

// Attach.
mod event_loop;
mod function_component;
mod keypress_reader_sync;
mod scroll;
mod select_component;
mod state;
mod style;

// Re-export.
pub use event_loop::*;
pub use function_component::*;
pub use keypress_reader_sync::*;
pub use scroll::*;
pub use select_component::*;
pub use state::*;
pub use style::*;

#[cfg(any(all(unix, doc), test))]
mod choose_integration_tests;
