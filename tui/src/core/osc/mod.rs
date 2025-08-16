// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC (Operating System Command) sequence parsing and formatting.
//!
//! This module provides support for:
//! - OSC 9;4 sequences used by Cargo and other build tools to communicate progress
//!   information. Supports four progress states: progress updates (0-100%), progress
//!   cleared, build errors, and indeterminate progress.
//! - OSC 8 sequences for creating terminal hyperlinks that can be clicked to open URLs or
//!   file paths.
//!
//! The [`OscBuffer`] handles partial sequences split across buffer reads and
//! gracefully ignores malformed input.

pub mod osc_buffer;
pub mod osc_codes;
pub mod osc_event;
pub mod osc_hyperlink;

// Re-export main types and functions for convenience
pub use osc_buffer::*;
pub use osc_event::*;
pub use osc_hyperlink::*;
