// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Raw mode terminal configuration constants.
//!
//! This module contains constants specific to raw mode terminal configuration,
//! particularly for POSIX termios special codes (VMIN and VTIME).
//!
//! # Raw Mode Configuration
//!
//! Raw mode disables terminal line buffering and processing, allowing applications
//! to read input character-by-character. The behavior is controlled by two special
//! codes in the termios structure:
//!
//! - **VMIN**: Minimum number of bytes to read before returning
//! - **VTIME**: Timeout in deciseconds (0.1s units)
//!
//! ## Standard Raw Mode Settings
//!
//! For typical raw mode (immediate, byte-by-byte input with no timeout):
//! - VMIN = 1: Return after reading at least 1 byte
//! - VTIME = 0: No timeout, blocking read
//!
//! This configuration is used by:
//! - `cfmakeraw()` in POSIX
//! - crossterm's raw mode implementation
//! - Most TUI applications requiring immediate input
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use rustix::termios::{self, SpecialCodeIndex};
//! use std::io::stdin;
//! use r3bl_tui::{VMIN_RAW_MODE, VTIME_RAW_MODE};
//!
//! // Get current terminal settings
//! let mut termios = termios::tcgetattr(&stdin()).unwrap();
//!
//! // Configure for raw mode: byte-by-byte, no timeout
//! termios.special_codes[SpecialCodeIndex::VMIN] = VMIN_RAW_MODE;
//! termios.special_codes[SpecialCodeIndex::VTIME] = VTIME_RAW_MODE;
//!
//! // Apply settings
//! termios::tcsetattr(&stdin(), termios::OptionalActions::Now, &termios).unwrap();
//! ```
//!
//! ## VMIN/VTIME Interaction Matrix
//!
//! | VMIN | VTIME | Behavior                                          |
//! |------|-------|---------------------------------------------------|
//! | 0    | 0     | Non-blocking: return immediately with available   |
//! | 0    | >0    | Timed read: return after timeout or data          |
//! | >0   | 0     | Blocking: return after VMIN bytes (no timeout)    |
//! | >0   | >0    | Interbyte timeout: return after VMIN or timeout   |
//!
//! Raw mode uses **VMIN=1, VTIME=0** for immediate, blocking input.

// ==================== Special Codes for Raw Mode ====================

/// VMIN value for raw mode: return after reading 1 byte.
///
/// In raw mode, `VMIN=1` means `read()` will block until at least one byte
/// is available, then return immediately with that byte. This enables
/// character-by-character input processing without line buffering.
///
/// This is the standard setting for:
/// - POSIX `cfmakeraw()`
/// - crossterm raw mode
/// - Interactive TUI applications
pub const VMIN_RAW_MODE: u8 = 1;

/// VTIME value for raw mode: no timeout (blocking read).
///
/// In raw mode, `VTIME=0` means `read()` will block indefinitely until
/// `VMIN` bytes are available (when VMIN > 0). Combined with `VMIN=1`,
/// this creates a simple blocking, byte-by-byte read behavior.
///
/// The unit for VTIME is deciseconds (0.1 second increments), so:
/// - 0 = no timeout (block forever)
/// - 1 = 0.1 second timeout
/// - 10 = 1 second timeout
pub const VTIME_RAW_MODE: u8 = 0;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_mode_special_codes() {
        // Verify standard raw mode configuration
        assert_eq!(VMIN_RAW_MODE, 1, "VMIN should be 1 for byte-by-byte reading");
        assert_eq!(
            VTIME_RAW_MODE, 0,
            "VTIME should be 0 for no timeout (blocking)"
        );
    }

    #[test]
    fn test_vmin_enables_immediate_return() {
        // With VMIN=1, read() returns after receiving at least 1 byte
        // This is the key to character-by-character input
        assert_eq!(VMIN_RAW_MODE, 1);
    }

    #[test]
    fn test_vtime_disables_timeout() {
        // With VTIME=0, read() blocks indefinitely (no timeout)
        // Combined with VMIN=1, this creates blocking character reads
        assert_eq!(VTIME_RAW_MODE, 0);
    }
}
