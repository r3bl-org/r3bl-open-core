// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for ANSI parser implementation.

use crate::{OffscreenBuffer, height, width};

/// Create a test `OffscreenBuffer` with 10x10 dimensions.
pub fn create_test_offscreen_buffer_10r_by_10c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(10) + width(10))
}

// Test modules.
#[rustfmt::skip] // Reorder the following for better readability

pub mod tests_processor_lifecycle;
pub mod tests_character_encoding;
pub mod tests_control_sequences;
pub mod tests_cursor_operations;
pub mod tests_display_operations;
pub mod tests_integration;
pub mod tests_line_and_buffer_control;
pub mod tests_osc_sequences;
