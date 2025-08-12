// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core components of the zero-copy gap buffer implementation.
//!
//! This module contains the fundamental data structures used by the gap buffer:
//! - [`ZeroCopyGapBuffer`] - The main buffer structure
//! - [`GapBufferLine`] - A line view combining content and metadata
//! - [`LineMetadata`] - Metadata about each line including segments and display info

// Attach.
pub mod gap_buffer_line;
pub mod line_metadata;
pub mod zero_copy_gap_buffer;

// Re-export.
pub use gap_buffer_line::*;
pub use line_metadata::*;
pub use zero_copy_gap_buffer::*;

/// Initial size of each line in bytes
pub const INITIAL_LINE_SIZE: usize = 256;

/// Page size for extending lines (bytes added when line overflows)
pub const LINE_PAGE_SIZE: usize = 256;
