// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words URXVT

//! Mode setting operations ([`SM`] / [`RM`]).
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
//!     Update OfsBuf state
//! ```
//!
//! [`CSI`]: crate::CsiSequence
//! [`RM`]: reset_mode
//! [`SM`]: set_mode
//! [`test_mode_ops`]: crate::vt_100_pty_output_conformance_tests::tests::vt_100_test_mode_ops
//! [`vt_100_impl_mode_ops`]: vt_100_pty_output_parser::ops_impl_ofs_buf::vt_100_impl_mode_ops
//! [module-level Architecture Overview]: super#architecture-overview
//! [module-level documentation]: self
//! [ops module]: vt_100_pty_output_parser::ops

use super::super::{PrivateModeType, ansi_parser_public_api::AnsiToOfsBufPerformer};
#[allow(unused_imports, reason = "Allows flat link ref defs in rustdocs")]
use crate::core::ansi::vt_100_pty_output_parser;
use crate::{AutoWrapMode, BRACKETED_PASTE_MODE, CursorKeyMode, CursorVisibilityMode,
            DEBUG_TUI_VT100_PARSER, MouseTrackingFormat, MouseTrackingMode,
            RequestedScreenMode, URXVT_MOUSE_EXTENSION, UTF8_MOUSE_EXTENSION,
            core::ansi::constants::CSI_PRIVATE_MODE_PREFIX};
use vte::Params;

/// Handles the Set Mode (`CSI h`) escape sequence.
///
/// This method processes both standard [`ANSI`] modes and [`DEC`] private modes (which
/// are indicated by the `?` prefix in the `intermediates` slice).
///
/// A single [`ANSI`] escape sequence can configure multiple modes at once. For example,
/// `htop` sends `ESC [ ? 1006 ; 1000 h` to enable both [`SGR`] and `X11` mouse tracking
/// simultaneously. This function iterates over all parameters in the sequence to ensure
/// every requested mode is applied.
///
/// # Arguments
///
/// - `performer`: The [`AnsiToOfsBufPerformer`] that maintains the terminal state.
/// - `params`: The [`Params`] parsed from the sequence, which may contain multiple mode
///   numbers.
/// - `intermediates`: The intermediate bytes of the sequence. If this contains `?`
///   ([`CSI_PRIVATE_MODE_PREFIX`]), the sequence is treated as a [`DEC`] private mode.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]:
///     crate::core::ansi::vt_100_pty_output_parser::AnsiToOfsBufPerformer
/// [`CSI_PRIVATE_MODE_PREFIX`]: crate::core::ansi::constants::CSI_PRIVATE_MODE_PREFIX
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`Params`]: vte::Params
/// [`SGR`]: crate::SgrCode
pub fn set_mode(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
    intermediates: &[u8],
) {
    let Some(modes) = parse_private_modes(params, intermediates) else {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("CSI h: Standard mode setting not implemented");
        });
        return;
    };

    for mode in modes {
        match mode {
            PrivateModeType::CursorKeys => {
                performer.ofs_buf_vt_100.terminal_mode.cursor_key_mode =
                    CursorKeyMode::Application;
            }
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
                    .set_requested_cursor_visibility_mode(
                        CursorVisibilityMode::Visible,
                    );
            }
            PrivateModeType::X11MouseTracking
            | PrivateModeType::CellMotionMouseTracking
            | PrivateModeType::ApplicationMouseTracking => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_mouse_tracking_mode(MouseTrackingMode::Enabled);
            }
            PrivateModeType::SgrMouseMode => {
                performer
                    .ofs_buf_vt_100
                    .set_mouse_tracking_format(MouseTrackingFormat::Sgr);
            }
            // Safely suppress/ignore other modern TUI extensions (like bracketed
            // paste). Currently, the multiplexer does not support
            // routing rich input events back into the PTY.
            PrivateModeType::Other(
                UTF8_MOUSE_EXTENSION | URXVT_MOUSE_EXTENSION | BRACKETED_PASTE_MODE,
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
                    tracing::debug!(
                        "CSI ?{}h: Unimplemented/ignored private mode set",
                        mode.as_u16()
                    );
                });
            }
        }
    }
}

/// Handles the Reset Mode (`CSI l`) escape sequence.
///
/// This method processes both standard [`ANSI`] modes and [`DEC`] private modes (which
/// are indicated by the `?` prefix in the `intermediates` slice).
///
/// A single [`ANSI`] escape sequence can configure multiple modes at once. For example,
/// `htop` sends `ESC [ ? 1006 ; 1000 l` to disable both [`SGR`] and `X11` mouse tracking
/// simultaneously. This function iterates over all parameters in the sequence to ensure
/// every requested mode is applied.
///
/// # Arguments
///
/// - `performer`: The [`AnsiToOfsBufPerformer`] that maintains the terminal state.
/// - `params`: The [`Params`] parsed from the sequence, which may contain multiple mode
///   numbers.
/// - `intermediates`: The intermediate bytes of the sequence. If this contains `?`
///   ([`CSI_PRIVATE_MODE_PREFIX`]), the sequence is treated as a [`DEC`] private mode.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`AnsiToOfsBufPerformer`]: crate::core::ansi::vt_100_pty_output_parser::AnsiToOfsBufPerformer
/// [`CSI_PRIVATE_MODE_PREFIX`]: crate::core::ansi::constants::CSI_PRIVATE_MODE_PREFIX
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`Params`]: vte::Params
/// [`SGR`]: crate::SgrCode
pub fn reset_mode(
    performer: &mut AnsiToOfsBufPerformer,
    params: &Params,
    intermediates: &[u8],
) {
    let Some(modes) = parse_private_modes(params, intermediates) else {
        DEBUG_TUI_VT100_PARSER.then(|| {
            tracing::warn!("CSI l: Standard mode reset not implemented");
        });
        return;
    };

    for mode in modes {
        match mode {
            PrivateModeType::CursorKeys => {
                performer.ofs_buf_vt_100.terminal_mode.cursor_key_mode =
                    CursorKeyMode::Normal;
            }
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
                    .set_requested_cursor_visibility_mode(
                        CursorVisibilityMode::Hidden,
                    );
            }
            PrivateModeType::X11MouseTracking
            | PrivateModeType::CellMotionMouseTracking
            | PrivateModeType::ApplicationMouseTracking => {
                performer
                    .ofs_buf_vt_100
                    .set_requested_mouse_tracking_mode(MouseTrackingMode::Disabled);
            }
            PrivateModeType::SgrMouseMode => {
                performer
                    .ofs_buf_vt_100
                    .set_mouse_tracking_format(MouseTrackingFormat::X10);
            }
            // Safely suppress/ignore other modern TUI extensions (like bracketed
            // paste). Currently, the multiplexer does not support
            // routing rich input events back into the PTY.
            PrivateModeType::Other(
                UTF8_MOUSE_EXTENSION | URXVT_MOUSE_EXTENSION | BRACKETED_PASTE_MODE,
            ) => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::debug!(
                        "CSI ?{}l: Suppressed/shimmed private mode reset",
                        mode.as_u16()
                    );
                });
            }
            _ => {
                DEBUG_TUI_VT100_PARSER.then(|| {
                    tracing::debug!(
                        "CSI ?{}l: Unimplemented/ignored private mode reset",
                        mode.as_u16()
                    );
                });
            }
        }
    }
}

/// Extracts [`DEC`] private modes from a parsed [`CSI`] sequence.
///
/// A single [`CSI`] sequence can configure multiple modes at once. For example, `htop`
/// sends `ESC [ ? 1006 ; 1000 h` to enable both [`SGR`] and `X11` mouse tracking. This
/// iterator processes all parameters in the sequence.
///
/// Returns an iterator over the parsed [`PrivateModeType`]s if the sequence is a
/// private mode (indicated by `?`), or `None` if it is a standard [`ANSI`] mode.
///
/// # Arguments
///
/// - `params`: The parameters parsed from the sequence.
/// - `intermediates`: The intermediate bytes of the sequence.
///
/// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`SGR`]: crate::SgrCode
fn parse_private_modes<'a>(
    params: &'a Params,
    intermediates: &[u8],
) -> Option<impl Iterator<Item = PrivateModeType> + 'a> {
    let is_private_mode = intermediates.contains(&(CSI_PRIVATE_MODE_PREFIX as u8));
    if is_private_mode {
        Some(params.iter().map(|sub_params| {
            let mode_num = sub_params.first().copied().unwrap_or(0);
            PrivateModeType::from(mode_num)
        }))
    } else {
        None
    }
}
