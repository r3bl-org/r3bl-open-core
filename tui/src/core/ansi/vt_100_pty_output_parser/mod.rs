// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`ANSI`]/[`VT-100`] sequence parsing for terminal emulation
//!
//! This module provides a comprehensive [`VT-100`]-compliant [`ANSI`] escape sequence
//! parser that processes terminal output and converts it into structured ops.
//!
//! ## Architecture
//!
//! - **[`performer`]**: [`VTE`] [`Perform`] trait implementation - handles state
//!   transitions
//! - **[`protocols`]**: [`ANSI`] sequence types and constants
//! - **[`ops`]**: Protocol handlers that translate sequences into ops
//! - **[`vt_100_pty_output_conformance_tests`]**: Comprehensive [`VT-100`] conformance
//!   tests
//!
//! ## Key Types
//!
//! - [`CsiSequence`] - Cursor manipulation and styling sequences
//! - [`crate::EscSequence`] - Simple escape sequences (in generator module)
//! - [`AnsiToOfsBufPerformer`] - Main parser state machine
//!
//! ## Primary Consumer
//!
//! This parser is primarily used by [`OfsBufVT100::apply_ansi_bytes`], which
//! processes [`PTY`] output from child processes and updates the terminal display state.
//!
//! ```text
//! pty_mux (process_manager.rs)
//!    │
//!    │ Receives bytes from child process (bash, vim, etc.)
//!    ▼
//! OfsBufVT100::apply_ansi_bytes()
//!    │
//!    │ Delegates to this parser
//!    ▼
//! AnsiToOfsBufPerformer (vte::Perform implementation)
//!    │
//!    │ Updates buffer state
//!    ▼
//! OfsBuf (cursor, text, styles)
//! ```
//!
//! For terminal multiplexer architecture, see the [`pty_mux`] module.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`OfsBufVT100::apply_ansi_bytes`]: crate::OfsBufVT100::apply_ansi_bytes
//! [`Perform`]: vte::Perform
//! [`pty_mux`]: mod@crate::core::pty_mux
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
//! [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
//! [`VTE`]: mod@vte

#![rustfmt::skip]

// Attach.
pub mod ansi_parser_public_api;
pub mod canvas;
pub mod hidden_screen_state;
pub mod ofs_buf_vt_100;
pub mod parser_state;
pub mod performer;
pub mod protocols;
pub mod pty_response_event;
mod modes;

#[cfg(any(test, doc))]
pub mod ops;
#[cfg(not(any(test, doc)))]
mod ops;

#[cfg(any(test, doc))]
pub mod ops_impl_ofs_buf;
#[cfg(not(any(test, doc)))]
mod ops_impl_ofs_buf;

// `VT-100` conformance tests module
pub mod vt_100_pty_output_conformance_tests;

// Re-export public API.
pub use ansi_parser_public_api::*;
pub use canvas::*;
pub use hidden_screen_state::*;
pub use modes::*;
pub use ofs_buf_vt_100::*;
pub use ops::*;
pub use ops_impl_ofs_buf::*;
pub use parser_state::*;
pub use protocols::*;
pub use pty_response_event::*;
