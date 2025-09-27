// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Margin setting operations (DECSTBM).
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! See the [module-level documentation](super::super) for details on the "shim → impl →
//! test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`impl_margin_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_margin_ops`] - Full pipeline testing via public API
//!
//! [`impl_margin_ops`]: crate::tui::terminal_lib_backends::offscreen_buffer::vt_100_ansi_impl::impl_margin_ops
//! [`test_margin_ops`]: crate::core::pty_mux::vt_100_ansi_parser::vt_100_ansi_conformance_tests::tests::test_margin_ops
//!
//! # Architecture Overview
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
//! # CSI Sequence Processing Flow
//!
//! ```text
//! Application sends "ESC[1;20r" (set top/bottom margins)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)    ╭───────────╮
//!       - margin_ops:: for margins (r) <- │THIS MODULE│
//!         ↓                               ╰───────────╯
//!     Update OffscreenBuffer state
//! ```

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::csi_codes::MarginRequest};

/// Handle Set Top and Bottom Margins (DECSTBM) command.
/// CSI r - ESC [ top ; bottom r
///
/// This command sets the scrolling region for the terminal. Lines outside
/// the scrolling region are not affected by scroll operations.
pub fn set_margins(performer: &mut AnsiToOfsBufPerformer, params: &Params) {
    let request = MarginRequest::from(params);

    match request {
        MarginRequest::Reset => {
            performer.ofs_buf.reset_scroll_margins();
        }
        MarginRequest::SetRegion { top, bottom } => {
            performer.ofs_buf.set_scroll_margins(top, bottom);
        }
    }
}
