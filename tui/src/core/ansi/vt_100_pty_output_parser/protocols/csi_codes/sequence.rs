// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! CSI sequence builder for terminal control operations.
//!
//! This module provides the `CsiSequence` enum which represents various CSI control
//! sequences and can serialize them into ANSI escape codes.

use super::{erase_mode::{EraseDisplayMode, EraseLineMode}, private_mode::PrivateModeType};
use crate::{BufTextStorage, CsiCount, FastStringify, NumericConversions, TermCol, TermColDelta,
            TermRow, TermRowDelta,
            core::ansi::{constants::{CHA_CURSOR_COLUMN, CNL_CURSOR_NEXT_LINE,
                                     CPL_CURSOR_PREV_LINE, CSI_PARAM_SEPARATOR,
                                     CSI_PRIVATE_MODE_PREFIX, CSI_START,
                                     CUB_CURSOR_BACKWARD, CUD_CURSOR_DOWN,
                                     CUF_CURSOR_FORWARD, CUP_CURSOR_POSITION,
                                     CUU_CURSOR_UP, DCH_DELETE_CHAR,
                                     DECSTBM_SET_MARGINS, DL_DELETE_LINE,
                                     DSR_DEVICE_STATUS, ECH_ERASE_CHAR,
                                     ED_ERASE_DISPLAY, EL_ERASE_LINE,
                                     HVP_CURSOR_POSITION, ICH_INSERT_CHAR,
                                     IL_INSERT_LINE, RCP_RESTORE_CURSOR,
                                     RM_RESET_PRIVATE_MODE, SCP_SAVE_CURSOR,
                                     SD_SCROLL_DOWN, SM_SET_PRIVATE_MODE,
                                     SU_SCROLL_UP, VPA_VERTICAL_POSITION},
                         generator::DsrRequestType},
            generate_impl_display_for_fast_stringify,
            stack_alloc_types::usize_fmt::{convert_u16_to_string_slice, u16_to_u8_array}};
use std::fmt::{Formatter, Result};

/// Builder for CSI (Control Sequence Introducer) sequences.
/// Similar to `SgrCode` but for cursor movement and other CSI commands.
///
/// # Make Illegal States Unrepresentable
///
/// This enum uses type-safe wrappers to prevent the CSI zero bug at compile time:
///
/// - **Relative movements** ([`TermRowDelta`], [`TermColDelta`]): For cursor
///   up/down/forward/backward
/// - **Absolute positions** ([`TermRow`], [`TermCol`]): For cursor positioning
/// - **Counts** ([`CsiCount`]): For insert/delete/erase operations
/// - **Modes** ([`EraseDisplayMode`], [`EraseLineMode`]): For erase operations
///
/// ANSI terminals interpret parameter 0 as 1 for most commands:
/// - `CSI 0 A` moves cursor **1 row up**, not 0
/// - `CSI 0 G` moves cursor to **column 1**, not 0
/// - `CSI 0 L` inserts **1 line**, not 0
///
/// Since all wrapper types use [`NonZeroU16`] internally, you are **forced** to
/// handle the zero case at construction time:
///
/// ```rust
/// use r3bl_tui::{TermRowDelta, CsiSequence};
///
/// // Fallible construction - must handle the None case
/// if let Some(delta) = TermRowDelta::new(3) {
///     // delta is guaranteed non-zero, safe to emit
///     let _ = CsiSequence::CursorDown(delta);
/// }
/// ```
///
/// [`TermRowDelta`]: crate::TermRowDelta
/// [`TermColDelta`]: crate::TermColDelta
/// [`TermRow`]: crate::TermRow
/// [`TermCol`]: crate::TermCol
/// [`CsiCount`]: crate::CsiCount
/// [`EraseDisplayMode`]: crate::EraseDisplayMode
/// [`EraseLineMode`]: crate::EraseLineMode
/// [`NonZeroU16`]: std::num::NonZeroU16
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CsiSequence {
    /// Cursor Up (CUU) - ESC [ n A.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    CursorUp(TermRowDelta),
    /// Cursor Down (CUD) - ESC [ n B.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    CursorDown(TermRowDelta),
    /// Cursor Forward (CUF) - ESC [ n C.
    ///
    /// Uses [`TermColDelta`] to prevent CSI zero bug.
    CursorForward(TermColDelta),
    /// Cursor Backward (CUB) - ESC [ n D.
    ///
    /// Uses [`TermColDelta`] to prevent CSI zero bug.
    CursorBackward(TermColDelta),
    /// Cursor Position (CUP) - ESC [ row ; col H.
    CursorPosition { row: TermRow, col: TermCol },
    /// Cursor Position alternate form (HVP) - ESC [ row ; col f.
    CursorPositionAlt { row: TermRow, col: TermCol },
    /// Erase Display (ED) - ESC [ n J.
    ///
    /// Uses [`EraseDisplayMode`] enum for type-safe mode selection.
    EraseDisplay(EraseDisplayMode),
    /// Erase Line (EL) - ESC [ n K.
    ///
    /// Uses [`EraseLineMode`] enum for type-safe mode selection.
    EraseLine(EraseLineMode),
    /// Save Cursor (SCP) - ESC [ s.
    SaveCursor,
    /// Restore Cursor (RCP) - ESC [ u.
    RestoreCursor,
    /// Cursor Next Line (CNL) - ESC [ n E.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    CursorNextLine(TermRowDelta),
    /// Cursor Previous Line (CPL) - ESC [ n F.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    CursorPrevLine(TermRowDelta),
    /// Cursor Horizontal Absolute (CHA) - ESC [ n G.
    ///
    /// Uses [`TermCol`] for type-safe 1-based column positioning.
    CursorHorizontalAbsolute(TermCol),
    /// Scroll Up (SU) - ESC [ n S.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    ScrollUp(TermRowDelta),
    /// Scroll Down (SD) - ESC [ n T.
    ///
    /// Uses [`TermRowDelta`] to prevent CSI zero bug.
    ScrollDown(TermRowDelta),
    /// Set Top and Bottom Margins (DECSTBM) - ESC [ top ; bottom r.
    SetScrollingMargins {
        top: Option<TermRow>,
        bottom: Option<TermRow>,
    },
    /// Device Status Report (DSR) - ESC [ n n.
    DeviceStatusReport(DsrRequestType),
    /// Enable Private Mode - ESC [ ? n h (n = mode number like `DECAWM_AUTO_WRAP`).
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    EnablePrivateMode(PrivateModeType),
    /// Disable Private Mode - ESC [ ? n l (n = mode number like `DECAWM_AUTO_WRAP`).
    /// See [`crate::offscreen_buffer::AnsiParserSupport::auto_wrap_mode`]
    DisablePrivateMode(PrivateModeType),
    /// Insert Line (IL) - ESC [ n L.
    ///
    /// Uses [`CsiCount`] to prevent CSI zero bug.
    InsertLine(CsiCount),
    /// Delete Line (DL) - ESC [ n M.
    ///
    /// Uses [`CsiCount`] to prevent CSI zero bug.
    DeleteLine(CsiCount),
    /// Delete Character (DCH) - ESC [ n P.
    ///
    /// Uses [`CsiCount`] to prevent CSI zero bug.
    DeleteChar(CsiCount),
    /// Insert Character (ICH) - ESC [ n @.
    ///
    /// Uses [`CsiCount`] to prevent CSI zero bug.
    InsertChar(CsiCount),
    /// Erase Character (ECH) - ESC [ n X.
    ///
    /// Uses [`CsiCount`] to prevent CSI zero bug.
    EraseChar(CsiCount),
    /// Vertical Position Absolute (VPA) - ESC [ n d.
    ///
    /// Uses [`TermRow`] for type-safe 1-based row positioning.
    VerticalPositionAbsolute(TermRow),
}

impl FastStringify for CsiSequence {
    #[allow(clippy::too_many_lines)]
    fn write_to_buf(&self, acc: &mut BufTextStorage) -> Result {
        acc.push_str(CSI_START);
        match self {
            CsiSequence::CursorUp(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CUU_CURSOR_UP);
            }
            CsiSequence::CursorDown(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CUD_CURSOR_DOWN);
            }
            CsiSequence::CursorForward(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CUF_CURSOR_FORWARD);
            }
            CsiSequence::CursorBackward(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CUB_CURSOR_BACKWARD);
            }
            CsiSequence::CursorPosition { row, col } => {
                let row_bytes = u16_to_u8_array(row.as_u16());
                acc.push_str(convert_u16_to_string_slice(&row_bytes));
                acc.push(CSI_PARAM_SEPARATOR);
                let col_bytes = u16_to_u8_array(col.as_u16());
                acc.push_str(convert_u16_to_string_slice(&col_bytes));
                acc.push(CUP_CURSOR_POSITION);
            }
            CsiSequence::CursorPositionAlt { row, col } => {
                let row_bytes = u16_to_u8_array(row.as_u16());
                acc.push_str(convert_u16_to_string_slice(&row_bytes));
                acc.push(CSI_PARAM_SEPARATOR);
                let col_bytes = u16_to_u8_array(col.as_u16());
                acc.push_str(convert_u16_to_string_slice(&col_bytes));
                acc.push(HVP_CURSOR_POSITION);
            }
            CsiSequence::EraseDisplay(mode) => {
                let n_bytes = u16_to_u8_array(mode.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(ED_ERASE_DISPLAY);
            }
            CsiSequence::EraseLine(mode) => {
                let n_bytes = u16_to_u8_array(mode.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(EL_ERASE_LINE);
            }
            CsiSequence::SaveCursor => {
                acc.push(SCP_SAVE_CURSOR);
            }
            CsiSequence::RestoreCursor => {
                acc.push(RCP_RESTORE_CURSOR);
            }
            CsiSequence::CursorNextLine(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CNL_CURSOR_NEXT_LINE);
            }
            CsiSequence::CursorPrevLine(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CPL_CURSOR_PREV_LINE);
            }
            CsiSequence::CursorHorizontalAbsolute(col) => {
                let n_bytes = u16_to_u8_array(col.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(CHA_CURSOR_COLUMN);
            }
            CsiSequence::ScrollUp(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(SU_SCROLL_UP);
            }
            CsiSequence::ScrollDown(delta) => {
                let n_bytes = u16_to_u8_array(delta.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(SD_SCROLL_DOWN);
            }
            CsiSequence::SetScrollingMargins { top, bottom } => {
                if let Some(top_row) = top {
                    let top_bytes = u16_to_u8_array(top_row.as_u16());
                    acc.push_str(convert_u16_to_string_slice(&top_bytes));
                }
                acc.push(CSI_PARAM_SEPARATOR);
                if let Some(bottom_row) = bottom {
                    let bottom_bytes = u16_to_u8_array(bottom_row.as_u16());
                    acc.push_str(convert_u16_to_string_slice(&bottom_bytes));
                }
                acc.push(DECSTBM_SET_MARGINS);
            }
            CsiSequence::DeviceStatusReport(dsr_type) => {
                let dsr_bytes = u16_to_u8_array(dsr_type.as_u16());
                acc.push_str(convert_u16_to_string_slice(&dsr_bytes));
                acc.push(DSR_DEVICE_STATUS);
            }
            CsiSequence::EnablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                let mode_bytes = u16_to_u8_array(mode.as_u16());
                acc.push_str(convert_u16_to_string_slice(&mode_bytes));
                acc.push(SM_SET_PRIVATE_MODE);
            }
            CsiSequence::DisablePrivateMode(mode) => {
                acc.push(CSI_PRIVATE_MODE_PREFIX);
                let mode_bytes = u16_to_u8_array(mode.as_u16());
                acc.push_str(convert_u16_to_string_slice(&mode_bytes));
                acc.push(RM_RESET_PRIVATE_MODE);
            }
            CsiSequence::InsertLine(count) => {
                let n_bytes = u16_to_u8_array(count.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(IL_INSERT_LINE);
            }
            CsiSequence::DeleteLine(count) => {
                let n_bytes = u16_to_u8_array(count.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(DL_DELETE_LINE);
            }
            CsiSequence::DeleteChar(count) => {
                let n_bytes = u16_to_u8_array(count.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(DCH_DELETE_CHAR);
            }
            CsiSequence::InsertChar(count) => {
                let n_bytes = u16_to_u8_array(count.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(ICH_INSERT_CHAR);
            }
            CsiSequence::EraseChar(count) => {
                let n_bytes = u16_to_u8_array(count.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
                acc.push(ECH_ERASE_CHAR);
            }
            CsiSequence::VerticalPositionAbsolute(row) => {
                let n_bytes = u16_to_u8_array(row.as_u16());
                acc.push_str(convert_u16_to_string_slice(&n_bytes));
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
