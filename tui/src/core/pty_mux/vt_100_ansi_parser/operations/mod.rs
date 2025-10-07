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
//! - [`char_ops.rs`] (this directory) - The shim layer for protocol translation
//! - [`impl_char_ops.rs`] (implementation) - The business logic layer
//! - [`test_char_ops.rs`] (tests) - The validation layer
//!
//! This same pattern applies to all operation types:
//!
//! | Ops | Shim | Tests |
//! |-----|------|-------|
//! | [`char_ops.rs`] | [`impl_char_ops.rs`] | [`test_char_ops.rs`] |
//! | [`control_ops.rs`] | [`impl_control_ops.rs`] | [`test_control_ops.rs`] |
//! | [`cursor_ops.rs`] | [`impl_cursor_ops.rs`] | [`test_cursor_ops.rs`] |
//! | [`dsr_ops.rs`] | [`impl_dsr_ops.rs`] | [`test_dsr_ops.rs`] |
//! | [`line_ops.rs`] | [`impl_line_ops.rs`] | [`test_line_ops.rs`] |
//! | [`margin_ops.rs`] | [`impl_margin_ops.rs`] | [`test_margin_ops.rs`] |
//! | [`mode_ops.rs`] | [`impl_mode_ops.rs`] | [`test_mode_ops.rs`] |
//! | [`osc_ops.rs`] | [`impl_osc_ops.rs`] | [`test_osc_ops.rs`] |
//! | [`scroll_ops.rs`] | [`impl_scroll_ops.rs`] | [`test_scroll_ops.rs`] |
//! | [`sgr_ops.rs`] | [`impl_sgr_ops.rs`] | [`test_sgr_ops.rs`] |
//! | [`terminal_ops.rs`] | [`impl_terminal_ops.rs`] | [`test_terminal_ops.rs`] |
//!
//! See the [main module documentation](super) for the complete explanation of this
//! architectural pattern and its benefits for IDE navigation.
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`vt_100_ansi_conformance_tests`]: mod@super::vt_100_ansi_conformance_tests
//! [`vt_100_ansi_impl`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl
//! [`char_ops.rs`]: char_ops
//! [`impl_char_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_char_ops
//! [`test_char_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_char_ops
//! [`control_ops.rs`]: control_ops
//! [`impl_control_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_control_ops
//! [`test_control_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_control_ops
//! [`cursor_ops.rs`]: cursor_ops
//! [`impl_cursor_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_cursor_ops
//! [`test_cursor_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_cursor_ops
//! [`dsr_ops.rs`]: dsr_ops
//! [`impl_dsr_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_dsr_ops
//! [`test_dsr_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_dsr_ops
//! [`line_ops.rs`]: line_ops
//! [`impl_line_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_line_ops
//! [`test_line_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_line_ops
//! [`margin_ops.rs`]: margin_ops
//! [`impl_margin_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_margin_ops
//! [`test_margin_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_margin_ops
//! [`mode_ops.rs`]: mode_ops
//! [`impl_mode_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_mode_ops
//! [`test_mode_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_mode_ops
//! [`osc_ops.rs`]: osc_ops
//! [`impl_osc_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_osc_ops
//! [`test_osc_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_osc_ops
//! [`scroll_ops.rs`]: scroll_ops
//! [`impl_scroll_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_scroll_ops
//! [`test_scroll_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_scroll_ops
//! [`sgr_ops.rs`]: sgr_ops
//! [`impl_sgr_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_sgr_ops
//! [`test_sgr_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_sgr_ops
//! [`terminal_ops.rs`]: terminal_ops
//! [`impl_terminal_ops.rs`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_terminal_ops
//! [`test_terminal_ops.rs`]: mod@super::super::vt_100_ansi_conformance_tests::tests::test_terminal_ops

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
