// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Trait providing ergonomic helper methods for common operations.
//!
//! # You Are Here
//!
//! ```text
//! [S1: App/Component] → [S2: Pipeline] → [S3: Compositor] →
//! [S4: Backend Converter] → [S5: Backend Executor] → [S6: Terminal]
//!
//! RenderOpCommonExt is used throughout all stages
//! ```
//!
//! Implemented by both `RenderOpIR` and `RenderOpOutput` to avoid code duplication.
//! Provides 27 ergonomic factory methods that wrap `RenderOpCommon` variants.
//!
//! # Usage
//!
//! ```ignore
//! use r3bl_tui::{RenderOpIR, RenderOpCommonExt, Pos};
//!
//! // Instead of: RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos))
//! let op = RenderOpIR::move_cursor(pos);
//! ```

use super::{RenderOpCommon, RenderOpIR, RenderOpOutput};
use crate::{ColIndex, InlineString, Pos, RowHeight, TuiColor, TuiStyle};

/// Trait providing ergonomic helper methods for common operations.
///
/// Implemented by both `RenderOpIR` and `RenderOpOutput` to avoid code duplication.
/// Provides factory methods that wrap `RenderOpCommon` variants in the appropriate type.
///
/// # Usage
///
/// ```ignore
/// use r3bl_tui::{RenderOpIR, RenderOpCommonExt, Pos};
///
/// // Instead of: RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos))
/// // Use the helper:
/// let op = RenderOpIR::move_cursor(pos);
/// ```
pub trait RenderOpCommonExt: Sized {
    /// Convert a common operation into this specific type (IR or Output).
    fn from_common(common: RenderOpCommon) -> Self;

    // === Terminal Mode Operations ===

    fn enter_raw_mode() -> Self { Self::from_common(RenderOpCommon::EnterRawMode) }

    fn exit_raw_mode() -> Self { Self::from_common(RenderOpCommon::ExitRawMode) }

    // === Cursor Movement Operations ===

    fn move_cursor(pos: Pos) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorPositionAbs(pos))
    }

    fn move_cursor_rel(origin: Pos, offset: Pos) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorPositionRelTo(origin, offset))
    }

    fn move_to_column(col: ColIndex) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToColumn(col))
    }

    fn move_to_next_line(rows: RowHeight) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToNextLine(rows))
    }

    fn move_to_previous_line(rows: RowHeight) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToPreviousLine(rows))
    }

    // === Screen Clearing Operations ===

    fn clear_screen() -> Self { Self::from_common(RenderOpCommon::ClearScreen) }

    fn clear_current_line() -> Self {
        Self::from_common(RenderOpCommon::ClearCurrentLine)
    }

    fn clear_to_end_of_line() -> Self {
        Self::from_common(RenderOpCommon::ClearToEndOfLine)
    }

    fn clear_to_start_of_line() -> Self {
        Self::from_common(RenderOpCommon::ClearToStartOfLine)
    }

    // === Color Operations ===

    fn set_fg_color(color: TuiColor) -> Self {
        Self::from_common(RenderOpCommon::SetFgColor(color))
    }

    fn set_bg_color(color: TuiColor) -> Self {
        Self::from_common(RenderOpCommon::SetBgColor(color))
    }

    fn reset_color() -> Self { Self::from_common(RenderOpCommon::ResetColor) }

    fn apply_colors(style: Option<TuiStyle>) -> Self {
        Self::from_common(RenderOpCommon::ApplyColors(style))
    }

    // === Text Output Operations ===

    fn print_styled_text(text: InlineString) -> Self {
        Self::from_common(RenderOpCommon::PrintStyledText(text))
    }

    // === Cursor Visibility Operations ===

    fn show_cursor() -> Self { Self::from_common(RenderOpCommon::ShowCursor) }

    fn hide_cursor() -> Self { Self::from_common(RenderOpCommon::HideCursor) }

    // === Cursor Position Save/Restore ===

    fn save_cursor_position() -> Self {
        Self::from_common(RenderOpCommon::SaveCursorPosition)
    }

    fn restore_cursor_position() -> Self {
        Self::from_common(RenderOpCommon::RestoreCursorPosition)
    }

    // === Alternate Screen Operations ===

    fn enter_alternate_screen() -> Self {
        Self::from_common(RenderOpCommon::EnterAlternateScreen)
    }

    fn exit_alternate_screen() -> Self {
        Self::from_common(RenderOpCommon::ExitAlternateScreen)
    }

    // === Mouse Tracking Operations ===

    fn enable_mouse_tracking() -> Self {
        Self::from_common(RenderOpCommon::EnableMouseTracking)
    }

    fn disable_mouse_tracking() -> Self {
        Self::from_common(RenderOpCommon::DisableMouseTracking)
    }

    // === Bracketed Paste Operations ===

    fn enable_bracketed_paste() -> Self {
        Self::from_common(RenderOpCommon::EnableBracketedPaste)
    }

    fn disable_bracketed_paste() -> Self {
        Self::from_common(RenderOpCommon::DisableBracketedPaste)
    }

    // === No-op ===

    fn noop() -> Self { Self::from_common(RenderOpCommon::Noop) }
}

// Implement trait for RenderOpIR
impl RenderOpCommonExt for RenderOpIR {
    fn from_common(common: RenderOpCommon) -> Self { RenderOpIR::Common(common) }
}

// Implement trait for RenderOpOutput
impl RenderOpCommonExt for RenderOpOutput {
    fn from_common(common: RenderOpCommon) -> Self { RenderOpOutput::Common(common) }
}
