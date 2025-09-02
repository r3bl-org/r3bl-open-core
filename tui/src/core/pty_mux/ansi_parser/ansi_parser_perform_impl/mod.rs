// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Module for ANSI/VT sequence processing implementation.
//!
//! This module contains the implementation of the `Perform` trait and all
//! related operation modules for handling ANSI escape sequences.

pub mod perform_impl;
pub mod cursor_ops;
pub mod scroll_ops;
pub mod sgr_ops;
pub mod terminal_ops;
pub mod char_translation;
pub mod param_utils;
pub mod mode_ops;
pub mod margin_ops;
pub mod device_ops;

// Re-export the main implementation (the Perform trait impl)
// Note: The actual implementation is in perform_impl module