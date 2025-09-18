// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT100/ANSI terminal operation implementations for `OffscreenBuffer`.
//!
//! This module contains the actual implementations of VT100 and ANSI escape sequence
//! operations that are delegated from the `vt_100_ansi_parser::operations` module. The
//! structure mirrors `vt_100_ansi_parser/operations/` to provide a clear 1:1 mapping
//! between the parser shim layer and the implementation layer.
//!
//! # Architecture
//!
//! ```text
//! vt_100_ansi_parser/operations/char_ops.rs     (shim - minimal logic)
//!           ↓ delegates to
//! vt_100_ansi_impl/impl_char_ops.rs     (implementation - full logic)
//! ```
//!
//! ## The `impl_` Prefix Convention
//!
//! All implementation files in this module use the `impl_` prefix. This deliberate
//! naming convention creates a searchable namespace that distinguishes implementations
//! from their corresponding shim layers and tests. When searching for any operation
//! (e.g., "char_ops"), developers can easily identify:
//! - The shim (no prefix): Protocol translation
//! - The implementation (`impl_` prefix): Business logic
//! - The tests (`test_` prefix): Validation
//!
//! This naming pattern solves the IDE search problem by creating a predictable hierarchy
//! that makes code navigation efficient and intuitive. See [parser module documentation](crate::core::pty_mux::vt_100_ansi_parser)
//! for the complete architectural pattern explanation.
//!
//! # Module Organization
//!
//! Each file corresponds directly to a file in `vt_100_ansi_parser/operations/`:
//!
//! - [`impl_char_ops`] - Character operations (`print_char`, ICH, DCH, ECH)
//! - [`impl_control_ops`] - Control character handling (BS, TAB, LF, CR)
//! - [`impl_cursor_ops`] - Cursor movement operations
//! - [`impl_dsr_ops`] - Device Status Report operations
//! - [`impl_line_ops`] - Line manipulation operations
//! - [`impl_margin_ops`] - Scroll margin operations (DECSTBM)
//! - [`impl_mode_ops`] - Mode setting operations (SM/RM)
//! - [`impl_osc_ops`] - Operating System Command operations
//! - [`impl_scroll_ops`] - Scrolling operations
//! - [`impl_sgr_ops`] - Select Graphic Rendition operations (styling)
//! - [`impl_terminal_ops`] - Terminal state operations (reset, clear, charset)
//! - [`ansi_bounds_check_helper`] - ANSI-specific bounds checking utilities
//!
//! # Testing Approach
//!
//! This module follows a **dual-layer testing strategy** that complements the three-layer
//! architecture:
//!
//! ## Unit Tests in Implementation Files
//!
//! Each `impl_*.rs` file contains comprehensive unit tests using `#[test]` functions:
//!
//! ```text
//! impl_char_ops.rs ──── Contains unit tests for:
//!     ├── insert_chars_at_cursor_basic()
//!     ├── delete_chars_at_cursor_overflow()
//!     ├── erase_chars_at_end_of_line()
//!     └── ... (dozens of focused unit tests)
//! ```
//!
//! These unit tests directly call implementation methods without going through the ANSI
//! parsing pipeline, allowing for:
//! - **Isolated Logic Testing**: Test edge cases and boundary conditions
//! - **Fast Execution**: No ANSI parsing overhead
//! - **Precise Error Diagnosis**: Pinpoint exact implementation bugs
//!
//! ## Integration Testing Relationship
//!
//! While this module contains unit tests, the **complete pipeline testing** is handled by
//! the integration tests in [`vt_100_ansi_conformance_tests`]. This creates two
//! complementary test layers:
//!
//! ```text
//! Integration Tests (conformance_tests) ─── Tests complete pipeline:
//!     ANSI bytes → VTE parser → shim → impl → buffer update
//!
//! Unit Tests (this module) ─── Tests isolated implementation:
//!     Direct impl method calls → buffer update
//! ```
//!
//! ## Navigation Between Testing Layers
//!
//! When working on any implementation file, you can navigate to its related layers:
//! - **Shim Layer**: [`operations`] - The delegation layer that calls these implementations
//! - **Integration Tests**: [`vt_100_ansi_conformance_tests`] - Tests the complete ANSI pipeline
//! - **Testing Philosophy**: See [parser module docs] for the complete three-layer strategy
//!
//! For example, when working on character operations:
//! 1. **Implementation**: [`impl_char_ops`] (this module) - Unit tests for buffer logic
//! 2. **Shim**: [`operations::char_ops`] - Parameter translation (no direct tests)
//! 3. **Integration**: [`test_char_ops`] - Full ANSI sequence testing
//!
//! # VT100 Compliance
//!
//! These implementations follow VT100 terminal specifications and are tested for
//! compliance in the [`vt_100_ansi_conformance_tests`] module.
//!
//! [`operations`]: crate::core::pty_mux::vt_100_ansi_parser::operations
//! [`vt_100_ansi_conformance_tests`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests
//! [`operations::char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::operations::char_ops
//! [`test_char_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_char_ops
//! [parser module docs]: crate::core::pty_mux::vt_100_ansi_parser

/// Standard terminal tab stop width (8 columns).
/// Used for calculating tab positions in VT100 terminal emulation.
/// This is a widely-adopted standard across most terminal emulators.
pub const TAB_STOP_WIDTH: usize = 8;

// Attach modules.
pub mod ansi_bounds_check_helper;
pub mod impl_char_ops;
pub mod impl_control_ops;
pub mod impl_cursor_ops;
pub mod impl_dsr_ops;
pub mod impl_line_ops;
pub mod impl_margin_ops;
pub mod impl_mode_ops;
pub mod impl_osc_ops;
pub mod impl_scroll_ops;
pub mod impl_sgr_ops;
pub mod impl_terminal_ops;

// Note: Individual modules are typically accessed directly by their respective
// vt_100_ansi_parser operation files. No re-exports needed here.
