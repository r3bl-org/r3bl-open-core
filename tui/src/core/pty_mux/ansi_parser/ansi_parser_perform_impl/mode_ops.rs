// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Mode setting operations (SM/RM).

use vte::Params;

use crate::ansi_parser_perform_impl::param_utils::extract_nth_optional_param;

use super::super::super::{ansi_parser_public_api::AnsiToBufferProcessor, csi_codes::PrivateModeType};

/// Handle Set Mode (CSI h) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn set_mode(processor: &mut AnsiToBufferProcessor, params: &Params, intermediates: &[u8]) {
    let is_private_mode = intermediates.contains(&b'?');
    if is_private_mode {
        let mode_num = extract_nth_optional_param(params, 0).unwrap_or(0);
        let mode = PrivateModeType::from(mode_num);
        match mode {
            PrivateModeType::AutoWrap => {
                processor.ofs_buf.ansi_parser_support.auto_wrap_mode = true;
                tracing::trace!("ESC[?7h: Enabled auto-wrap mode (DECAWM)");
            }
            _ => tracing::debug!(
                "CSI ?{}h: Unhandled private mode",
                mode.as_u16()
            ),
        }
    } else {
        tracing::debug!("CSI h: Standard mode setting not implemented");
    }
}

/// Handle Reset Mode (CSI l) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn reset_mode(processor: &mut AnsiToBufferProcessor, params: &Params, intermediates: &[u8]) {
    let is_private_mode = intermediates.contains(&b'?');
    if is_private_mode {
        let mode_num = extract_nth_optional_param(params, 0).unwrap_or(0);
        let mode = PrivateModeType::from(mode_num);
        match mode {
            PrivateModeType::AutoWrap => {
                processor.ofs_buf.ansi_parser_support.auto_wrap_mode = false;
                tracing::trace!("ESC[?7l: Disabled auto-wrap mode (DECAWM)");
            }
            _ => tracing::debug!(
                "CSI ?{}l: Unhandled private mode",
                mode.as_u16()
            ),
        }
    } else {
        tracing::debug!("CSI l: Standard mode reset not implemented");
    }
}