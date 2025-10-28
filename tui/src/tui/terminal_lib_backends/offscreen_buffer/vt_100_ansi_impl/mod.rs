// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! VT100/ANSI terminal operation implementations for `OffscreenBuffer`.
//!
//! This module contains the actual implementations of VT100 and ANSI escape sequence
//! operations that are delegated from the `vt_100_pty_output_parser::operations` module.
//! The structure mirrors `vt_100_pty_output_parser/operations/` to provide a clear 1:1
//! mapping between the parser shim layer and the implementation layer.
//!
//! # Architecture
//!
//! ```text
//! vt_100_pty_output_parser/operations/vt_100_shim_char_ops <- (shim - minimal logic)
//!           ↓ delegates to
//! vt_100_ansi_impl/vt_100_impl_char_ops                  <- (implementation - full logic)
//! ```
//!
//! ## The `impl_` Prefix Convention
//!
//! All implementation files in this module use the `impl_` prefix. This deliberate
//! naming convention creates a searchable namespace that distinguishes implementations
//! from their corresponding shim layers and tests. When searching for any operation
//! (e.g., "`char_ops`"), developers can easily identify:
//! - The shim (no prefix): Protocol translation
//! - The implementation (`impl_` prefix): Business logic
//! - The tests (`test_` prefix): Validation
//!
//! This naming pattern solves the IDE search problem by creating a predictable hierarchy
//! that makes code navigation efficient and intuitive. See [parser module
//! documentation for the complete
//! architectural pattern explanation.
//!
//! # Module Organization
//!
//! Each file corresponds directly to a file in `vt_100_pty_output_parser/operations/`:
//!
//! - [`vt_100_impl_char_ops`] - Character operations (`print_char`, ICH, DCH, ECH)
//! - [`vt_100_impl_control_ops`] - Control character handling (BS, TAB, LF, CR)
//! - [`vt_100_impl_cursor_ops`] - Cursor movement operations
//! - [`vt_100_impl_dsr_ops`] - Device Status Report operations
//! - [`vt_100_impl_line_ops`] - Line manipulation operations
//! - [`vt_100_impl_margin_ops`] - Scroll margin operations (DECSTBM)
//! - [`vt_100_impl_mode_ops`] - Mode setting operations (SM/RM)
//! - [`vt_100_impl_osc_ops`] - Operating System Command operations
//! - [`vt_100_impl_scroll_ops`] - Scrolling operations
//! - [`vt_100_impl_sgr_ops`] - Select Graphic Rendition operations (styling)
//! - [`vt_100_impl_terminal_ops`] - Terminal state operations (reset, clear, charset)
//! - [`vt_100_impl_ansi_scroll_helper`] - ANSI scroll region helper utilities
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
//! vt_100_impl_char_ops ──── Contains unit tests for:
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
//! integration tests for VT100 conformance. This creates two complementary test layers:
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
//! - **Shim Layer** - The delegation layer that calls these implementations
//! - **Integration Tests** - Tests the complete ANSI pipeline
//! - **Testing Philosophy**: See the three-layer architecture documentation above for
//!   strategy
//!
//! For example, when working on character operations:
//! 1. **Implementation** - Unit tests for buffer logic
//! 2. **Shim** - Parameter translation (no direct tests)
//! 3. **Integration** - Full ANSI sequence testing
//!
//! ## Complete Navigation Map
//!
//! All operation types follow the same three-layer pattern. From any implementation file,
//! you can navigate to its corresponding shim and test layers, or by using IDE search
//! with the operation name to find all related files.
//!
//!
//! # VT100 Compliance
//!
//! These implementations follow VT100 terminal specifications.
//!
//!
//! // Implementation layer hyperlinks (this module)
//! [`vt_100_impl_char_ops`]: `impl_char_ops`
//! [`vt_100_impl_control_ops`]: `impl_control_ops`
//! [`vt_100_impl_cursor_ops`]: `impl_cursor_ops`
//! [`vt_100_impl_dsr_ops`]: `impl_dsr_ops`
//! [`vt_100_impl_line_ops`]: `impl_line_ops`
//! [`vt_100_impl_margin_ops`]: `impl_margin_ops`
//! [`vt_100_impl_mode_ops`]: `impl_mode_ops`
//! [`vt_100_impl_osc_ops`]: `impl_osc_ops`
//! [`vt_100_impl_scroll_ops`]: `impl_scroll_ops`
//! [`vt_100_impl_sgr_ops`]: `impl_sgr_ops`
//! [`vt_100_impl_terminal_ops`]: `impl_terminal_ops`
//! [`vt_100_impl_ansi_scroll_helper`]: `ansi_scroll_helper`

/// Standard terminal tab stop width (8 columns).
/// Used for calculating tab positions in VT100 terminal emulation.
/// This is a widely-adopted standard across most terminal emulators.
pub const TAB_STOP_WIDTH: usize = 8;

// Attach modules.
pub mod vt_100_impl_ansi_scroll_helper;
pub mod vt_100_impl_char_ops;
pub mod vt_100_impl_control_ops;
pub mod vt_100_impl_cursor_ops;
pub mod vt_100_impl_dsr_ops;
pub mod vt_100_impl_line_ops;
pub mod vt_100_impl_margin_ops;
pub mod vt_100_impl_mode_ops;
pub mod vt_100_impl_osc_ops;
pub mod vt_100_impl_scroll_ops;
pub mod vt_100_impl_sgr_ops;
pub mod vt_100_impl_terminal_ops;

// Note: Individual modules are typically accessed directly by their respective
// vt_100_pty_output_parser operation files. No re-exports needed here.
