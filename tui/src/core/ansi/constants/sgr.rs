// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! SGR (Select Graphic Rendition) sequence constants.

/// SGR Reset sequence bytes.
///
/// Resets all text attributes (color, bold, italic, etc.) to default.
/// This constant provides zero-overhead access for performance-critical paths.
pub const SGR_RESET_BYTES: &[u8] = b"\x1b[0m";

/// CRLF (Carriage Return + Line Feed) sequence for terminal line endings.
/// Used to move cursor to beginning of next line in terminal output.
pub const CRLF_BYTES: &[u8] = b"\r\n";
