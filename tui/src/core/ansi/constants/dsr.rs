// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) response sequence constants.

// DSR response sequence components.

/// CSI sequence start for DSR responses: ESC [
pub const DSR_RESPONSE_START: &str = "\x1b[";

/// Status OK code: 0
pub const DSR_STATUS_OK_CODE: &str = "0";

/// Status response terminator: n
pub const DSR_STATUS_RESPONSE_END: char = 'n';

/// Cursor position response terminator: R
pub const DSR_CURSOR_POSITION_RESPONSE_END: char = 'R';

// Complete response sequences for testing.

/// Complete status OK response: ESC[0n
pub const DSR_STATUS_OK_FULL_RESPONSE: &str = "\x1b[0n";
