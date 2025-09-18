// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Line manipulation operations for VT100/ANSI terminal emulation.
//!
//! This module implements line-level operations that correspond to ANSI line
//! sequences handled by the `vt100_ansi_parser::operations::line_ops` module. These
//! include:
//!
//! - **IL** (Insert Lines) - `shift_lines_down`
//! - **DL** (Delete Lines) - `shift_lines_up`
//! - **EL** (Erase Line) - `clear_line`
//!
//! All operations maintain VT100 compliance and handle proper line manipulation
//! within scroll regions as specified in VT100 documentation.

#[allow(clippy::wildcard_imports)]
use super::super::*;

impl OffscreenBuffer {
    // TODO: Move line manipulation operations from existing files here
    // These methods currently exist in ofs_buf_line_level_ops.rs and should be moved here
    // to provide a clean mapping from vt100_ansi_parser::operations::line_ops
}

#[cfg(test)]
mod tests_line_ops {
    // TODO: Add comprehensive tests for line manipulation operations.
}
