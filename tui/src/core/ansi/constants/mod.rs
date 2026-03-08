// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words VMIN, VTIME

//! Centralized [`ANSI`]/[`VT-100`] escape sequence constants.
//!
//! This module consolidates all [`ANSI`] terminal constants into a single, discoverable
//! location organized by protocol type.
//!
//! # Design
//!
//! Constants are split into three tiers based on what they represent.
//!
//! ## Tier 1 - Foundational parts
//!
//! Manual `pub const` definitions for single bytes, characters, or byte slices
//! representing protocol building blocks. Values are visible at a glance, and there is no
//! macro or generator function usage.
//!
//! Each definition should include:
//!
//! | Element              | Description                                      |
//! | :------------------- | :----------------------------------------------- |
//! | **Summary line**     | `Name ([Protocol Link]): Brief description.`     |
//! | **Technical detail** | `Value: 'X' dec, 'YY' hex` or `Sequence: ESC X`. |
//! | **Context**          | Representation and protocol fit.                 |
//!
//! Here's an example:
//! ```no_run
//! /// Save Cursor (DECSC): Saves cursor position and attributes.
//! ///
//! /// Sequence: `ESC 7`
//! pub const DECSC_SAVE_CURSOR: u8 = b'7';
//! ```
//!
//! ## Tier 2 - Composed sequences
//!
//! [`define_ansi_const!`] macro invocations for compile-time strings that concatenate a
//! prefix with a value. The macro handles the [`concat!`] magic and auto-generates
//! documentation from two string literals in its DSL: `$doc_title` and `$doc_details`.
//! The following table lists all the parts of the documentation that are generated:
//!
//! | Element            | Description                                                  |
//! | :----------------- | :----------------------------------------------------------- |
//! | **Summary line**   | `$doc_title: $doc_details` (auto-joined by the macro).       |
//! | **Full sequence**  | Auto-generated, e.g., `Full sequence: CSI ?1049h`.           |
//! | **Protocol links** | Auto-added for [`CSI`], [`ESC`], [`SGR`], etc.               |
//!
//! Here's an example of macro invocation:
//!
//! ```no_run
//! # use r3bl_tui::define_ansi_const;
//! define_ansi_const!(@esc_str : ESC_SAVE_CURSOR_STR = ["7"] =>
//!     "Save Cursor (DECSC)" : "Saves cursor position and attributes."
//! );
//! ```
//!
//! It expands to:
//!
//! ```no_run
//! /// Save Cursor (DECSC): Saves cursor position and attributes.
//! ///
//! /// Full sequence: `ESC 7`
//! pub const ESC_SAVE_CURSOR_STR: &str = "\x1b7";
//! ```
//!
//! ## Tier 3 - Dynamic sequences
//!
//! Complex compositions requiring runtime values are handled by enums like
//! [`EscSequence`], [`CsiSequence`], and [`SgrCode`], or by specialized generator
//! functions like [`generate_keyboard_sequence`] and [`generate_mouse_sequence_bytes`].
//!
//! # Organization
//!
//! Constants are grouped by protocol domain:
//!
//! | Module                  | Domain                                                          |
//! | :---------------------- | :-------------------------------------------------------------- |
//! | **[`generic`]**         | Terminal modes, [`DEC`] modes, mouse tracking, alternate screen |
//! | **[`csi`]**             | [`CSI`] sequences, cursor movement, [`SGR`] parameters, colors  |
//! | **[`esc`]**             | [`ESC`] sequences, character set selection, C0 control chars    |
//! | **[`dsr`]**             | Device Status Report response constants                         |
//! | **[`input_sequences`]** | Keyboard input, control characters                              |
//! | **[`mouse`]**           | Mouse protocol constants ([`SGR`], [`X10`], [`RXVT`])           |
//! | **[`raw_mode`]**        | Raw mode terminal configuration (VMIN, VTIME)                   |
//! | **[`sgr`]**             | [`SGR`] byte constants for performance-critical paths           |
//! | **[`utf8`]**            | [`UTF-8`] encoding constants for byte-level text parsing        |
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`CSI`]: crate::CsiSequence
//! [`CsiSequence`]: crate::CsiSequence
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
//! [`define_ansi_const!`]: crate::define_ansi_const
//! [`ESC`]: crate::EscSequence
//! [`EscSequence`]: crate::EscSequence
//! [`generate_keyboard_sequence`]: crate::generate_keyboard_sequence
//! [`generate_mouse_sequence_bytes`]: crate::generate_mouse_sequence_bytes
//! [`RXVT`]: https://en.wikipedia.org/wiki/Rxvt
//! [`SGR`]: crate::SgrCode
//! [`SgrCode`]: crate::SgrCode
//! [`UTF-8`]: https://en.wikipedia.org/wiki/UTF-8
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`X10`]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking

#![rustfmt::skip]

// Module is public only when building documentation or tests.
// This allows rustdoc links to work while keeping it private in release builds.
#[cfg(any(test, doc))]
pub mod csi;
#[cfg(not(any(test, doc)))]
mod csi;
#[cfg(any(test, doc))]
pub mod dsr;
#[cfg(not(any(test, doc)))]
mod dsr;
#[cfg(any(test, doc))]
pub mod esc;
#[cfg(not(any(test, doc)))]
mod esc;
#[cfg(any(test, doc))]
pub mod generic;
#[cfg(not(any(test, doc)))]
mod generic;
#[cfg(any(test, doc))]
pub mod input_sequences;
#[cfg(not(any(test, doc)))]
mod input_sequences;
#[cfg(any(test, doc))]
pub mod mouse;
#[cfg(not(any(test, doc)))]
mod mouse;
#[cfg(any(test, doc))]
pub mod raw_mode;
#[cfg(not(any(test, doc)))]
mod raw_mode;
#[cfg(any(test, doc))]
pub mod sgr;
#[cfg(not(any(test, doc)))]
mod sgr;
#[cfg(any(test, doc))]
pub mod utf8;
#[cfg(not(any(test, doc)))]
mod utf8;

// Macros for const expansion.
mod macros;

// Public re-exports (flat API) for convenience.
pub use csi::*;
pub use dsr::*;
pub use esc::*;
pub use generic::*;
pub use input_sequences::*;
pub use raw_mode::*;
pub use sgr::*;
pub use utf8::*;
#[allow(unused_imports)]
pub use mouse::*;
