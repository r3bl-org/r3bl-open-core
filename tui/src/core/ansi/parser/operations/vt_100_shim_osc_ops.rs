// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! OSC (Operating System Command) sequence operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the operations module for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_osc_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_osc_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full
//!   pipeline
//!
//! For the complete testing philosophy,
//! and rationale behind this approach.
//!
//! # Architecture Overview
//!
//! ```text
//! ╭─────────────────╮    ╭───────────────╮    ╭─────────────────╮    ╭──────────────╮
//! │ Child Process   │────▶ PTY Master    │────▶ VTE Parser      │────▶ OffscreenBuf │
//! │ (vim, bash...)  │    │ (byte stream) │    │ (state machine) │    │ (terminal    │
//! ╰─────────────────╯    ╰───────────────╯    ╰─────────────────╯    │  buffer)     │
//!        │                                            │              ╰──────────────╯
//!        │                                            │                      │
//!        │                                   ╔════════▼════════╗             │
//!        │                                   ║ Perform Trait   ║             │
//!        │                                   ║ Implementation  ║             │
//!        │                                   ╚═════════════════╝             │
//!        │                                                                   │
//!        │                                   ╭─────────────────╮             │
//!        │                                   │ RenderPipeline  ◀─────────────╯
//!        │                                   │ paint()         │
//!        ╰───────────────────────────────────▶ Terminal Output │
//!                                            ╰─────────────────╯
//! ```
//!
//! # OSC Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC]0;My Title\007"
//!         ↓
//!     PTY Slave (OSC sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (accumulates OSC params)
//!         ↓
//!     osc_dispatch() [routes to functions below]
//!         ↓
//!     Route to OSC operations:                            ╭───────────╮
//!       - osc_ops:: for OS commands (title, hyperlink) <- │THIS MODULE│
//!         ↓                                               ╰───────────╯
//!     Queue OscEvent for later rendering
//! ```
//!
//! # Supported OSC Sequences
//!
//! This module handles Operating System Command sequences that provide
//! integration between terminal applications and the operating system:
//!
//! - **OSC 0**: Set both window title and icon name
//! - **OSC 1**: Set icon name only (treated same as title)
//! - **OSC 2**: Set window title only
//! - **OSC 8**: Create hyperlinks (format: OSC 8 ; params ; URI)
//!
//! OSC sequences are queued as events for later processing by the output renderer.
//!
//! [`impl_osc_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::vt_100_impl_osc_ops
//! [`test_osc_ops`]: crate::core::ansi::parser::vt_100_ansi_conformance_tests::tests::vt_100_test_osc_ops
//! [module-level documentation]: self

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::core::osc::osc_codes;

/// Handle OSC dispatch - process all OSC (Operating System Command) sequences.
/// This is the main entry point for OSC sequence processing.
/// See individual helper functions for specific OSC code handling.
pub fn dispatch_osc(
    performer: &mut AnsiToOfsBufPerformer,
    params: &[&[u8]],
    _bell_terminated: bool,
) {
    if params.is_empty() {
        return;
    }

    // Parse the OSC code (first parameter).
    if let Ok(code) = std::str::from_utf8(params[0]) {
        match code {
            // OSC 0: Set both window title and icon name.
            // OSC 1: Set icon name only (we treat same as title).
            // OSC 2: Set window title only.
            osc_codes::OSC_CODE_TITLE_AND_ICON
            | osc_codes::OSC_CODE_ICON
            | osc_codes::OSC_CODE_TITLE
                if params.len() > 1 =>
            {
                if let Ok(title) = std::str::from_utf8(params[1]) {
                    handle_title_and_icon(performer, title);
                }
            }
            // OSC 8: Hyperlink (format: OSC 8 ; params ; URI).
            osc_codes::OSC_CODE_HYPERLINK if params.len() > 2 => {
                if let Ok(uri) = std::str::from_utf8(params[2]) {
                    handle_hyperlink(performer, uri);
                }
            }
            // OSC 9;4: Progress sequences (already handled by OscBuffer in some
            // contexts) We could handle them here too if needed.
            _ => {
                // Ignore other OSC sequences for now.
            }
        }
    }
}

/// Handle OSC title and icon sequences (OSC 0, 1, 2).
/// Sets window title and/or icon name.
/// Queues [`SetTitleAndTab`] event for later processing by output renderer.
///
/// [`SetTitleAndTab`]: crate::OscEvent::SetTitleAndTab
pub fn handle_title_and_icon(performer: &mut AnsiToOfsBufPerformer, title: &str) {
    performer.ofs_buf.handle_title_and_icon(title);
}

/// Handle OSC 8 hyperlink sequences.
/// Creates hyperlinks with URI for later processing.
/// The display text is handled separately via `print()` calls.
/// Queues Hyperlink event for later processing by output renderer.
pub fn handle_hyperlink(performer: &mut AnsiToOfsBufPerformer, uri: &str) {
    performer.ofs_buf.handle_hyperlink(uri);
}
