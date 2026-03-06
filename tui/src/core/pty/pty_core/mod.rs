// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core [`PTY`] (Pseudo-Terminal) types and functionality.
//!
//! This module provides the fundamental building blocks for [`PTY`] operations including:
//! - Type aliases for [`PTY`] components and channels
//! - Event types for bidirectional communication
//! - Control character handling and conversion
//! - Session handles for read-only and read-write operations
//! - Utility functions for cross-platform compatibility
//!
//! The module is organized into focused submodules for maintainability:
//! - [`pty_pair`] - [`PtyPair`] struct (controlled side lifecycle, [`fd`] ownership, and
//!   [`PTY`] primer)
//! - [`pty_types`] - Core type aliases and constants
//! - [`pty_input_events`] - Input event definitions and `KeyPress` conversion
//! - [`pty_output_events`] - Output event definitions, terminal control sequences, and
//!   cursor mode detection
//! - [`pty_sessions`] - Session handle types
//!
//! [`fd`]: https://en.wikipedia.org/wiki/File_descriptor
//! [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal

pub mod pty_input_events;
pub mod pty_output_events;
pub mod pty_pair;
pub mod pty_sessions;
pub mod pty_size;
pub mod pty_types;

// Re-export all public types and functions for convenience.
pub use pty_input_events::*;
pub use pty_output_events::*;
pub use pty_pair::*;
pub use pty_sessions::*;
pub use pty_size::*;
pub use pty_types::*;
