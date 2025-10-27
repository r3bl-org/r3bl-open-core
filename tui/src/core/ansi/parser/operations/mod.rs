// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence operation modules.
//!
//! This module organizes all the different types of ANSI operations into
//! logical groups for better maintainability and discoverability.
//!
//! For the complete architecture overview, including the shim → impl → test design
//! pattern and testing philosophy, see the [module-level documentation](super).
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
//! The operation modules in this directory **intentionally do not have direct unit
//! tests**. This is a deliberate architectural decision that diverges from the codebase
//! norm for excellent reasons:
//!
//! ## Why No Direct Tests?
//!
//! 1. **Pure Delegation**: These operations are thin delegation layers with no business
//!    logic
//! 2. **Parameter Translation Only**: They primarily translate ANSI parameters into
//!    [`OffscreenBuffer`] method calls
//! 3. **Minimal Risk**: Code simplicity means minimal risk of bugs
//! 4. **Comprehensive Coverage**: Testing is handled by two complementary layers:
//!    - **Unit Tests**: The underlying [`OffscreenBuffer`] methods have comprehensive
//!      unit tests
//!    - **Integration Tests**: The VT100 conformance tests in
//!      [`vt_100_ansi_conformance_tests`] verify the complete ANSI processing pipeline
//!
//! ## Testing Philosophy
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │ Shim Layer (operations/*)                               │
//! │ • NO DIRECT TESTS (by design)                           │
//! │ • Pure delegation, no business logic                    │
//! └────────────────────┬────────────────────────────────────┘
//!                      │ delegates to
//!                      ▼
//! ┌─────────────────────────────────────────────────────────┐
//! │ Implementation Layer (vt_100_ansi_impl/impl_*)          │
//! │ • UNIT TESTS: #[test] functions                         │
//! │ • Contains all business logic                           │
//! └─────────────────────────────────────────────────────────┘
//!
//! ┌─────────────────────────────────────────────────────────┐
//! │ Conformance Tests (vt_100_ansi_conformance_tests/*)     │
//! │ • INTEGRATION TESTS: Full pipeline validation           │
//! │ • Tests: Shim → Implementation → Buffer                 │
//! └─────────────────────────────────────────────────────────┘
//! ```
//!
//! This approach avoids redundant testing while ensuring both unit-level correctness
//! and system-level behavior validation.
//!
//! ## Navigation to Related Testing Layers
//!
//! When working with any operation module, you can navigate to its related testing
//! layers:
//! - **Implementation with Unit Tests**: [`vt_100_ansi_impl`] module
//! - **Integration Tests**: [`vt_100_ansi_conformance_tests`] module
//! - **Complete Testing Philosophy**: See the [main module documentation](super) for the
//!   full three-layer testing strategy
//!
//! # Naming Convention
//!
//! Files in this directory intentionally have no prefix, serving as the "shim" layer
//! in our three-layer architecture. This creates a searchable hierarchy when combined
//! with `impl_` prefixed implementations and `test_` prefixed tests.
//!
//! When you search for any operation (e.g., "`char_ops`") in your IDE, you'll see:
//! - [`vt_100_shim_char_ops`] (this directory) - The shim layer for protocol translation
//! - [`vt_100_impl_char_ops`] (implementation) - The business logic layer
//! - [`vt_100_test_char_ops`] (tests) - The validation layer
//!
//! This same pattern applies to all operation types:
//!
//! | Shim                          | Ops                          | Tests                          |
//! |-------------------------------|------------------------------|--------------------------------|
//! | [`vt_100_shim_char_ops`]      | [`vt_100_impl_char_ops`]     | [`vt_100_test_char_ops`]       |
//! | [`vt_100_shim_control_ops`]   | [`vt_100_impl_control_ops`]  | [`vt_100_test_control_ops`]    |
//! | [`vt_100_shim_cursor_ops`]    | [`vt_100_impl_cursor_ops`]   | [`vt_100_test_cursor_ops`]     |
//! | [`vt_100_shim_dsr_ops`]       | [`vt_100_impl_dsr_ops`]      | [`vt_100_test_dsr_ops`]        |
//! | [`vt_100_shim_line_ops`]      | [`vt_100_impl_line_ops`]     | [`vt_100_test_line_ops`]       |
//! | [`vt_100_shim_margin_ops`]    | [`vt_100_impl_margin_ops`]   | [`vt_100_test_margin_ops`]     |
//! | [`vt_100_shim_mode_ops`]      | [`vt_100_impl_mode_ops`]     | [`vt_100_test_mode_ops`]       |
//! | [`vt_100_shim_osc_ops`]       | [`vt_100_impl_osc_ops`]      | [`vt_100_test_osc_ops`]        |
//! | [`vt_100_shim_scroll_ops`]    | [`vt_100_impl_scroll_ops`]   | [`vt_100_test_scroll_ops`]     |
//! | [`vt_100_shim_sgr_ops`]       | [`vt_100_impl_sgr_ops`]      | [`vt_100_test_sgr_ops`]        |
//! | [`vt_100_shim_terminal_ops`]  | [`vt_100_impl_terminal_ops`] | [`vt_100_test_terminal_ops`]   |
//!
//! See the [main module documentation](super) for the complete explanation of this
//! architectural pattern and its benefits for IDE navigation.
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`vt_100_ansi_conformance_tests`]: mod@super::vt_100_ansi_conformance_tests
//! [`vt_100_ansi_impl`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl
//! [`vt_100_shim_char_ops`]: vt_100_shim_char_ops
//! [`vt_100_impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_char_ops
//! [`vt_100_test_char_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_char_ops
//! [`vt_100_shim_control_ops`]: vt_100_shim_control_ops
//! [`vt_100_impl_control_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_control_ops
//! [`vt_100_test_control_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_control_ops
//! [`vt_100_shim_cursor_ops`]: vt_100_shim_cursor_ops
//! [`vt_100_impl_cursor_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_cursor_ops
//! [`vt_100_test_cursor_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_cursor_ops
//! [`vt_100_shim_dsr_ops`]: vt_100_shim_dsr_ops
//! [`vt_100_impl_dsr_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_dsr_ops
//! [`vt_100_test_dsr_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_dsr_ops
//! [`vt_100_shim_line_ops`]: vt_100_shim_line_ops
//! [`vt_100_impl_line_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_line_ops
//! [`vt_100_test_line_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_line_ops
//! [`vt_100_shim_margin_ops`]: vt_100_shim_margin_ops
//! [`vt_100_impl_margin_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_margin_ops
//! [`vt_100_test_margin_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_margin_ops
//! [`vt_100_shim_mode_ops`]: vt_100_shim_mode_ops
//! [`vt_100_impl_mode_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_mode_ops
//! [`vt_100_test_mode_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_mode_ops
//! [`vt_100_shim_osc_ops`]: vt_100_shim_osc_ops
//! [`vt_100_impl_osc_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_osc_ops
//! [`vt_100_test_osc_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_osc_ops
//! [`vt_100_shim_scroll_ops`]: vt_100_shim_scroll_ops
//! [`vt_100_impl_scroll_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_scroll_ops
//! [`vt_100_test_scroll_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_scroll_ops
//! [`vt_100_shim_sgr_ops`]: vt_100_shim_sgr_ops
//! [`vt_100_impl_sgr_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_sgr_ops
//! [`vt_100_test_sgr_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_sgr_ops
//! [`vt_100_shim_terminal_ops`]: vt_100_shim_terminal_ops
//! [`vt_100_impl_terminal_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_terminal_ops
//! [`vt_100_test_terminal_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_terminal_ops

pub mod vt_100_shim_char_ops;
pub mod vt_100_shim_control_ops;
pub mod vt_100_shim_cursor_ops;
pub mod vt_100_shim_dsr_ops;
pub mod vt_100_shim_line_ops;
pub mod vt_100_shim_margin_ops;
pub mod vt_100_shim_mode_ops;
pub mod vt_100_shim_osc_ops;
pub mod vt_100_shim_scroll_ops;
pub mod vt_100_shim_sgr_ops;
pub mod vt_100_shim_terminal_ops;

// Re-export all operations for easier access.
pub use vt_100_shim_char_ops::*;
pub use vt_100_shim_control_ops::*;
pub use vt_100_shim_cursor_ops::*;
pub use vt_100_shim_dsr_ops::*;
pub use vt_100_shim_line_ops::*;
pub use vt_100_shim_margin_ops::*;
pub use vt_100_shim_mode_ops::*;
pub use vt_100_shim_osc_ops::*;
pub use vt_100_shim_scroll_ops::*;
pub use vt_100_shim_sgr_ops::*;
pub use vt_100_shim_terminal_ops::*;
