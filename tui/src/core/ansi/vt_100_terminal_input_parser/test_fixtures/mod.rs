// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Shared test infrastructure for ANSI sequence generation.
//!
//! This module provides generator functions used by both unit tests and integration
//! tests. These are **test utilities only** - not tests themselves.
//!
//! # Purpose
//!
//! The input event generator enables:
//! 1. **Round-trip validation**: Parse ANSI → InputEvent → Generate ANSI → Verify
//! 2. **System integration testing**: Generate sequences for real PTY testing
//! 3. **Test helpers**: Build test sequences without hardcoding raw bytes
//!
//! # Module Organization
//!
//! - **Generator functions**: Convert `InputEvent` → ANSI bytes
//!   - Keyboard sequences (arrows, function keys, modifiers)
//!   - Terminal events (resize, focus, paste)
//!   - Mouse events (SGR format)
//! - **Round-trip tests**: Validate generator ↔ parser compatibility
//!   (see [`crate::core::ansi::vt_100_terminal_input_parser::unit_tests`])
//!
//! See the [parent module](super#testing-strategy) for the overall testing strategy.

#[cfg(any(test, doc))]
pub mod input_sequence_generator;

#[cfg(any(test, doc))]
pub use input_sequence_generator::*;
