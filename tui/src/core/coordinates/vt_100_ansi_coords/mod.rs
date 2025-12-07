// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! 1-based VT-100 ANSI escape sequence coordinates.
//!
//! This module provides coordinate types specifically for VT-100 ANSI escape sequence
//! parsing. These types use 1-based indexing and wrap [`std::num::NonZeroU16`] as
//! mandated by the VT-100 specification.
//!
//! # Usage
//!
//! Use these types **only** when:
//! - Parsing ANSI escape sequences (e.g., `ESC[5;10H`)
//! - Working with `vt_100_pty_output_parser` module
//!
//! For all other terminal operations (including crossterm), use [`buffer_coords`] types
//! which are 0-based.
//!
//! # Core Types
//!
//! ## Absolute Positioning (1-based)
//!
//! - [`TermRow`]: 1-based row coordinate for ANSI sequences
//! - [`TermCol`]: 1-based column coordinate for ANSI sequences
//! - [`TermPos`]: 1-based position combining column and row (used in mouse events)
//!
//! ## Relative Movement (0-based delta)
//!
//! - [`TermRowDelta`]: How many rows to move (for `CursorUp`/`CursorDown`)
//! - [`TermColDelta`]: How many columns to move (for `CursorForward`/`CursorBackward`)
//!
//! # The CSI Zero Problem - Make Illegal States Unrepresentable
//!
//! ANSI cursor movement commands interpret parameter 0 as 1:
//! - `CSI 0 A` moves the cursor **1 row up**, not 0
//! - `CSI 0 C` moves the cursor **1 column right**, not 0
//!
//! The delta types wrap [`NonZeroU16`] internally, making zero-valued deltas
//! **impossible to represent**. Construction is fallible:
//!
//! ```rust
//! use r3bl_tui::{TermRowDelta, CsiSequence};
//!
//! // Fallible construction - must handle the None case
//! if let Some(delta) = TermRowDelta::new(0) {
//!     // This branch is NOT taken for zero, preventing the bug
//!     let _ = CsiSequence::CursorDown(delta);
//! }
//! ```
//!
//! [`NonZeroU16`]: std::num::NonZeroU16
//!
//! # Coordinate Conversion
//!
//! Always use explicit conversion methods:
//! - `.to_zero_based()`: Convert to 0-based buffer coordinates
//! - `.from_zero_based()`: Convert from 0-based buffer coordinates
//!
//! [`buffer_coords`]: crate::coordinates::buffer_coords

// Submodule declarations (private).
mod term_col;
mod term_col_delta;
mod term_pos;
mod term_row;
mod term_row_delta;

// Re-export for flat public API.
pub use term_col::*;
pub use term_col_delta::*;
pub use term_pos::*;
pub use term_row::*;
pub use term_row_delta::*;
