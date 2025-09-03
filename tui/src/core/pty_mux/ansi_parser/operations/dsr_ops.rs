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
    let dsr_type = params.extract_nth_opt(0).unwrap_or(0).into();
    match dsr_type {
        DsrRequestType::RequestStatus => {
            // Status report request - respond with ESC[0n (terminal OK)
            processor
                .ofs_buf
                .ansi_parser_support
                .pending_dsr_responses
                .push(DsrRequestFromPtyEvent::TerminalStatus);
        }
        DsrRequestType::RequestCursorPosition => {
            // Cursor position report - respond with ESC[row;colR
            // Convert 0-based internal position to 1-based terminal position
            let row = processor.ofs_buf.my_pos.row_index.into();
            let col = processor.ofs_buf.my_pos.col_index.into();
            processor
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
