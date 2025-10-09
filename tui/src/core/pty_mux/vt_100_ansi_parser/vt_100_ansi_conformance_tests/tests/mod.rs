// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Test modules for VT100 ANSI conformance validation.
//!
//! This module organizes conformance tests by functionality and architectural layer:
//! - Operations tests (test_*_ops.rs) - Test operations modules directly
//! - Protocol tests (`test_protocol_*.rs`) - Test ANSI/VT100 protocol parsing
//! - System tests (`test_system_*.rs`) - Test system components and lifecycle
//! - Integration tests (`test_integration_*.rs`) - Test cross-cutting scenarios
//!
//! # Testing Architecture Relationships
//!
//! Each test file in this module corresponds to files in the other two layers of the
//! architecture, creating a clear 1:1:1 mapping for navigation:
//!
//! ```text
//! vt_100_test_char_ops ←→ operations/vt_100_shim_char_ops ←→ vt_100_ansi_impl/vt_100_impl_char_ops
//! vt_100_test_cursor_ops ←→ operations/vt_100_shim_cursor_ops ←→ vt_100_ansi_impl/vt_100_impl_cursor_ops
//! vt_100_test_sgr_ops ←→ operations/vt_100_shim_sgr_ops ←→ vt_100_ansi_impl/vt_100_impl_sgr_ops
//! ...and so on
//! ```
//!
//! ## Navigation Between Layers
//!
//! When working on any test file, you can easily navigate to its corresponding:
//! - **Shim Layer**: [`operations`] - The delegation layer being tested indirectly
//! - **Implementation Layer**: [`vt_100_ansi_impl`] - The business logic being tested
//! - **Parent Documentation**: See [conformance tests] for the integration testing
//!   philosophy
//!
//! For example, when working on character operations:
//! 1. **Integration**: [`vt_100_test_char_ops`] - Tests using [`apply_ansi_bytes`] public
//!    API
//! 2. **Shim**: [`operations::char_ops`] - Parameter translation (no direct tests)
//! 3. **Implementation**: [`impl_char_ops`] - Buffer logic (has unit tests)
//!
//! [`operations`]: super::super::operations
//! [`vt_100_ansi_impl`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl
//! [`operations::char_ops`]: super::super::operations::vt_100_shim_char_ops
//! [`impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_char_ops
//! [`apply_ansi_bytes`]: crate::tui::terminal_lib_backends::offscreen_buffer::OffscreenBuffer::apply_ansi_bytes
//! [conformance tests]: super

// === OPERATIONS TESTS ===
// These test files map 1:1 with operations/ modules.

#[cfg(any(test, doc))]
pub mod vt_100_test_char_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_control_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_cursor_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_dsr_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_line_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_margin_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_mode_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_osc_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_scroll_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_sgr_ops;

#[cfg(any(test, doc))]
pub mod vt_100_test_terminal_ops;

// === PROTOCOL TESTS ===
// These test ANSI/VT100 protocol parsing and sequence handling.

#[cfg(test)]
mod vt_100_test_protocol_csi_basic;

#[cfg(test)]
mod vt_100_test_protocol_char_encoding;

#[cfg(test)]
mod vt_100_test_protocol_control_chars;

// === SYSTEM TESTS ===
// These test system components and lifecycle management.

#[cfg(test)]
mod vt_100_test_system_error_handling;

#[cfg(test)]
mod vt_100_test_system_performer_lifecycle;

#[cfg(test)]
mod vt_100_test_system_state_management;

// === INTEGRATION TESTS ===
// These test cross-cutting scenarios and real-world use cases

#[cfg(test)]
mod vt_100_test_integration_basic;

#[cfg(test)]
mod vt_100_test_integration_real_world;
