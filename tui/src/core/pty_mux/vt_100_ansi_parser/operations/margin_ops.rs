// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Margin setting operations (DECSTBM).
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
