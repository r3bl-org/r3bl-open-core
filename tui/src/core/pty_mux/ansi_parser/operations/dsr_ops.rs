// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device Status Report (DSR) operations.

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::dsr_codes::DsrRequestType};
use crate::{DsrRequestFromPtyEvent, core::pty_mux::ansi_parser::param_utils::ParamsExt};

/// Handle Device Status Report (CSI n) command.
///
/// This command is used by applications to query the terminal's status.
/// Generates DSR response events that will be processed by the process manager
/// and sent back to the child process through the PTY input channel.
pub fn status_report(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = params.extract_nth_opt(0).unwrap_or(0);
    let dsr_type = DsrRequestType::from(n);

    match dsr_type {
        DsrRequestType::RequestStatus => {
            // Status report request - respond with ESC[0n (terminal OK)
            tracing::debug!(
                "CSI 5n (DSR): Status report requested - generating response"
            );
            processor
                .ofs_buf
                .ansi_parser_support
                .pending_dsr_responses
                .push(DsrRequestFromPtyEvent::TerminalStatus);
        }
        DsrRequestType::RequestCursorPosition => {
            // Cursor position report - respond with ESC[row;colR
            // Convert 0-based internal position to 1-based terminal position
            let row = processor.ofs_buf.my_pos.row_index.as_u16() + 1;
            let col = processor.ofs_buf.my_pos.col_index.as_u16() + 1;

            tracing::debug!(
                "CSI 6n (DSR): Cursor position report requested at {:?} - generating response with row={}, col={}",
                processor.ofs_buf.my_pos,
                row,
                col
            );

            processor
                .ofs_buf
                .ansi_parser_support
                .pending_dsr_responses
                .push(DsrRequestFromPtyEvent::CursorPosition { row, col });
        }
        DsrRequestType::Other(n) => {
            tracing::debug!(
                "CSI {}n (DSR): Unknown device status report - no response generated",
                n
            );
        }
    }
}
