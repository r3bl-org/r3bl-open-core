// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

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

use super::super::{PrivateModeType, ansi_parser_public_api::AnsiToOfsBufPerformer};
use crate::{APPLICATION_MOUSE_TRACKING, AutoWrapState, BRACKETED_PASTE_MODE,
            CELL_MOTION_MOUSE_TRACKING, CursorVisibilityState, DEBUG_TUI_VT100_PARSER,
            RequestedScreenMode, SGR_MOUSE_MODE, URXVT_MOUSE_EXTENSION,
            UTF8_MOUSE_EXTENSION, X11_MOUSE_TRACKING,
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
                    .set_requested_auto_wrap_mode(AutoWrapState::Enabled);
            }
            PrivateModeType::AlternateScreenBuffer => {
                performer
                    .ofs_buf_vt_100
                    .set_alt_screen_mode(RequestedScreenMode::Alternate);
            }
            PrivateModeType::ShowCursor => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_cursor_visibility_mode(CursorVisibilityState::Visible);
            }
            PrivateModeType::FocusEvents => {
                performer.ofs_buf_vt_100.set_focus_events_mode(true);
            }
            // Safely suppress/ignore modern TUI extensions (like mouse tracking and
            // bracketed paste). Currently, the multiplexer does not support
            // routing rich input events back into the PTY. Downgrading to
            // debug prevents heavy log spam from interactive TUIs (like hx/gitui).
            PrivateModeType::Other(
                X11_MOUSE_TRACKING
                | CELL_MOTION_MOUSE_TRACKING
                | APPLICATION_MOUSE_TRACKING
                | UTF8_MOUSE_EXTENSION
                | SGR_MOUSE_MODE
                | URXVT_MOUSE_EXTENSION
                | BRACKETED_PASTE_MODE,
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
                    .set_requested_auto_wrap_mode(AutoWrapState::Disabled);
            }
            PrivateModeType::AlternateScreenBuffer => {
                performer
                    .ofs_buf_vt_100
                    .set_alt_screen_mode(RequestedScreenMode::Primary);
            }
            PrivateModeType::ShowCursor => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_cursor_visibility_mode(CursorVisibilityState::Hidden);
            }
            PrivateModeType::FocusEvents => {
                performer.ofs_buf_vt_100.set_focus_events_mode(false);
            }
            // Safely suppress/ignore modern TUI extensions (like mouse tracking and
            // bracketed paste). Currently, the multiplexer does not support
            // routing rich input events back into the PTY. Downgrading to
            // debug prevents heavy log spam from interactive TUIs (like hx/gitui).
            PrivateModeType::Other(
                X11_MOUSE_TRACKING
                | CELL_MOTION_MOUSE_TRACKING
                | APPLICATION_MOUSE_TRACKING
                | UTF8_MOUSE_EXTENSION
                | SGR_MOUSE_MODE
                | URXVT_MOUSE_EXTENSION
                | BRACKETED_PASTE_MODE,
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
