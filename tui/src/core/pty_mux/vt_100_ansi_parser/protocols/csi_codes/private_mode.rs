// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! DEC Private Mode types for CSI ? h/l sequences.
//!
//! This module handles the various private mode settings that control terminal behavior,
//! such as cursor visibility, auto-wrap, and alternate screen buffer.

use super::constants::*;

/// DEC Private Mode types for CSI ? h/l sequences
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
            n => Self::Other(n),
        }
    }
}

impl From<&vte::Params> for PrivateModeType {
    fn from(params: &vte::Params) -> Self {
        use super::params::ParamsExt;
        let mode_num = params.extract_nth_opt(0).unwrap_or(0);
        mode_num.into()
    }
}
