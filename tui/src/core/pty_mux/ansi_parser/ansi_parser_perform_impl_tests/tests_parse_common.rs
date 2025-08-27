// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared test utilities for ANSI parser tests.

use crate::{OffscreenBuffer, height, width};

/// Create a test `OffscreenBuffer` with 10x10 dimensions.
pub fn create_test_offscreen_buffer_10r_by_10c() -> OffscreenBuffer {
    OffscreenBuffer::new_empty(height(10) + width(10))
}
