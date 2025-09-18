// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence operation modules.
//!
//! This module organizes all the different types of ANSI operations into
//! logical groups for better maintainability and discoverability.
//!
//! # Design Architecture
//!
//! The operation modules in this directory follow a consistent **thin shim pattern**:
//!
//! - Each operation module acts as a parameter translator from ANSI sequences to
//!   [`OffscreenBuffer`] calls
//! - Operations contain minimal logic, primarily focused on parameter parsing and
//!   delegation
//! - All actual terminal buffer logic is implemented in dedicated [`OffscreenBuffer`]
//!   methods
//! - This design ensures a clear separation between ANSI protocol handling and buffer
//!   operations
//!
//! This consistent pattern across all operation modules makes the codebase predictable
//! and maintainable, with clear boundaries between protocol translation and buffer
//! management.
//!
//! # Testing Strategy
//!
//! The operation modules in this directory intentionally do not have direct unit tests,
//! which diverges from the codebase norm. This is because:
//!
//! 1. These operations are thin delegation layers with minimal logic
//! 2. They primarily translate ANSI parameters into [`OffscreenBuffer`] method calls
//! 3. The underlying [`OffscreenBuffer`] methods have comprehensive unit tests
//! 4. The VT100 conformance tests in [`vt_100_ansi_conformance_tests`] verify the
//!    complete ANSI processing pipeline
//!
//! This approach avoids redundant testing while ensuring both unit-level correctness
//! (in [`OffscreenBuffer`]) and system-level behavior (in conformance tests).
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`vt_100_ansi_conformance_tests`]: mod@super::vt_100_ansi_conformance_tests

pub mod char_ops;
pub mod control_ops;
pub mod cursor_ops;
pub mod dsr_ops;
pub mod line_ops;
pub mod margin_ops;
pub mod mode_ops;
pub mod osc_ops;
pub mod scroll_ops;
pub mod sgr_ops;
pub mod terminal_ops;

// Re-export all operations for easier access.
pub use char_ops::*;
pub use control_ops::*;
pub use cursor_ops::*;
pub use dsr_ops::*;
pub use line_ops::*;
pub use margin_ops::*;
pub use mode_ops::*;
pub use osc_ops::*;
pub use scroll_ops::*;
pub use sgr_ops::*;
pub use terminal_ops::*;
