// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Device status report operations (DSR).

use vte::Params;

use super::super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                           csi_codes::DeviceStatusReportType};
use crate::ansi_parser_perform_impl::param_utils::extract_nth_optional_param;

/// Handle Device Status Report (CSI n) command.
///
/// This command is used by applications to query the terminal's status.
/// In a full terminal implementation, this would send responses back through the PTY,
/// but for now we just log the requests as our current architecture doesn't support
/// sending responses.
pub fn device_status_report(processor: &mut AnsiToBufferProcessor, params: &Params) {
    let n = extract_nth_optional_param(params, 0).unwrap_or(0);
    let dsr_type = DeviceStatusReportType::from(n);

    match dsr_type {
        DeviceStatusReportType::RequestStatus => {
            // Status report request - should respond with ESC[0n (OK)
            tracing::debug!(
                "CSI 5n (DSR): Status report requested (response needed but not implemented)"
            );
        }
        DeviceStatusReportType::RequestCursorPosition => {
            // Cursor position report - should respond with ESC[row;colR
            tracing::debug!(
                "CSI 6n (DSR): Cursor position report requested at {:?} (response needed but not implemented)",
                processor.ofs_buf.my_pos
            );
        }
        DeviceStatusReportType::Other(n) => {
            tracing::debug!("CSI {}n (DSR): Unknown device status report", n);
        }
    }
}
