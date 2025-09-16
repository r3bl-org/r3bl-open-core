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
//! ╭─────────────────╮    ╭──────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │───▶│ PTY Master   │───▶│ VTE Parser      │───▶│ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream)│    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰──────────────╯    ╰─────────────────╯    │  buffer)     │
//!                                                     │             ╰──────────────╯
//!                                                     ▼
//!                                            ╭─────────────────╮
//!                                            │ Perform Trait   │
//!                                            │ Implementation  │
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
//! PTY_MUX is designed as a **modern terminal multiplexer** focused on multiplexing
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
//! 1. **Modern applications don't use them** - bash, vim, tmux, ncurses apps work without them
//! 2. **Legacy mainframe focus** - Primarily used by IBM terminals and EBCDIC systems
//! 3. **Complexity vs utility** - Implementation complexity outweighs benefit for target use cases
//! 4. **Alternative implementations exist** - Applications needing full VT100 emulation should use xterm, iTerm2, etc.
//!
//! PTY_MUX implements the **80% of VT100 features that 99% of modern applications use**,
//! providing excellent compatibility for contemporary terminal applications while
//! maintaining implementation simplicity and focus.
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
//! - **[`vt_100_ansi_conformance_tests`]**: Comprehensive VT100 standard compliance
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
//! cargo test test_real_world_scenarios     # vim, emacs, tmux patterns
//! cargo test test_cursor_operations         # cursor positioning
//! cargo test test_sgr_and_character_sets    # text styling & colors
//! ```
//!
//! # Usage Example
//!
//! ```rust
//! use r3bl_tui::*;
//!
//! // Create terminal buffer
//! let mut buffer = OffscreenBuffer::new_empty(size!(24, 80));
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
#[cfg(test)]
mod vt_100_ansi_conformance_tests;
