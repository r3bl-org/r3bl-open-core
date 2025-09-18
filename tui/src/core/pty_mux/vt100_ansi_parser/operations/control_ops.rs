// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Control character operations.
//!
//! # Architecture Overview
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
//! # Control Character Processing Flow
//!
//! ```text
//! Application sends '\n' (0x0A Line Feed)
//!         ↓
//!     PTY Slave (control character)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (identifies C0 control chars)
//!         ↓
//!     execute() [routes to functions below]
//!         ↓
//!     Route to control operations:                          ╭───────────╮
//!       - control_ops:: for control chars (BS,TAB,LF,CR) <- │THIS MODULE│
//!         ↓                                                 ╰───────────╯
//!     Update OffscreenBuffer state
//! ```
//!
//! # Supported Control Characters
//!
//! This module handles the basic C0 control characters that are essential for
//! terminal operation:
//!
//! - **BS (0x08)**: Backspace - move cursor left one position
//! - **TAB (0x09)**: Horizontal Tab - move cursor to next tab stop
//! - **LF (0x0A)**: Line Feed - move cursor down one line
//! - **CR (0x0D)**: Carriage Return - move cursor to start of current line
//!
//! These operations are fundamental cursor control operations that don't require
//! parameters, unlike their CSI sequence counterparts.

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;

/// Handle BS (Backspace) - move cursor left one position.
/// If cursor is at start of line, behavior depends on terminal settings.
/// See `OffscreenBuffer::handle_backspace` for detailed behavior.
pub fn handle_backspace(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_backspace();
}

/// Handle TAB (Horizontal Tab) - move cursor to next tab stop.
/// Tab stops are typically at columns 0, 8, 16, 24, 32, etc.
/// See `OffscreenBuffer::handle_tab` for detailed behavior and examples.
pub fn handle_tab(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_tab();
}

/// Handle LF (Line Feed) - move cursor down one line.
/// Cursor column position remains unchanged.
/// See `OffscreenBuffer::handle_line_feed` for detailed behavior.
pub fn handle_line_feed(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_line_feed();
}

/// Handle CR (Carriage Return) - move cursor to start of current line.
/// Cursor row position remains unchanged.
/// See `OffscreenBuffer::handle_carriage_return` for detailed behavior.
pub fn handle_carriage_return(performer: &mut AnsiToOfsBufPerformer) {
    performer.ofs_buf.handle_carriage_return();
}
