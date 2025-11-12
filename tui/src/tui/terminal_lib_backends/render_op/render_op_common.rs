// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Common render operations used in both IR (app/component) and Output (backend)
//! contexts.
//!
//! # You Are Here
//!
//! ```text
//! [Stage 1: App/Component] → [Stage 2: Pipeline] → [Stage 3: Compositor] →
//! [Stage 4: Backend Converter] → [Stage 5: Backend Executor] → [Stage 6: Terminal]
//!
//! See [`crate::render_op`] module documentation for shared architectural patterns,
//! type safety guarantees, and the rendering pipeline overview.
//!
//! # Context
//!
//! These 27 shared operations are used identically in:
//! - [`crate::RenderOpIR`] - IR layer for components/app (with clipping info)
//! - [`crate::RenderOpOutput`] - Output layer for backend (post-clipping)
//!
//! The enum defines operations like cursor movement, colors, text, and screen control
//! that every stage of the pipeline needs to understand.

use crate::{ColIndex, InlineString, Pos, RowHeight, TuiColor, TuiStyle};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderOpCommon {
    /// Enables terminal raw mode for direct control over input/output.
    ///
    /// Raw mode disables line buffering and special character processing,
    /// allowing the application to receive keystrokes immediately and
    /// handle all terminal control sequences directly.
    EnterRawMode,

    /// Exits terminal raw mode and restores normal terminal behavior.
    ///
    /// This restores line buffering and standard terminal input processing.
    /// Should always be called before application exit to avoid leaving
    /// the terminal in an unusable state.
    ExitRawMode,

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
    /// Maps to CSI `<n>G` ANSI sequence (1-indexed).
    ///
    /// Useful for incremental rendering operations where you need precise
    /// horizontal cursor positioning without affecting the row.
    MoveCursorToColumn(ColIndex),

    /// Move cursor down by N lines and to column 0 (start of line).
    ///
    /// Maps to CSI `<n>E` ANSI sequence. Equivalent to moving down N rows
    /// and then moving to column 0. Used for line-by-line incremental rendering.
    MoveCursorToNextLine(RowHeight),

    /// Move cursor up by N lines and to column 0 (start of line).
    ///
    /// Maps to CSI `<n>F` ANSI sequence. Useful for updating content above
    /// the current cursor position, with safe bounds checking.
    MoveCursorToPreviousLine(RowHeight),

    /// Clear current line only, leaving cursor position unchanged.
    ///
    /// Maps to CSI `2K` ANSI sequence. Erases the entire line from start to end,
    /// but preserves the current cursor column position.
    ClearCurrentLine,

    /// Clear from cursor to end of line (inclusive).
    ///
    /// Maps to CSI `0K` (or `CSI K`) ANSI sequence. Erases from the cursor position
    /// to the end of the line, leaving the cursor position unchanged.
    ///
    /// Useful for partial line updates where you want to preserve content to the left
    /// of the cursor but clear everything to the right.
    ClearToEndOfLine,

    /// Clear from cursor to beginning of line (inclusive).
    ///
    /// Maps to CSI `1K` ANSI sequence. Erases from the start of the line to the
    /// cursor position (inclusive), leaving the cursor position unchanged.
    ///
    /// Useful for left-side clearing operations in incremental rendering.
    ClearToStartOfLine,

    /// Print text that already contains ANSI escape codes (pre-styled text).
    ///
    /// No additional styling applied - text is rendered exactly as provided.
    ///
    /// This variant is used when `CliTextInline` or other text formatting
    /// has already generated the final ANSI-escaped output. The text is printed
    /// as-is without any additional processing, attribute application, or color
    /// application.
    PrintStyledText(InlineString),

    /// Show cursor (make it visible).
    ///
    /// Maps to CSI `?25h` ANSI sequence (DEC Private Mode Set).
    ///
    /// Restores cursor visibility after it has been hidden with
    /// [`RenderOpCommon::HideCursor`].
    ShowCursor,

    /// Hide cursor (make it invisible).
    ///
    /// Maps to CSI `?25l` ANSI sequence (DEC Private Mode Reset).
    ///
    /// Useful for animations or rendering where cursor visibility would be distracting.
    /// Remember to call [`RenderOpCommon::ShowCursor`] before normal operation resumes.
    HideCursor,

    /// Save cursor position to be restored later.
    ///
    /// Maps to CSI `s` ANSI sequence (also known as DECSC - save cursor).
    ///
    /// Saves the current cursor position (row and column) in terminal memory.
    /// Use with [`RenderOpCommon::RestoreCursorPosition`] to return to this position.
    ///
    /// Note: Some terminals may not support this sequence. Use with caution
    /// in cross-platform applications.
    SaveCursorPosition,

    /// Restore cursor position previously saved with
    /// [`RenderOpCommon::SaveCursorPosition`].
    ///
    /// Maps to CSI `u` ANSI sequence (also known as DECRC - restore cursor).
    ///
    /// Restores the cursor to the position that was previously saved.
    ///
    /// Note: Some terminals may not support this sequence. Must be preceded by
    /// a corresponding [`RenderOpCommon::SaveCursorPosition`] call.
    RestoreCursorPosition,

    /// Switches to alternate screen buffer for full-screen applications.
    ///
    /// When enabled, the terminal saves the current screen content and switches to an
    /// alternate buffer. This is used by full-screen applications (vim, less, etc.) to
    /// preserve shell history and avoid cluttering the original screen.
    ///
    /// Maps to CSI `?1049h` ANSI sequence (DEC Private Mode Set).
    EnterAlternateScreen,

    /// Exits alternate screen buffer and restores original screen content.
    ///
    /// Restores the screen content that was saved when
    /// [`RenderOpCommon::EnterAlternateScreen`] was called. Should always be called
    /// before returning to normal shell operation.
    ///
    /// Maps to CSI `?1049l` ANSI sequence (DEC Private Mode Reset).
    ExitAlternateScreen,

    /// Enables mouse event tracking (clicks, movement, scroll).
    ///
    /// When enabled, the terminal reports mouse events to the application.
    /// This includes mouse clicks, movements, and scroll wheel events.
    ///
    /// Maps to CSI `?1000h` ANSI sequence (DEC Private Mode Set for mouse tracking).
    EnableMouseTracking,

    /// Disables mouse event tracking.
    ///
    /// Restores normal mouse behavior where the terminal no longer reports mouse events
    /// to the application. Called to restore normal operation after mouse tracking is
    /// no longer needed.
    ///
    /// Maps to CSI `?1000l` ANSI sequence (DEC Private Mode Reset).
    DisableMouseTracking,

    /// Enables bracketed paste mode for distinguishing pasted text.
    ///
    /// When enabled, text pasted from the clipboard is wrapped with special escape
    /// sequences, allowing the application to distinguish pasted content from keyboard
    /// input. This prevents pasted content from being misinterpreted as commands.
    ///
    /// Maps to CSI `?2004h` ANSI sequence (DEC Private Mode Set for bracketed paste).
    EnableBracketedPaste,

    /// Disables bracketed paste mode.
    ///
    /// Restores normal paste behavior where the terminal doesn't wrap pasted text
    /// with special escape sequences. Called when clipboard detection is no longer
    /// needed.
    ///
    /// Maps to CSI `?2004l` ANSI sequence (DEC Private Mode Reset).
    DisableBracketedPaste,

    /// No-operation render operation that does nothing when executed.
    ///
    /// Used as a placeholder or default value in situations where a render operation
    /// is required but no actual rendering should occur. Safe to include in operation
    /// lists as it has no side effects.
    Noop,
}
