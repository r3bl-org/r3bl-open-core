// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input handling for `DirectToAnsi` backend.
//!
//! See [`DirectToAnsiInputDevice`] for the async input device implementation with
//! zero-latency ESC key detection.

// Private submodules - organized by functional concern.
mod buffer;
mod input_device;
mod input_event_handlers;
mod paste_state_machine;
mod singleton;
mod stdin_reader_thread;
mod types;

// Conditionally public for documentation (to allow rustdoc links).
#[cfg(any(test, doc))]
pub mod protocol_conversion;
#[cfg(not(any(test, doc)))]
mod protocol_conversion;

// Re-exports - flatten the public API.
pub use input_device::*;
