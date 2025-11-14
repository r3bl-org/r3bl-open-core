// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{RenderOpCommon, RenderOpIR, RenderOpOutput};
use crate::{ColIndex, InlineString, Pos, RowHeight, TuiColor, TuiStyle};

/// Trait providing ergonomic helper methods for common operations.
///
/// Implemented by both `RenderOpIR` and `RenderOpOutput` to avoid code duplication.
/// Provides factory methods that wrap `RenderOpCommon` variants in the appropriate type.
///
/// # You Are Here: **Supporting Trait** (Cross-Stage)
///
/// ```text
/// RenderOpCommonExt trait ← YOU ARE HERE
/// (Used by all pipeline stages 1-5)
/// ```
///
/// <div class="warning">
///
/// **For the complete 6-stage rendering pipeline with visual diagrams and stage
/// reference table**, see the [rendering pipeline overview].
///
/// </div>
///
/// # Purpose
///
/// Provides 27 ergonomic factory methods that wrap `RenderOpCommon` variants.
/// Used throughout the rendering pipeline to create render operations conveniently.
///
/// [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture
///
/// # Usage
///
/// Instead of `RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(pos))`,
/// use the ergonomic helper method: `RenderOpIR::move_cursor(pos)`.
pub trait RenderOpCommonExt: Sized {
    /// Convert a common operation into this specific type (IR or Output).
    fn from_common(common: RenderOpCommon) -> Self;

    // === Terminal Mode Operations ===

    #[must_use]
    fn enter_raw_mode() -> Self { Self::from_common(RenderOpCommon::EnterRawMode) }

    #[must_use]
    fn exit_raw_mode() -> Self { Self::from_common(RenderOpCommon::ExitRawMode) }

    // === Cursor Movement Operations ===

    #[must_use]
    fn move_cursor(pos: Pos) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorPositionAbs(pos))
    }

    #[must_use]
    fn move_cursor_rel(origin: Pos, offset: Pos) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorPositionRelTo(origin, offset))
    }

    #[must_use]
    fn move_to_column(col: ColIndex) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToColumn(col))
    }

    #[must_use]
    fn move_to_next_line(rows: RowHeight) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToNextLine(rows))
    }

    #[must_use]
    fn move_to_previous_line(rows: RowHeight) -> Self {
        Self::from_common(RenderOpCommon::MoveCursorToPreviousLine(rows))
    }

    // === Screen Clearing Operations ===

    #[must_use]
    fn clear_screen() -> Self { Self::from_common(RenderOpCommon::ClearScreen) }

    #[must_use]
    fn clear_current_line() -> Self {
        Self::from_common(RenderOpCommon::ClearCurrentLine)
    }

    #[must_use]
    fn clear_to_end_of_line() -> Self {
        Self::from_common(RenderOpCommon::ClearToEndOfLine)
    }

    #[must_use]
    fn clear_to_start_of_line() -> Self {
        Self::from_common(RenderOpCommon::ClearToStartOfLine)
    }

    // === Color Operations ===

    #[must_use]
    fn set_fg_color(color: TuiColor) -> Self {
        Self::from_common(RenderOpCommon::SetFgColor(color))
    }

    #[must_use]
    fn set_bg_color(color: TuiColor) -> Self {
        Self::from_common(RenderOpCommon::SetBgColor(color))
    }

    #[must_use]
    fn reset_color() -> Self { Self::from_common(RenderOpCommon::ResetColor) }

    #[must_use]
    fn apply_colors(style: Option<TuiStyle>) -> Self {
        Self::from_common(RenderOpCommon::ApplyColors(style))
    }

    // === Text Output Operations ===

    #[must_use]
    fn print_styled_text(text: InlineString) -> Self {
        Self::from_common(RenderOpCommon::PrintStyledText(text))
    }

    // === Cursor Visibility Operations ===

    #[must_use]
    fn show_cursor() -> Self { Self::from_common(RenderOpCommon::ShowCursor) }

    #[must_use]
    fn hide_cursor() -> Self { Self::from_common(RenderOpCommon::HideCursor) }

    // === Cursor Position Save/Restore ===

    #[must_use]
    fn save_cursor_position() -> Self {
        Self::from_common(RenderOpCommon::SaveCursorPosition)
    }

    #[must_use]
    fn restore_cursor_position() -> Self {
        Self::from_common(RenderOpCommon::RestoreCursorPosition)
    }

    // === Alternate Screen Operations ===

    #[must_use]
    fn enter_alternate_screen() -> Self {
        Self::from_common(RenderOpCommon::EnterAlternateScreen)
    }

    #[must_use]
    fn exit_alternate_screen() -> Self {
        Self::from_common(RenderOpCommon::ExitAlternateScreen)
    }

    // === Mouse Tracking Operations ===

    #[must_use]
    fn enable_mouse_tracking() -> Self {
        Self::from_common(RenderOpCommon::EnableMouseTracking)
    }

    #[must_use]
    fn disable_mouse_tracking() -> Self {
        Self::from_common(RenderOpCommon::DisableMouseTracking)
    }

    // === Bracketed Paste Operations ===

    #[must_use]
    fn enable_bracketed_paste() -> Self {
        Self::from_common(RenderOpCommon::EnableBracketedPaste)
    }

    #[must_use]
    fn disable_bracketed_paste() -> Self {
        Self::from_common(RenderOpCommon::DisableBracketedPaste)
    }

    // === No-op ===

    #[must_use]
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
