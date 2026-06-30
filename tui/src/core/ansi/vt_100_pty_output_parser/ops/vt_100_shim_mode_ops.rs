// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words URXVT

//! Mode setting operations (SM/RM).
//!
//! This module acts as a thin shim layer that delegates to the actual implementation.
//! Refer to the module-level documentation in the ops module for details on the
//! "shim → impl → test" architecture and naming conventions.
//!
//! **Related Files:**
//! - **Implementation**: [`vt_100_impl_mode_ops`] - Business logic with unit tests
//! - **Integration Tests**: [`test_mode_ops`] - Full pipeline testing via public API
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
//! Application sends `ESC [?7h` (set autowrap mode)
//!         ↓
//!     PTY Controlled (escape sequence)
//!         ↓
//!     PTY Controller (byte stream) <- in process_manager.rs
//!         ↓
//!     VTE Parser (parses ESC [...char pattern)
//!         ↓
//!     csi_dispatch() [routes to modules below]
//!         ↓
//!     Route to ops module:
//!       - cursor_ops:: for movement (A,B,C,D,H)
//!       - scroll_ops:: for scrolling (S,T)
//!       - sgr_ops:: for styling (m)
//!       - line_ops:: for lines (L,M)
//!       - char_ops:: for chars (@,P,X)                         ╭───────────╮
//!       - mode_ops:: for modes (h,l)                        <- │THIS MODULE│
//!         ↓                                                    ╰───────────╯
//!     Update OffscreenBuffer state
//! ```
//!
//! [`CSI`]: crate::CsiSequence
//! [`test_mode_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_mode_ops
//! [`vt_100_impl_mode_ops`]: crate::core::ansi::vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_mode_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: crate::core::ansi::vt_100_pty_output_parser::ops

use super::super::{PrivateModeType, ansi_parser_public_api::AnsiToOfsBufPerformer,
                   modes::terminal_mode_state_todo};
use crate::{AutoWrapMode, CursorKeyMode, CursorVisibilityMode,
             DEBUG_TUI_VT100_PARSER, RequestedScreenMode, URXVT_MOUSE_EXTENSION,
             UTF8_MOUSE_EXTENSION, MouseTrackingMode,
             core::ansi::constants::CSI_PRIVATE_MODE_PREFIX};
use vte::Params;

/// Handle Set Mode (`CSI h`) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn set_mode(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
    intermediates: &[u8],
) {
    let is_private_mode = intermediates.contains(&(CSI_PRIVATE_MODE_PREFIX as u8));
    if is_private_mode {
        let mode = PrivateModeType::from(params);
        match mode {
            PrivateModeType::AutoWrap => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_auto_wrap_mode(AutoWrapMode::Enabled);
            }
            PrivateModeType::AlternateScreenBuffer => {
                performer
                    .ofs_buf_vt_100
                    .set_alt_screen_mode(RequestedScreenMode::Alternate);
            }
            PrivateModeType::ShowCursor => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_cursor_visibility_mode(CursorVisibilityMode::Visible);
            }
            PrivateModeType::X11MouseTracking
            | PrivateModeType::CellMotionMouseTracking
            | PrivateModeType::ApplicationMouseTracking => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_mouse_tracking_mode(MouseTrackingMode::Enabled);
            }
            PrivateModeType::SgrMouseMode => {
                // We always use SGR formatting internally, so we don't need to do
                // anything here.
            }
            PrivateModeType::BracketedPaste => {
                performer
                    .ofs_buf_vt_100
                    .terminal_mode
                    .bracketed_paste =
                    terminal_mode_state_todo::BracketedPasteMode::Enabled;
            }
            // Safely suppress/ignore other modern TUI extensions.
            // Downgrading to debug prevents heavy log spam from interactive
            // TUIs (like hx/gitui).
            PrivateModeType::Other(
                UTF8_MOUSE_EXTENSION | URXVT_MOUSE_EXTENSION,
            ) => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::debug!(
                        "CSI ?{}h: Suppressed/shimmed private mode",
                        mode.as_u16()
                    );
                });
            }
            _ => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::warn!("CSI ?{}h: Unhandled private mode", mode.as_u16());
                });
            }
        }
    } else {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("CSI h: Standard mode setting not implemented");
        });
    }
}

/// Handle Reset Mode (`CSI l`) command.
/// Supports both standard modes and private modes (with ? prefix).
pub fn reset_mode(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
    intermediates: &[u8],
) {
    let is_private_mode = intermediates.contains(&(CSI_PRIVATE_MODE_PREFIX as u8));
    if is_private_mode {
        let mode = PrivateModeType::from(params);
        match mode {
            PrivateModeType::AutoWrap => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_auto_wrap_mode(AutoWrapMode::Disabled);
            }
            PrivateModeType::AlternateScreenBuffer => {
                performer
                    .ofs_buf_vt_100
                    .set_alt_screen_mode(RequestedScreenMode::Primary);
            }
            PrivateModeType::ShowCursor => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_cursor_visibility_mode(CursorVisibilityMode::Hidden);
            }
            PrivateModeType::X11MouseTracking
            | PrivateModeType::CellMotionMouseTracking
            | PrivateModeType::ApplicationMouseTracking => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_mouse_tracking_mode(MouseTrackingMode::Disabled);
            }
            PrivateModeType::SgrMouseMode => {
                // We always use SGR formatting internally, so we don't need to do anything here.
            }
            PrivateModeType::BracketedPaste => {
                performer
                    .ofs_buf_vt_100
                    .terminal_mode
                    .bracketed_paste =
                    terminal_mode_state_todo::BracketedPasteMode::Disabled;
            }
            // Safely suppress/ignore other modern TUI extensions.
            // Downgrading to debug prevents heavy log spam from interactive
            // TUIs (like hx/gitui).
            PrivateModeType::Other(
                UTF8_MOUSE_EXTENSION | URXVT_MOUSE_EXTENSION,
            ) => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::debug!(
                        "CSI ?{}l: Suppressed/shimmed private mode",
                        mode.as_u16()
                    );
                });
            }
            _ => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::warn!("CSI ?{}l: Unhandled private mode", mode.as_u16());
                });
            }
        }
    } else {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("CSI l: Standard mode reset not implemented");
        });
    }
}
