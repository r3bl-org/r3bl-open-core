// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Unix input handling for [`DirectToAnsi`] backend.
//!
//! This module is **Unix-only** (gated by `#[cfg(unix)]`) because it uses:
//! - `SIGWINCH` signals for terminal resize detection
//! - Unix-specific stdin semantics
//!
//! See `TODO(windows)` comments throughout for what would need to change for
//! Windows support.
//!
//! # Entry Point
//!
//! [`DirectToAnsiInputDevice::try_read_event`] is the main async method for reading
//! terminal input with zero-latency `ESC` key detection.
//!
//! [`DirectToAnsi`]: mod@super

// Private submodules - organized by functional concern.
mod input_device;
mod parse_buffer;
mod paste_state_machine;
mod global_input_resource;
mod types;

// Conditionally public for documentation (to allow rustdoc links).
#[cfg(any(test, doc))]
pub mod protocol_conversion;
#[cfg(not(any(test, doc)))]
mod protocol_conversion;

// Re-exports - flatten the public API.
pub use input_device::*;
