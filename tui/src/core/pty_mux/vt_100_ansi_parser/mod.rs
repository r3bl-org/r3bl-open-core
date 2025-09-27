// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence parsing for PTY multiplexer.
//!
//! This module provides a comprehensive VT100-compliant ANSI escape sequence parser
//! that processes PTY output and updates an [`OffscreenBuffer`] accordingly. It uses
//! the `vte` crate (same as Alacritty terminal) for robust ANSI parsing with a proven
//! state machine implementation.
//!
//! # Architecture Overview
//!
//! The ANSI parser implements a pipeline that transforms raw PTY output into structured
//! terminal buffer updates:
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master    │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            ▼                      │
//!        │                                   ╔═════════════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  │◀────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # Core Components
//!
//! - **[`ansi_parser_public_api`]**: Public API for ANSI sequence processing
//! - **[`perform`]**: VTE `Perform` trait implementation with detailed architecture docs
//! - **[`protocols`]**: ANSI sequence builders (`CsiSequence`, `EscSequence`, `SgrCode`)
//! - **[`operations`]**: Modular operation handlers (cursor, SGR, scrolling, etc.)
//! - **[`term_units`]**: Type-safe terminal coordinate system
//!
//! # Code Organization and Naming Convention
//!
//! This module follows a deliberate three-layer architecture with a specific naming
//! pattern that solves the IDE search problem and creates clear code boundaries:
//!
//! ## The Three-Layer Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────┐
//! │                     Layer 1: SHIM (no prefix)                  │
//! │  vt_100_ansi_parser/operations/char_ops.rs                     │
//! │  • Thin delegation layer                                       │
//! │  • Parameter parsing and translation                           │
//! │  • Minimal logic, maximum clarity                              │
//! └────────────────────────────────────────────────────────────────┘
//!                               ↓ delegates to
//! ┌────────────────────────────────────────────────────────────────┐
//! │                  Layer 2: IMPLEMENTATION (impl_ prefix)        │
//! │  offscreen_buffer/vt_100_ansi_impl/impl_char_ops.rs            │
//! │  • Full business logic                                         │
//! │  • Buffer manipulation                                         │
//! │  • VT100 compliance implementation                             │
//! └────────────────────────────────────────────────────────────────┘
//!                               ↓ tested by
//! ┌────────────────────────────────────────────────────────────────┐
//! │                     Layer 3: TESTS (test_ prefix)              │
//! │  vt_100_ansi_conformance_tests/tests/test_char_ops.rs          │
//! │  • Comprehensive test coverage                                 │
//! │  • VT100 conformance validation                                │
//! │  • Real-world scenario testing                                 │
//! └────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## The IDE Search Problem (and Our Solution)
//!
//! **Problem**: When searching for "`char_ops`" in a large codebase, IDEs typically
//! return many unrelated results, making it difficult to navigate between related
//! files.
//!
//! **Solution**: Our naming convention creates a searchable, hierarchical namespace:
//!
//! When you search for "`char_ops`" in your IDE, you'll see:
//! - `char_ops.rs` - The parser shim (clean, minimal delegation)
//! - `impl_char_ops.rs` - The full implementation
//! - `test_char_ops.rs` - The test files
//!
//! This predictable pattern means developers can quickly jump between:
//! - The protocol definition (shim)
//! - The implementation details (impl)
//! - The test specifications (test)
//!
//! ## Benefits of This Architecture
//!
//! 1. **Clear Separation of Concerns**: Protocol handling vs. business logic
//! 2. **Predictable Navigation**: Consistent naming across all operations
//! 3. **Reduced Cognitive Load**: Know exactly where to look for each aspect
//! 4. **Maintainability**: Changes to protocol don't affect implementation
//! 5. **Testability**: Each layer can be tested independently
//!
//! ## File Naming Rules
//!
//! | Layer | Prefix | Example | Purpose |
//! |-------|--------|---------|---------|
//! | Shim | (none) | `char_ops.rs` | Protocol translation |
//! | Implementation | `impl_` | `impl_char_ops.rs` | Business logic |
//! | Test | `test_` | `test_char_ops.rs` | Validation |
//!
//! This pattern is consistently applied across all operation types:
//! - Character operations (`char_ops` → `impl_char_ops` → `test_char_ops`)
//! - Cursor operations (`cursor_ops` → `impl_cursor_ops` → `test_cursor_ops`)
//! - Terminal operations (`terminal_ops` → `impl_terminal_ops` → `test_terminal_ops`)
//! - And all others...
//!
//! # VT100 Specification Compliance
//!
//! This parser implements VT100 terminal compatibility as documented in:
//! - [VT100 User Guide](https://vt100.net/docs/vt100-ug/)
//! - [ANSI X3.64 Standard](https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
//! - [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
//!
//! **Supported Sequence Types:**
//! - **CSI sequences**: Cursor movement, text styling, scrolling, device control
//! - **ESC sequences**: Simple escape commands, character set selection
//! - **OSC sequences**: Operating system commands (window titles, etc.)
//! - **Control characters**: Backspace, tab, line feed, carriage return
//! - **SGR codes**: Text styling (colors, bold, italic, underline)
//!
//! ## Intentionally Unimplemented VT100 Features
//!
//! `PTY_MUX` is designed as a **modern terminal multiplexer** focused on multiplexing
//! contemporary TUI applications, interactive shells, and command-line tools. As such,
//! certain legacy VT100 features that are primarily used by mainframe terminals and
//! very old applications are intentionally **not implemented**:
//!
//! ### Legacy Tab Control Sequences
//! - **HTS (ESC H)**: Horizontal Tab Set - Modern apps use fixed 8-column tab stops
//! - **TBC (CSI g)**: Tab Clear - Custom tab stops rarely used outside legacy systems
//! - **CHT (CSI I)**: Cursor Horizontal Tab - Basic TAB (0x09) is sufficient
//! - **CBT (CSI Z)**: Cursor Backward Tab - Reverse tab navigation rarely needed
//!
//! ### Legacy Line Control
//! - **NEL (ESC E)**: Next Line - Modern apps use standard `\n` (LF) or `\r\n` (CRLF)
//!
//! ### Legacy Terminal Modes
//! - **IRM**: Insert/Replace Mode - Modern TUI apps manage their own insert modes
//! - **DECOM**: Origin Mode - Rarely used absolute vs relative positioning mode
//!
//! ### Design Rationale
//!
//! These features were omitted because:
//! 1. **Modern applications don't use them** - bash, vim, tmux, ncurses apps work without
//!    them
//! 2. **Legacy mainframe focus** - Primarily used by IBM terminals and EBCDIC systems
//! 3. **Complexity vs utility** - Implementation complexity outweighs benefit for target
//!    use cases
//! 4. **Alternative implementations exist** - Applications needing full VT100 emulation
//!    should use xterm, iTerm2, etc.
//!
//! `PTY_MUX` implements the **80% of VT100 features that 99% of modern applications
//! use**, providing excellent compatibility for contemporary terminal applications while
//! maintaining implementation simplicity and focus.
//!
//! # Testing Philosophy and Architecture
//!
//! This module employs a **deliberate three-layer testing strategy** that perfectly
//! aligns with the shim → impl → test architecture:
//!
//! ## Testing Strategy Overview
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                        LAYER 1: SHIM (no tests)                            │
//! │  operations/char_ops.rs                                                    │
//! │  • Pure delegation, no business logic                                      │
//! │  • No direct unit tests (intentional!)                                     │
//! │  • Tested indirectly via integration tests                                 │
//! └────────────────────────────────────────────────────────────────────────────┘
//!                                     ↓ delegates to
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                    LAYER 2: IMPLEMENTATION (unit tests)                    │
//! │  vt_100_ansi_impl/impl_char_ops.rs                                         │
//! │  • Full business logic and buffer manipulation                             │
//! │  • Comprehensive unit tests (#[test] functions)                            │
//! │  • Tests isolated logic without ANSI parsing                               │
//! └────────────────────────────────────────────────────────────────────────────┘
//!                                     ↓ tested by
//! ┌────────────────────────────────────────────────────────────────────────────┐
//! │                    LAYER 3: INTEGRATION TESTS (full pipeline)              │
//! │  vt_100_ansi_conformance_tests/tests/test_char_ops.rs                      │
//! │  • Tests complete ANSI sequence → buffer update pipeline                   │
//! │  • Uses public API (apply_ansi_bytes) for real-world scenarios             │
//! │  • VT100 conformance validation                                            │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## Why Shims Have No Direct Tests
//!
//! The **intentional absence** of unit tests in the shim layer (operations modules) is a
//! deliberate architectural decision:
//!
//! 1. **Pure Delegation**: Shims contain no business logic, only parameter translation
//! 2. **No Risk**: Minimal code means minimal risk of bugs
//! 3. **Integration Coverage**: Conformance tests exercise the complete pipeline
//!    including shims
//! 4. **Avoid Redundancy**: Testing delegation would duplicate impl layer unit tests
//!
//! ## Testing Layer Relationships
//!
//! ```text
//! apply_ansi_bytes("ESC[2P")
//!         ↓
//! VTE Parser → shim (char_ops::delete_chars) → impl (delete_chars_at_cursor) → buffer
//!                     ↑                                    ↑                      ↑
//!                NO TESTS                         UNIT TESTS              INTEGRATION TESTS
//!            (intentional!)                    (#[test] fns)           (conformance tests)
//! ```
//!
//! ## Navigation Between Testing Layers
//!
//! The three layers are tightly coupled and designed for seamless navigation:
//! - **Shim Layer**: [`operations`] - Parameter translation and delegation
//! - **Implementation Layer**: [`vt_100_ansi_impl`] - Business logic with unit tests
//! - **Integration Tests**: [`vt_100_ansi_conformance_tests`] - Full pipeline validation
//!
//! When working on any operation (e.g., character operations), you can easily jump
//! between:
//! 1. The protocol interface ([`operations::char_ops`])
//! 2. The implementation ([`vt_100_ansi_impl::impl_char_ops`])
//! 3. The integration tests ([`vt_100_ansi_conformance_tests::tests::test_char_ops`])
//!
//! # Testing Infrastructure
//!
//! The module includes comprehensive conformance testing to ensure VT100 compatibility:
//!
//! ## Type-Safe Sequence Builders
//!
//! Instead of hardcoded escape strings, tests use type-safe builders that provide
//! compile-time validation and clear semantic intent:
//!
//! ```rust
//! use r3bl_tui::{ANSIBasicColor, SgrCode};
//!
//! // ❌ Hardcoded escape sequences (error-prone)
//! let sequence = "\x1b[31mHello\x1b[0m";
//!
//! // ✅ Type-safe builders (compile-time validated)
//! let sequence = format!("{}Hello{}",
//!     SgrCode::ForegroundBasic(ANSIBasicColor::Red),
//!     SgrCode::Reset
//! );
//! ```
//!
//! ## Conformance Test Categories
//!
//! - **[`mod@vt_100_ansi_conformance_tests`]**: Comprehensive VT100 standard compliance
//! - **Real-world scenarios**: vim, emacs, tmux terminal application patterns
//! - **Edge cases**: Malformed sequences, boundary conditions, stress testing
//! - **Performance validation**: Large text blocks, rapid style changes
//!
//! ## Running Conformance Tests
//!
//! ```bash
//! # All conformance tests (101+ tests)
//! cargo test vt_100_ansi_conformance_tests
//!
//! # Specific test categories
//! cargo test test_real_world_scenarios      # vim, emacs, tmux patterns
//! cargo test test_cursor_operations         # cursor positioning
//! cargo test test_sgr_and_character_sets    # text styling & colors
//! ```
//!
//! # Usage Example
//!
//! ```rust
//! use r3bl_tui::{*, height, width};
//!
//! // Create terminal buffer
//! let mut buffer = OffscreenBuffer::new_empty(height(24) + width(80));
//!
//! // Process ANSI sequences from PTY
//! let pty_output = b"\x1b[31mRed text\x1b[0m\x1b[2;3HPositioned text";
//! let (osc_events, dsr_responses) = buffer.apply_ansi_bytes(pty_output);
//!
//! // Buffer now contains styled text at correct positions
//! ```
//!
//! # Performance Characteristics
//!
//! - **Zero-copy parsing**: VTE parser processes bytes directly without string allocation
//! - **Incremental updates**: Only modified buffer regions are updated
//! - **Bounds checking**: All operations are bounds-checked to prevent crashes
//! - **Memory efficient**: Sequences are processed as they arrive, no buffering overhead
//!
//! [`OffscreenBuffer`]: crate::OffscreenBuffer
//! [`vt_100_ansi_impl`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl
//! [`vt_100_ansi_impl::impl_char_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_char_ops
//! [`operations::char_ops`]: operations::char_ops
//! [`vt_100_ansi_conformance_tests::tests::test_char_ops`]: vt_100_ansi_conformance_tests::tests::test_char_ops

// Attach.
pub mod ansi_parser_public_api;
pub mod ansi_to_tui_color;
pub mod operations;
pub mod param_utils;
pub mod perform;
pub mod protocols;
pub mod term_units;

// Re-export.
pub use ansi_parser_public_api::*;
pub use operations::*;
pub use param_utils::*;
pub use protocols::*;
pub use term_units::*;

// VT100 ANSI conformance test modules.
#[cfg(any(test, doc))]
pub mod vt_100_ansi_conformance_tests;
