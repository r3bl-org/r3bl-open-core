// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations (SM/RM).

use vte::Params;

use super::super::{ansi_parser_public_api::AnsiToBufferProcessor,
                   protocols::csi_codes::PrivateModeType};
use crate::core::pty_mux::ansi_parser::param_utils::ParamsExt;

/// Handle Set Mode (CSI h) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn set_mode(
    processor: &mut AnsiToBufferProcessor,
    params: &Params,
    intermediates: &[u8],
) {
    let is_private_mode = intermediates.contains(&b'?');
    if is_private_mode {
        let mode_num = params.extract_nth_opt(0).unwrap_or(0);
        let mode = PrivateModeType::from(mode_num);
        match mode {
            PrivateModeType::AutoWrap => {
                processor.ofs_buf.ansi_parser_support.auto_wrap_mode = true;
            }
            _ => {
                tracing::warn!("CSI ?{}h: Unhandled private mode", mode.as_u16());
            }
        }
    } else {
        tracing::warn!("CSI h: Standard mode setting not implemented");
    }
}

/// Handle Reset Mode (CSI l) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn reset_mode(
    processor: &mut AnsiToBufferProcessor,
    params: &Params,
    intermediates: &[u8],
) {
    let is_private_mode = intermediates.contains(&b'?');
    if is_private_mode {
        let mode_num = params.extract_nth_opt(0).unwrap_or(0);
        let mode = PrivateModeType::from(mode_num);
        match mode {
            PrivateModeType::AutoWrap => {
                processor.ofs_buf.ansi_parser_support.auto_wrap_mode = false;
            }
            _ => {
                tracing::warn!("CSI ?{}l: Unhandled private mode", mode.as_u16());
            }
        }
    } else {
        tracing::warn!("CSI l: Standard mode reset not implemented");
    }
}
