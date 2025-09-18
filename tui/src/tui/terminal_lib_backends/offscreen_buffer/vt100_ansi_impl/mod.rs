// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT100/ANSI terminal operation implementations for `OffscreenBuffer`.
//!
//! This module contains the actual implementations of VT100 and ANSI escape sequence
//! operations that are delegated from the `vt100_ansi_parser::operations` module. The
//! structure mirrors `vt100_ansi_parser/operations/` to provide a clear 1:1 mapping
//! between the parser shim layer and the implementation layer.
//!
//! # Architecture
//!
//! ```text
//! vt100_ansi_parser/operations/char_ops.rs     (shim - minimal logic)
//!           ↓ delegates to
//! vt100_ansi_impl/char_ops.rs           (implementation - full logic)
//! ```
//!
//! # Module Organization
//!
//! Each file corresponds directly to a file in `vt100_ansi_parser/operations/`:
//!
//! - [`char_ops`] - Character operations (`print_char`, ICH, DCH, ECH)
//! - [`control_ops`] - Control character handling (BS, TAB, LF, CR)
//! - [`cursor_ops`] - Cursor movement operations
//! - [`line_ops`] - Line manipulation operations
//! - [`scroll_ops`] - Scrolling operations
//! - [`terminal_ops`] - Terminal state operations (reset, clear, charset)
//! - [`bounds_check`] - Bounds checking utilities
//!
//! # VT100 Compliance
//!
//! These implementations follow VT100 terminal specifications and are tested for
//! compliance in the `vt_100_ansi_conformance_tests` module.

/// Standard terminal tab stop width (8 columns).
/// Used for calculating tab positions in VT100 terminal emulation.
/// This is a widely-adopted standard across most terminal emulators.
pub const TAB_STOP_WIDTH: usize = 8;

// Attach modules.
pub mod bounds_check;
pub mod char_ops;
pub mod control_ops;
pub mod cursor_ops;
pub mod line_ops;
pub mod scroll_ops;
pub mod terminal_ops;

// Note: Individual modules are typically accessed directly by their respective
// vt100_ansi_parser operation files. No re-exports needed here.
