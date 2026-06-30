// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! [`DEC`] Private Mode types for the following [`CSI`] sequences:
//! 1. `CSI ? <param> h` (set) and
//! 2. `CSI ? <param> l` (reset) sequences.
//!
//! This module handles the various private mode settings that control terminal behavior,
//! such as cursor visibility, auto-wrap, and alternate screen buffer.
//!
//! [`CSI`]: crate::CsiSequence
//! [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation

use crate::{ParamsExt,
            core::ansi::constants::{ALT_SCREEN_BUFFER, APPLICATION_MOUSE_TRACKING,
                                    BRACKETED_PASTE_MODE, CELL_MOTION_MOUSE_TRACKING,
                                    DECANM_VT52_MODE, DECAWM_AUTO_WRAP, DECCKM_CURSOR_KEYS,
                                    DECCOLM_132_COLUMN, DECOM_ORIGIN_MODE,
                                    DECSCLM_SMOOTH_SCROLL, DECSCNM_REVERSE_VIDEO,
                                    DECTCEM_SHOW_CURSOR, SAVE_CURSOR_DEC,
                                    SGR_MOUSE_MODE, X11_MOUSE_TRACKING}};

/// [`DEC`] Private Mode types for the following [`CSI`] sequences:
/// 1. `CSI ? <param> h` (set) and
/// 2. `CSI ? <param> l` (reset) sequences.
///
/// [`CSI`]: crate::CsiSequence
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PrivateModeType {
    /// DECCKM - Application Cursor Keys (1)
    CursorKeys,

    /// DECANM - VT52 Mode (2)
    Vt52Mode,

    /// DECCOLM - 132 Column Mode (3)
    Column132,

    /// DECSCLM - Smooth Scroll (4)
    SmoothScroll,

    /// DECSCNM - Reverse Video (5)
    ReverseVideo,

    /// DECOM - Origin Mode (6)
    OriginMode,

    /// DECAWM - Auto Wrap Mode (7)
    AutoWrap,

    /// DECTCEM - Show/Hide Cursor (25)
    ShowCursor,

    /// Save Cursor (1048)
    SaveCursorDec,

    /// Use Alternate Screen Buffer (1049)
    AlternateScreenBuffer,

    /// X11 Mouse Tracking (1000).
    /// Only sends mouse clicks and scroll wheel events to the app.
    ///
    /// **Note:** See [`MouseTrackingMode`] for how our engine handles this.
    ///
    /// [`MouseTrackingMode`]: crate::MouseTrackingMode
    X11MouseTracking,

    /// Cell Motion Mouse Tracking (1002).
    /// Sends clicks and drag events (when the mouse moves while a button is held down).
    ///
    /// **Note:** See [`MouseTrackingMode`] for how our engine handles this.
    ///
    /// [`MouseTrackingMode`]: crate::MouseTrackingMode
    CellMotionMouseTracking,

    /// Application Mouse Tracking (1003).
    /// Sends everything—clicks, drags, and all raw mouse movement (hovering).
    ///
    /// **Note:** See [`MouseTrackingMode`] for how our engine handles this.
    ///
    /// [`MouseTrackingMode`]: crate::MouseTrackingMode
    ApplicationMouseTracking,

    /// [`SGR`] Mouse Mode (1006).
    ///
    /// This is a modifier. By default, legacy mouse reporting breaks if your terminal is
    /// wider than 223 columns. [`SGR`] mode tells the terminal to format the coordinates
    /// as plain text (e.g., `CSI < button ; x ; y M`) so it can support infinitely large
    /// screens.
    ///
    /// Usually, modern TUI apps will request both a tracking mode (like
    /// [`CellMotionMouseTracking`] `1002` or [`ApplicationMouseTracking`] `1003` to say
    /// *when* to get events) and `SgrMouseMode` `1006` (to say *how* to format those
    /// events).
    ///
    /// **Note:** See [`MouseTrackingMode`] for how our engine handles this.
    ///
    /// [`ApplicationMouseTracking`]: Self::ApplicationMouseTracking
    /// [`CellMotionMouseTracking`]: Self::CellMotionMouseTracking
    /// [`MouseTrackingMode`]: crate::MouseTrackingMode
    /// [`SGR`]: crate::SgrCode
    SgrMouseMode,

    /// Bracketed Paste Mode (2004)
    BracketedPaste,

    /// Unknown/unsupported private mode
    Other(u16),
}

impl PrivateModeType {
    #[must_use]
    pub fn as_u16(&self) -> u16 {
        match self {
            Self::CursorKeys => DECCKM_CURSOR_KEYS,
            Self::Vt52Mode => DECANM_VT52_MODE,
            Self::Column132 => DECCOLM_132_COLUMN,
            Self::SmoothScroll => DECSCLM_SMOOTH_SCROLL,
            Self::ReverseVideo => DECSCNM_REVERSE_VIDEO,
            Self::OriginMode => DECOM_ORIGIN_MODE,
            Self::AutoWrap => DECAWM_AUTO_WRAP,
            Self::ShowCursor => DECTCEM_SHOW_CURSOR,
            Self::SaveCursorDec => SAVE_CURSOR_DEC,
            Self::AlternateScreenBuffer => ALT_SCREEN_BUFFER,
            Self::X11MouseTracking => X11_MOUSE_TRACKING,
            Self::CellMotionMouseTracking => CELL_MOTION_MOUSE_TRACKING,
            Self::ApplicationMouseTracking => APPLICATION_MOUSE_TRACKING,
            Self::SgrMouseMode => SGR_MOUSE_MODE,
            Self::BracketedPaste => BRACKETED_PASTE_MODE,

            Self::Other(n) => *n,
        }
    }
}

impl From<u16> for PrivateModeType {
    fn from(value: u16) -> Self {
        match value {
            DECCKM_CURSOR_KEYS => Self::CursorKeys,
            DECANM_VT52_MODE => Self::Vt52Mode,
            DECCOLM_132_COLUMN => Self::Column132,
            DECSCLM_SMOOTH_SCROLL => Self::SmoothScroll,
            DECSCNM_REVERSE_VIDEO => Self::ReverseVideo,
            DECOM_ORIGIN_MODE => Self::OriginMode,
            DECAWM_AUTO_WRAP => Self::AutoWrap,
            DECTCEM_SHOW_CURSOR => Self::ShowCursor,
            SAVE_CURSOR_DEC => Self::SaveCursorDec,
            ALT_SCREEN_BUFFER => Self::AlternateScreenBuffer,
            X11_MOUSE_TRACKING => Self::X11MouseTracking,
            CELL_MOTION_MOUSE_TRACKING => Self::CellMotionMouseTracking,
            APPLICATION_MOUSE_TRACKING => Self::ApplicationMouseTracking,
            SGR_MOUSE_MODE => Self::SgrMouseMode,
            BRACKETED_PASTE_MODE => Self::BracketedPaste,

            n => Self::Other(n),
        }
    }
}

impl From<&vte::Params> for PrivateModeType {
    fn from(params: &vte::Params) -> Self {
        let mode_num = params.extract_nth_single_opt_raw(0).unwrap_or(0);
        mode_num.into()
    }
}
