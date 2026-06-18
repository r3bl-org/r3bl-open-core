// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`OSC`] (Operating System Command) sequence operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_osc_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_osc_ops`] - Full pipeline testing via public API
//!
//! # Testing Strategy
//!
//! **This shim layer intentionally has no direct unit tests.**
//!
//! This is a deliberate architectural decision: these functions are pure delegation
//! layers with no business logic. Testing is comprehensively handled by:
//! - **Unit tests** in the implementation layer (with `#[test]` functions)
//! - **Integration tests** in the conformance tests validating the full pipeline
//!
//! For the complete testing philosophy and rationale behind this approach,
//! see the [ops module].
//!
//! # Architecture Overview
//!
//! See the [module-level Architecture Overview].
//!
//! # [`OSC`] Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC ]0;My Title\007"
//!         ↓
//!     PTY Controlled (`OSC` sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (accumulates `OSC` params)
//!         ↓
//!     osc_dispatch() [routes to functions below]
//!         ↓
//!     Route to `OSC` operations:                               ╭───────────╮
//!       - osc_ops:: for OS commands (title, hyperlink)      <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Queue OscEvent for later rendering
//! ```
//!
//! # Supported [`OSC`] Sequences
//!
//! This module handles Operating System Command sequences that provide
//! integration between terminal applications and the operating system:
//!
//! - **`OSC 0`**: Set both window title and icon name
//! - **`OSC 1`**: Set icon name only (treated same as title)
//! - **`OSC 2`**: Set window title only
//! - **`OSC 8`**: Create hyperlinks (Format: `ESC ] 8 ; params ; URI ESC \ (ST)`)
//!
//! [`OSC`] sequences are queued as events for later processing by the output renderer.
//!
//! [`OSC`]: crate::osc_codes::OscSequence
//! [`test_osc_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_osc_ops
//! [`vt_100_impl_osc_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_osc_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::ansi_parser_public_api::AnsiToOfsBufPerformer;
use crate::core::osc::osc_codes;

/// Handle [`OSC`] dispatch - process all [`OSC`] (Operating System Command) sequences.
/// This is the main entry point for [`OSC`] sequence processing.
/// See individual helper functions for specific [`OSC`] code handling.
///
/// [`OSC`]: crate::osc_codes::OscSequence
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

/// Handle [`OSC`] title and icon sequences (`OSC 0`, `OSC 1`, `OSC 2`).
/// Sets window title and/or icon name.
/// Queues [`SetTitleAndTab`] event for later processing by output renderer.
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`SetTitleAndTab`]: crate::OscEvent::SetTitleAndTab
pub fn handle_title_and_icon(performer: &mut AnsiToOfsBufPerformer, title: &str) {
    performer.ofs_buf_vt_100.handle_title_and_icon(title);
}

/// Handle `OSC 8` hyperlink sequences.
/// Creates hyperlinks with URI for later processing.
/// The display text is handled separately via `print()` calls.
/// Queues Hyperlink event for later processing by output renderer.
pub fn handle_hyperlink(performer: &mut AnsiToOfsBufPerformer, uri: &str) {
    performer.ofs_buf_vt_100.handle_hyperlink(uri);
}
