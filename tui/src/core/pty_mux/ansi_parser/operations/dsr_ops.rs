// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) operations.
//!
//! # CSI Sequence Architecture
//!
//! ```text
//! Application sends "ESC[6n" (request cursor position)
//!         ↓
//!     PTY Slave (escape sequence)
//!         ↓
//!     PTY Master (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC[...char pattern)
//!         ↓
//!     csi_dispatch() [THIS METHOD]
//!         ↓
//!     Route to operations module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)
//!       - dsr_ops:: for device status (n)
//!         ↓
//!     Update OffscreenBuffer state
//! ```

use super::super::{ansi_parser_public_api::AnsiToOfsBufPerformer,
                   protocols::dsr_codes::DsrRequestType};
use crate::DsrRequestFromPtyEvent;

/// Handle Device Status Report (CSI n) command.
///
/// This command is used by applications to query the terminal's status.
/// Generates DSR response events that will be processed by the process manager
/// and sent back to the child process through the PTY input channel.
pub fn status_report(performer: &mut AnsiToOfsBufPerformer, params: &vte::Params) {
    match DsrRequestType::from(params) {
        DsrRequestType::RequestStatus => {
            // Status report request - respond with ESC[0n (terminal OK).
            performer
                .ofs_buf
                .ansi_parser_support
                .pending_dsr_responses
                .push(DsrRequestFromPtyEvent::TerminalStatus);
        }
        DsrRequestType::RequestCursorPosition => {
            // Cursor position report - respond with ESC[row;colR.
            // Convert 0-based internal position to 1-based terminal position.
            let row = performer.ofs_buf.my_pos.row_index.into();
            let col = performer.ofs_buf.my_pos.col_index.into();
            performer
                .ofs_buf
                .ansi_parser_support
                .pending_dsr_responses
                .push(DsrRequestFromPtyEvent::CursorPosition { row, col });
        }
        DsrRequestType::Other(n) => {
            tracing::warn!("CSI {}n (DSR): Unsupported device status report request", n);
        }
    }
}
