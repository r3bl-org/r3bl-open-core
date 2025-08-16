// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Core PTY (Pseudo-Terminal) types and functionality.
//!
//! This module provides the fundamental building blocks for PTY operations including:
//! - Type aliases for PTY components and channels
//! - Event types for bidirectional communication
//! - Control character handling and conversion
//! - Session handles for read-only and read-write operations
//! - Utility functions for cross-platform compatibility
//!
//! The module is organized into focused submodules for maintainability:
//! - [`pty_types`] - Core type aliases and constants
//! - [`pty_events`] - Input/output event definitions
//! - [`pty_control`] - Terminal control characters and sequences
//! - [`pty_sessions`] - Session handle types
//! - [`pty_utils`] - Cross-platform utility functions

pub mod pty_control;
pub mod pty_events;
pub mod pty_sessions;
pub mod pty_types;
pub mod pty_utils;

// Re-export all public types and functions for convenience
pub use pty_control::*;
pub use pty_events::*;
pub use pty_sessions::*;
pub use pty_types::*;
pub use pty_utils::*;
