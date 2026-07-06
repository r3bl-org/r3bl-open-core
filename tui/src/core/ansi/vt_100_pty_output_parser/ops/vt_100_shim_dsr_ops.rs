// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report ([`DSR`]) operations.
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_dsr_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_dsr_ops`] - Full pipeline testing via public API
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
//! # [`CSI`] Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC [6n" (request cursor position)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses `ESC [`...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to ops module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)                         ╭───────────╮
//!       - dsr_ops:: for device status (n)                   <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Update OfsBuf state
//! ```
//!
//! [`CSI`]: crate::CsiSequence
//! [`DSR`]: crate::DsrSequence
//! [`test_dsr_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_dsr_ops
//! [`vt_100_impl_dsr_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_dsr_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use crate::{DEBUG_TUI_VT100_PARSER,
            core::ansi::{generator::DsrRequestType,
                         vt_100_pty_output_parser::ansi_parser_public_api::AnsiToOfsBufPerformer}};

/// Handle Device Status Report (`CSI n`) command.
///
/// This command is used by applications to query the terminal's status.
/// Generates [`DSR`] response events that will be processed by the process manager
/// and sent back to the child process through the [`PTY`] input channel.
///
/// [`DSR`]: crate::DsrSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn status_report(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    match DsrRequestType::from(params) {
        DsrRequestType::RequestStatus => {
            performer.ofs_buf_vt_100.handle_status_report_request();
        }
        DsrRequestType::RequestCursorPosition => {
            performer.ofs_buf_vt_100.handle_cursor_position_request();
        }
        DsrRequestType::Other(n) => {
            DEBUG_TUI_VT100_PARSER.then(|| {
                tracing::warn!(
                    "CSI {}n (DSR): Unsupported device status report request",
                    n
                );
            });
        }
    }
}
