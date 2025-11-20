// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Input handling for `DirectToAnsi` backend.
//!
//! See [`DirectToAnsiInputDevice`] for the async input device implementation with
//! zero-latency ESC key detection.

// Private inner modules.
mod input_device_impl;

// Conditionally public for documentation (to allow rustdoc links).
#[cfg(any(test, doc))]
pub mod protocol_conversion;
#[cfg(not(any(test, doc)))]
mod protocol_conversion;

// Re-exports - flatten the public API.
pub use input_device_impl::*;
