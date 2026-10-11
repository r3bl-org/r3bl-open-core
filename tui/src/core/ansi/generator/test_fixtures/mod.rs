// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test fixtures for [`ANSI`] sequence generation.
//!
//! Provides synthetic input sequence generators for testing terminal input parsing and
//! [`PTY`] round-trips.
//!
//! [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
//! [`PTY`]: crate::core::pty

pub mod ansi_input;
pub use ansi_input::*;
