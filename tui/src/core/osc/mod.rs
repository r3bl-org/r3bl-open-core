// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC (Operating System Command) sequence parsing for terminal progress indicators.
//!
//! Parses OSC 9;4 sequences used by Cargo and other build tools to communicate
//! progress information. Supports four progress states: progress updates (0-100%),
//! progress cleared, build errors, and indeterminate progress.
//!
//! The [`OscBuffer`] handles partial sequences split across buffer reads and
//! gracefully ignores malformed input.

pub mod buffer;
pub mod codes;
pub mod event;
pub mod hyperlink;

// Re-export main types and functions for convenience
pub use buffer::OscBuffer;
pub use event::OscEvent;