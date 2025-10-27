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
//! - **[`vt_100_ansi_conformance_tests`]**: Comprehensive VT100 conformance tests
//!
//! ## Key Types
//!
//! - [`CsiSequence`] - Cursor manipulation and styling sequences
//! - [`crate::EscSequence`] - Simple escape sequences (in generator module)
//! - [`AnsiToOfsBufPerformer`] - Main parser state machine

pub mod ansi_parser_public_api;
pub mod operations;
pub mod performer;
pub mod protocols;

// VT100 conformance tests module
pub mod vt_100_ansi_conformance_tests;

// Re-export public API
pub use ansi_parser_public_api::*;
pub use protocols::*;
