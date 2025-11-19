// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for terminal emulation
//!
//! This module provides a comprehensive VT100-compliant ANSI escape sequence parser
//! that processes terminal output and converts it into structured operations.
//!
//! ## Architecture
//!
//! - **[`performer`]**: VTE `Perform` trait implementation - handles state transitions
//! - **[`protocols`]**: ANSI sequence types and constants
//! - **[`operations`]**: Protocol handlers that translate sequences into operations
//! - **[`vt_100_pty_output_conformance_tests`]**: Comprehensive VT100 conformance tests
//!
//! ## Key Types
//!
//! - [`CsiSequence`] - Cursor manipulation and styling sequences
//! - [`crate::EscSequence`] - Simple escape sequences (in generator module)
//! - [`AnsiToOfsBufPerformer`] - Main parser state machine
//!
//! ## Primary Consumer
//!
//! This parser is primarily used by [`OffscreenBuffer::apply_ansi_bytes`], which
//! processes PTY output from child processes and updates the terminal display state.
//!
//! ```text
//! pty_mux (process_manager.rs)
//!    │
//!    │ Receives bytes from child process (bash, vim, etc.)
//!    ▼
//! OffscreenBuffer::apply_ansi_bytes()
//!    │
//!    │ Delegates to this parser
//!    ▼
//! AnsiToOfsBufPerformer (vte::Perform implementation)
//!    │
//!    │ Updates buffer state
//!    ▼
//! OffscreenBuffer (cursor, text, styles)
//! ```
//!
//! For terminal multiplexer architecture, see the [`pty_mux`] module.
//!
//! [`OffscreenBuffer::apply_ansi_bytes`]: crate::OffscreenBuffer::apply_ansi_bytes
//! [`pty_mux`]: mod@crate::core::pty_mux

pub mod ansi_parser_public_api;
pub mod operations;
pub mod performer;
pub mod protocols;

// VT100 conformance tests module
pub mod vt_100_pty_output_conformance_tests;

// Re-export public API
pub use ansi_parser_public_api::*;
pub use operations::*;
pub use protocols::*;
