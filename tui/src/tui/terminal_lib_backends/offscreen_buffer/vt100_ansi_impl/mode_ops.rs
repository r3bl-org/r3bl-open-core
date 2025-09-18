// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations for VT100/ANSI terminal emulation.
//!
//! This module implements mode operations that correspond to ANSI mode
//! sequences handled by the `vt_100_ansi_parser::operations::mode_ops` module. These
//! include:
//!
//! - **SM h** (Set Mode) - `set_auto_wrap_mode`
//! - **RM l** (Reset Mode) - `reset_auto_wrap_mode`
//!
//! All operations maintain VT100 compliance and handle proper mode state
//! management for terminal operations.

#[allow(clippy::wildcard_imports)]
use super::super::*;

impl OffscreenBuffer {
    /// Set auto wrap mode on.
    /// When enabled, text automatically wraps to the next line when it
    /// reaches the right margin.
    pub fn set_auto_wrap_mode(&mut self, enabled: bool) {
        self.ansi_parser_support.auto_wrap_mode = enabled;
    }
}

#[cfg(test)]
mod tests_mode_ops {
    use super::*;
    use crate::{height, width};

    fn create_test_buffer() -> OffscreenBuffer {
        let size = width(10) + height(6);
        OffscreenBuffer::new_empty(size)
    }

    #[test]
    fn test_set_auto_wrap_mode_enabled() {
        let mut buffer = create_test_buffer();

        // Initially should be enabled by default
        assert!(buffer.ansi_parser_support.auto_wrap_mode);

        buffer.set_auto_wrap_mode(true);
        assert!(buffer.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_set_auto_wrap_mode_disabled() {
        let mut buffer = create_test_buffer();

        buffer.set_auto_wrap_mode(false);
        assert!(!buffer.ansi_parser_support.auto_wrap_mode);
    }

    #[test]
    fn test_toggle_auto_wrap_mode() {
        let mut buffer = create_test_buffer();

        // Start enabled
        buffer.set_auto_wrap_mode(true);
        assert!(buffer.ansi_parser_support.auto_wrap_mode);

        // Disable
        buffer.set_auto_wrap_mode(false);
        assert!(!buffer.ansi_parser_support.auto_wrap_mode);

        // Enable again
        buffer.set_auto_wrap_mode(true);
        assert!(buffer.ansi_parser_support.auto_wrap_mode);
    }
}
