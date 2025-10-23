// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! CSI sequence builder for terminal control operations.
//!
//! This module provides the `CsiSequence` enum which represents various CSI control
//! sequences and can serialize them into ANSI escape codes.

use super::{super::dsr_codes::DsrRequestType,
            constants::{CHA_CURSOR_COLUMN, CNL_CURSOR_NEXT_LINE, CPL_CURSOR_PREV_LINE,
                        CSI_PARAM_SEPARATOR, CSI_PRIVATE_MODE_PREFIX, CSI_START,
                        CUB_CURSOR_BACKWARD, CUD_CURSOR_DOWN, CUF_CURSOR_FORWARD,
                        CUP_CURSOR_POSITION, CUU_CURSOR_UP, DCH_DELETE_CHAR,
                        DECSTBM_SET_MARGINS, DL_DELETE_LINE, DSR_DEVICE_STATUS,
                        ECH_ERASE_CHAR, ED_ERASE_DISPLAY, EL_ERASE_LINE,
                        HVP_CURSOR_POSITION, ICH_INSERT_CHAR, IL_INSERT_LINE,
                        RCP_RESTORE_CURSOR, RM_RESET_PRIVATE_MODE, SCP_SAVE_CURSOR,
                        SD_SCROLL_DOWN, SM_SET_PRIVATE_MODE, SU_SCROLL_UP,
                        VPA_VERTICAL_POSITION},
            private_mode::PrivateModeType};
use crate::{BufTextStorage, FastStringify, TermCol, TermRow,
            generate_impl_display_for_fast_stringify};
use std::fmt::{Formatter, Result};

/// Builder for CSI (Control Sequence Introducer) sequences.
/// Similar to `SgrCode` but for cursor movement and other CSI commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CsiSequence {
    /// Cursor Up (CUU) - ESC [ n A
    CursorUp(u16),
    /// Cursor Down (CUD) - ESC [ n B
    CursorDown(u16),
    /// Cursor Forward (CUF) - ESC [ n C
    CursorForward(u16),
    /// Cursor Backward (CUB) - ESC [ n D
    CursorBackward(u16),
    /// Cursor Position (CUP) - ESC [ row ; col H
    CursorPosition { row: TermRow, col: TermCol },
    /// Cursor Position alternate form (HVP) - ESC [ row ; col f
    CursorPositionAlt { row: TermRow, col: TermCol },
    /// Erase Display (ED) - ESC [ n J
    EraseDisplay(u16),
    /// Erase Line (EL) - ESC [ n K
    EraseLine(u16),
    /// Save Cursor (SCP) - ESC [ s
    SaveCursor,
    /// Restore Cursor (RCP) - ESC [ u
    RestoreCursor,
    /// Cursor Next Line (CNL) - ESC [ n E
    CursorNextLine(u16),
    /// Cursor Previous Line (CPL) - ESC [ n F
    CursorPrevLine(u16),
    /// Cursor Horizontal Absolute (CHA) - ESC [ n G
    CursorHorizontalAbsolute(u16),
    /// Scroll Up (SU) - ESC [ n S
    ScrollUp(u16),
    /// Scroll Down (SD) - ESC [ n T
    ScrollDown(u16),
    /// Set Top and Bottom Margins (DECSTBM) - ESC [ top ; bottom r
    SetScrollingMargins {
        top: Option<TermRow>,
        bottom: Option<TermRow>,
    },
    /// Device Status Report (DSR) - ESC [ n n
    DeviceStatusReport(DsrRequestType),
    /// Enable Private Mode - ESC [ ? n h (n = mode number like `DECAWM_AUTO_WRAP`)
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    EnablePrivateMode(PrivateModeType),
    /// Disable Private Mode - ESC [ ? n l (n = mode number like `DECAWM_AUTO_WRAP`)
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    DisablePrivateMode(PrivateModeType),
    /// Insert Line (IL) - ESC [ n L
    InsertLine(u16),
    /// Delete Line (DL) - ESC [ n M
    DeleteLine(u16),
    /// Delete Character (DCH) - ESC [ n P
    DeleteChar(u16),
    /// Insert Character (ICH) - ESC [ n @
    InsertChar(u16),
    /// Erase Character (ECH) - ESC [ n X
    EraseChar(u16),
    /// Vertical Position Absolute (VPA) - ESC [ n d
    VerticalPositionAbsolute(u16),
}

impl FastStringify for CsiSequence {
    #[allow(clippy::too_many_lines)]
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
        acc.push_str(CSI_START);
        match self {
            CsiSequence::CursorUp(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUU_CURSOR_UP);
            }
            CsiSequence::CursorDown(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUD_CURSOR_DOWN);
            }
            CsiSequence::CursorForward(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUF_CURSOR_FORWARD);
            }
            CsiSequence::CursorBackward(n) => {
                acc.push_str(&n.to_string());
                acc.push(CUB_CURSOR_BACKWARD);
            }
            CsiSequence::CursorPosition { row, col } => {
                acc.push_str(&row.as_u16().to_string());
                acc.push(CSI_PARAM_SEPARATOR);
                acc.push_str(&col.as_u16().to_string());
                acc.push(CUP_CURSOR_POSITION);
            }
            CsiSequence::CursorPositionAlt { row, col } => {
                acc.push_str(&row.as_u16().to_string());
                acc.push(CSI_PARAM_SEPARATOR);
                acc.push_str(&col.as_u16().to_string());
                acc.push(HVP_CURSOR_POSITION);
            }
            CsiSequence::EraseDisplay(n) => {
                acc.push_str(&n.to_string());
                acc.push(ED_ERASE_DISPLAY);
            }
            CsiSequence::EraseLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(EL_ERASE_LINE);
            }
            CsiSequence::SaveCursor => {
                acc.push(SCP_SAVE_CURSOR);
            }
            CsiSequence::RestoreCursor => {
                acc.push(RCP_RESTORE_CURSOR);
            }
            CsiSequence::CursorNextLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(CNL_CURSOR_NEXT_LINE);
            }
            CsiSequence::CursorPrevLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(CPL_CURSOR_PREV_LINE);
            }
            CsiSequence::CursorHorizontalAbsolute(n) => {
                acc.push_str(&n.to_string());
                acc.push(CHA_CURSOR_COLUMN);
            }
            CsiSequence::ScrollUp(n) => {
                acc.push_str(&n.to_string());
                acc.push(SU_SCROLL_UP);
            }
            CsiSequence::ScrollDown(n) => {
                acc.push_str(&n.to_string());
                acc.push(SD_SCROLL_DOWN);
            }
            CsiSequence::SetScrollingMargins { top, bottom } => {
                if let Some(top_row) = top {
                    acc.push_str(&top_row.as_u16().to_string());
                }
                acc.push(CSI_PARAM_SEPARATOR);
                if let Some(bottom_row) = bottom {
                    acc.push_str(&bottom_row.as_u16().to_string());
                }
                acc.push(DECSTBM_SET_MARGINS);
            }
            CsiSequence::DeviceStatusReport(dsr_type) => {
                acc.push_str(&dsr_type.as_u16().to_string());
                acc.push(DSR_DEVICE_STATUS);
            }
            CsiSequence::EnablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                acc.push_str(&mode.as_u16().to_string());
                acc.push(SM_SET_PRIVATE_MODE);
            }
            CsiSequence::DisablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                acc.push_str(&mode.as_u16().to_string());
                acc.push(RM_RESET_PRIVATE_MODE);
            }
            CsiSequence::InsertLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(IL_INSERT_LINE);
            }
            CsiSequence::DeleteLine(n) => {
                acc.push_str(&n.to_string());
                acc.push(DL_DELETE_LINE);
            }
            CsiSequence::DeleteChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(DCH_DELETE_CHAR);
            }
            CsiSequence::InsertChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(ICH_INSERT_CHAR);
            }
            CsiSequence::EraseChar(n) => {
                acc.push_str(&n.to_string());
                acc.push(ECH_ERASE_CHAR);
            }
            CsiSequence::VerticalPositionAbsolute(n) => {
                acc.push_str(&n.to_string());
                acc.push(VPA_VERTICAL_POSITION);
            }
        }
        Ok(())
    }

    fn write_buf_to_fmt(&self, acc: &BufTextStorage, f: &mut Formatter<'_>) -> Result {
        f.write_str(&acc.clone())
    }
}

generate_impl_display_for_fast_stringify!(CsiSequence);
