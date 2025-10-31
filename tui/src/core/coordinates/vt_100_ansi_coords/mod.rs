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
//! - [`TermRow`]: 1-based row coordinate for ANSI sequences
//! - [`TermCol`]: 1-based column coordinate for ANSI sequences
//! - [`TermPos`]: 1-based position combining column and row (used in mouse events)
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
mod term_pos;
mod term_row;

// Re-export for flat public API.
pub use term_col::*;
pub use term_pos::*;
pub use term_row::*;
