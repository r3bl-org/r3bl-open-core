// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common render operations used in both IR (app/component) and Output (backend)
//! contexts.
//!
//! # You Are Here: **Shared Data Type** (Cross-Stage)
//!
//! ```text
//! RenderOpCommon enum ← YOU ARE HERE
//! (Used by all pipeline stages 1-5)
//! ```
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! # Context
//!
//! These shared operations are used identically in:
//! - [`crate::RenderOpIR`] - IR layer for components/app (with clipping info)
//! - [`crate::RenderOpOutput`] - Output layer for backend (post-clipping)
//!
//! The enum defines operations like cursor movement, colors, text, and screen control
//! that every stage of the pipeline needs to understand.
//!
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

use crate::{ColIndex, InlineString, Pos, RowHeight, TuiColor, TuiStyle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderOpCommon {

    /// Move cursor to absolute position. This is always painted on top.
    ///
    /// Position is the absolute column and row on the terminal screen.
    /// The compositor uses [`sanitize_and_save_abs_pos`] to clean up the given position.
    ///
    /// [`sanitize_and_save_abs_pos`]: crate::paint::sanitize_and_save_abs_pos
    MoveCursorPositionAbs(/* absolute position */ Pos),

    /// Move cursor relative to origin. 1st position is origin, 2nd is offset.
    ///
    /// They are added together to move the absolute position on the terminal screen.
    /// Then [`RenderOpCommon::MoveCursorPositionAbs`] is used internally.
    MoveCursorPositionRelTo(
        /* origin position */ Pos,
        /* relative position */ Pos,
    ),

    /// Clears the entire terminal screen and positions cursor at top-left.
    ///
    /// This operation erases all visible content on the terminal screen
    /// and resets the cursor to position (0, 0). Useful for initializing
    /// a clean display state before rendering new content.
    ClearScreen,

    /// Directly set the fg color for crossterm without using [`TuiStyle`].
    SetFgColor(TuiColor),

    /// Directly set the bg color for crossterm without using [`TuiStyle`].
    SetBgColor(TuiColor),

    /// Resets terminal colors to their default values.
    ///
    /// This clears any previously set foreground and background colors,
    /// returning the terminal to its default color scheme. Essential for
    /// ensuring clean color state between different rendering operations.
    ResetColor,

    /// Translate [`TuiStyle`] into fg and bg colors for crossterm.
    ///
    /// Note that this does not apply attributes (bold, italic, underline, etc.).
    /// If you need to apply attributes, use context-specific text painting operations
    /// instead.
    ApplyColors(Option<TuiStyle>),

    /// Move cursor to specific column in current row (leaving row unchanged).
    ///
    /// Maps to [`CSI`] `<n>G` [`ANSI`] sequence (1-indexed).
    ///
    /// Useful for incremental rendering operations where you need precise
    /// horizontal cursor positioning without affecting the row.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    MoveCursorToColumn(ColIndex),

    /// Move cursor down by N lines and to column 0 (start of line).
    ///
    /// Maps to [`CSI`] `<n>E` [`ANSI`] sequence. Equivalent to moving down N rows
    /// and then moving to column 0. Used for line-by-line incremental rendering.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    MoveCursorToNextLine(RowHeight),

    /// Move cursor up by N lines and to column 0 (start of line).
    ///
    /// Maps to [`CSI`] `<n>F` [`ANSI`] sequence. Useful for updating content above
    /// the current cursor position, with safe bounds checking.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    MoveCursorToPreviousLine(RowHeight),

    /// Clear current line only, leaving cursor position unchanged.
    ///
    /// Maps to [`CSI`] `2K` [`ANSI`] sequence. Erases the entire line from start to end,
    /// but preserves the current cursor column position.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    ClearCurrentLine,

    /// Clear from cursor to end of line (inclusive).
    ///
    /// Maps to [`CSI`] `0K` (or `CSI K`) [`ANSI`] sequence. Erases from the cursor
    /// position to the end of the line, leaving the cursor position unchanged.
    ///
    /// Useful for partial line updates where you want to preserve content to the left
    /// of the cursor but clear everything to the right.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    ClearToEndOfLine,

    /// Clear from cursor to beginning of line (inclusive).
    ///
    /// Maps to [`CSI`] `1K` [`ANSI`] sequence. Erases from the start of the line to the
    /// cursor position (inclusive), leaving the cursor position unchanged.
    ///
    /// Useful for left-side clearing operations in incremental rendering.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    ClearToStartOfLine,

    /// Print text that already contains [`ANSI`] escape codes (pre-styled text).
    ///
    /// No additional styling applied - text is rendered exactly as provided.
    ///
    /// This variant is used when `CliTextInline` or other text formatting
    /// has already generated the final [`ANSI`]-escaped output. The text is printed
    /// as-is without any additional processing, attribute application, or color
    /// application.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    PrintStyledText(InlineString),


    /// Save cursor position to be restored later.
    ///
    /// Maps to [`CSI`] `s` [`ANSI`] sequence (also known as [`DECSC`] - save cursor).
    ///
    /// Saves the current cursor position (row and column) in terminal memory.
    /// Use with [`RenderOpCommon::RestoreCursorPosition`] to return to this position.
    ///
    /// Note: Some terminals may not support this sequence. Use with caution
    /// in cross-platform applications.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DECSC`]: https://vt100.net/docs/vt510-rm/DECSC.html
    SaveCursorPosition,

    /// Restore cursor position previously saved with
    /// [`RenderOpCommon::SaveCursorPosition`].
    ///
    /// Maps to [`CSI`] `u` [`ANSI`] sequence (also known as [`DECRC`] - restore cursor).
    ///
    /// Restores the cursor to the position that was previously saved.
    ///
    /// Note: Some terminals may not support this sequence. Must be preceded by
    /// a corresponding [`RenderOpCommon::SaveCursorPosition`] call.
    ///
    /// [`ANSI`]: https://en.wikipedia.org/wiki/ANSI_escape_code
    /// [`CSI`]: crate::CsiSequence
    /// [`DECRC`]: https://vt100.net/docs/vt510-rm/DECRC.html
    RestoreCursorPosition,


    /// No-operation render operation that does nothing when executed.
    ///
    /// Used as a placeholder or default value in situations where a render operation
    /// is required but no actual rendering should occur. Safe to include in operation
    /// lists as it has no side effects.
    Noop,
}
