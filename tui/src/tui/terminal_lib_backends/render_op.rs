// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.
use super::TERMINAL_LIB_BACKEND;
use crate::{ColIndex, InlineString, InlineVec, LockedOutputDevice,
            PaintRenderOpImplCrossterm, Pos, RowHeight, Size, TerminalLibBackend, TuiColor,
            TuiStyle, ok};
use std::{fmt::{Debug, Formatter, Result},
          ops::{AddAssign, Deref, DerefMut}};

/// Local state tracking for render operations optimization.
///
/// Maintains the last known terminal state to avoid sending redundant
/// escape sequences when the state hasn't changed. This significantly
/// reduces the amount of data sent to the terminal.
#[derive(Default, Debug)]
pub struct RenderOpsLocalData {
    /// Current cursor position in the terminal.
    ///
    /// Used to determine if cursor movement commands need to be sent
    /// when rendering at a new position.
    pub cursor_pos: Pos,

    /// Last known foreground color.
    ///
    /// Tracks the current foreground color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub fg_color: Option<TuiColor>,

    /// Last known background color.
    ///
    /// Tracks the current background color to avoid sending redundant
    /// color escape sequences when the color hasn't changed.
    pub bg_color: Option<TuiColor>,
}

/// Implementation details for [`RenderOpsIR`] functionality.
///
/// This module contains the core implementation of render operation execution,
/// including methods for processing operation lists, routing to backend implementations,
/// and providing convenient trait implementations for common operations.
pub mod render_ops_impl {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl RenderOpsIR {
        /// Executes all render operations in the collection sequentially.
        ///
        /// This method processes each [`RenderOpIR`] in the list, maintaining local state
        /// for optimization and routing each operation to the appropriate backend
        /// implementation based on the configured terminal library.
        ///
        /// # Parameters
        /// - `skip_flush`: Mutable reference to control flush behavior
        /// - `window_size`: Current terminal window dimensions
        /// - `locked_output_device`: Locked terminal output for thread-safe writing
        /// - `is_mock`: Whether this is a mock execution for testing
        pub fn execute_all(
            &self,
            skip_flush: &mut bool,
            window_size: Size,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            let mut render_local_data = RenderOpsLocalData::default();
            for render_op_ir in &self.list {
                RenderOpsIR::route_paint_render_op_ir_to_backend(
                    &mut render_local_data,
                    skip_flush,
                    render_op_ir,
                    window_size,
                    locked_output_device,
                    is_mock,
                );
            }
        }

        /// Routes a single IR render operation to the appropriate backend implementation.
        ///
        /// This method acts as a dispatcher, selecting the correct terminal library
        /// backend (currently Crossterm) and delegating the actual rendering work
        /// to the backend-specific implementation.
        ///
        /// # Parameters
        /// - `render_local_data`: Mutable state for render optimization
        /// - `skip_flush`: Mutable reference to control flush behavior
        /// - `render_op_ir`: The specific IR operation to execute
        /// - `window_size`: Current terminal window dimensions
        /// - `locked_output_device`: Locked terminal output for thread-safe writing
        /// - `is_mock`: Whether this is a mock execution for testing
        pub fn route_paint_render_op_ir_to_backend(
            render_local_data: &mut RenderOpsLocalData,
            skip_flush: &mut bool,
            render_op_ir: &RenderOpIR,
            window_size: Size,
            locked_output_device: LockedOutputDevice<'_>,
            is_mock: bool,
        ) {
            match TERMINAL_LIB_BACKEND {
                TerminalLibBackend::Crossterm => {
                    // Convert RenderOpIR to something the paint method can understand.
                    // For now, we'll implement this in Phase 5+ when we handle the
                    // compositor. This is a placeholder that will be
                    // filled in later.
                    match render_op_ir {
                        RenderOpIR::Common(common_op) => {
                            PaintRenderOpImplCrossterm {}.paint_common(
                                skip_flush,
                                common_op,
                                window_size,
                                render_local_data,
                                locked_output_device,
                                is_mock,
                            );
                        }
                        RenderOpIR::PaintTextWithAttributes(text, style) => {
                            // IR-level text painting with clipping handled by Compositor
                            // The Compositor has already applied clipping, so we just
                            // paint the text as-is using the
                            // unified renderer.
                            PaintRenderOpImplCrossterm::paint_text_with_attributes(
                                text,
                                *style,
                                window_size,
                                render_local_data,
                                locked_output_device,
                            );
                        }
                    }
                }
                TerminalLibBackend::Termion => unimplemented!(),
            }
        }
    }

    impl Deref for RenderOpsIR {
        type Target = InlineVec<RenderOpIR>;

        fn deref(&self) -> &Self::Target { &self.list }
    }

    impl DerefMut for RenderOpsIR {
        fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
    }

    impl AddAssign<RenderOpIR> for RenderOpsIR {
        fn add_assign(&mut self, rhs: RenderOpIR) { self.list.push(rhs); }
    }

    impl Debug for RenderOpsIR {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            const DELIM: &str = "\n  - ";

            let mut iter = self.iter();

            // We don't care about the result of this operation.
            f.write_str("RenderOpsIR.len(): ").ok();
            write!(f, "{}", self.list.len()).ok();

            // First line.
            if let Some(first) = iter.next() {
                // We don't care about the result of this operation.
                f.write_str("[").ok();
                write!(f, "{first:?}").ok();
                f.write_str("]").ok();
            }

            // Subsequent lines.
            for item in iter {
                // We don't care about the result of this operation.
                f.write_str(DELIM).ok();
                f.write_str("[").ok();
                write!(f, "{item:?}").ok();
                f.write_str("]").ok();
            }

            ok!()
        }
    }
}

impl Deref for RenderOpsOutput {
    type Target = InlineVec<RenderOpOutput>;

    fn deref(&self) -> &Self::Target { &self.list }
}

impl DerefMut for RenderOpsOutput {
    fn deref_mut(&mut self) -> &mut Self::Target { &mut self.list }
}

impl AddAssign<RenderOpOutput> for RenderOpsOutput {
    fn add_assign(&mut self, rhs: RenderOpOutput) { self.list.push(rhs); }
}

impl Debug for RenderOpsOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const DELIM: &str = "\n  - ";

        let mut iter = self.iter();

        // We don't care about the result of this operation.
        f.write_str("RenderOpsOutput.len(): ").ok();
        write!(f, "{}", self.list.len()).ok();

        // First line.
        if let Some(first) = iter.next() {
            // We don't care about the result of this operation.
            f.write_str("[").ok();
            write!(f, "{first:?}").ok();
            f.write_str("]").ok();
        }

        // Subsequent lines.
        for item in iter {
            // We don't care about the result of this operation.
            f.write_str(DELIM).ok();
            f.write_str("[").ok();
            write!(f, "{item:?}").ok();
            f.write_str("]").ok();
        }

        ok!()
    }
}

/// Common render operations used in both IR (app/component) and Output (backend)
/// contexts.
///
/// These 27 operations are shared between the Intermediate Representation (IR) layer
/// and the Terminal Output (Output) layer. They work identically in both contexts.
///
/// # Architecture
///
/// The rendering pipeline has distinct stages:
/// 1. **App Layer** → Components create `RenderOpIR` (with clipping)
/// 2. **Compositor** → Processes `RenderOpIR`, writes to `OffscreenBuffer`
/// 3. **Backend Converter** → Scans `OffscreenBuffer`, creates `RenderOpOutput` (no
///    clipping needed)
/// 4. **Terminal Executor** → Executes `RenderOpOutput` on terminal
///
/// This enum contains operations that appear in all these stages without change.
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
    /// Uses [`super::sanitize_and_save_abs_pos`] to clean up the given position.
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

/// Intermediate Representation operations for app/component layer.
///
/// These operations are used by components and the app layer to describe
/// high-level rendering operations. They get processed by the compositor
/// to populate the offscreen buffer.
///
/// # Type Safety
///
/// This enum type ensures that only IR-appropriate operations are used
/// in component rendering code. Operations like `PaintTextWithAttributes`
/// (which handles clipping) are IR-specific and cannot be accidentally
/// used in backend code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpIR {
    /// Shared operation used identically in IR and Output contexts.
    Common(RenderOpCommon),

    /// Paint text with attributes (handles clipping, Unicode, emoji).
    ///
    /// This operation is used by components to render text with styling.
    /// The compositor is responsible for:
    /// - Clipping text to available terminal width
    /// - Handling Unicode and emoji display width
    /// - Applying styles correctly
    ///
    /// This is the **IR-specific** variant. The backend converter
    /// generates `CompositorNoClipTruncPaintTextWithAttributes` after
    /// clipping has been done by the compositor.
    PaintTextWithAttributes(InlineString, Option<TuiStyle>),
}

/// Terminal output operations for backend/execution layer.
///
/// These operations are optimized for terminal execution. They are generated
/// by backend converters (e.g., OffscreenBufferPaint) after processing the IR
/// and don't require additional clipping or validation.
///
/// # Type Safety
///
/// This enum type ensures that only Output-appropriate operations are used
/// in backend code. Operations like `CompositorNoClipTruncPaintTextWithAttributes`
/// (which assumes clipping is already done) cannot be accidentally used in
/// component code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpOutput {
    /// Shared operation used identically in IR and Output contexts.
    Common(RenderOpCommon),

    /// Paint text without clipping/truncation (already handled by compositor).
    ///
    /// **Internal use only** - this operation is used by backend converters
    /// after the OffscreenBuffer has been fully processed. The compositor has
    /// already handled:
    /// - Clipping text to available width
    /// - Unicode and emoji display width
    /// - Style application
    ///
    /// The backend just needs to paint the result as-is to the terminal.
    CompositorNoClipTruncPaintTextWithAttributes(InlineString, Option<TuiStyle>),
}

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

/// Collection of IR-level render operations from app/component layer.
///
/// This type wraps `RenderOpIR` values and provides ergonomic collection methods.
/// Used throughout the app/component layer and passed to the compositor.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RenderOpsIR {
    pub list: InlineVec<RenderOpIR>,
}

impl RenderOpsIR {
    /// Create a new empty collection of IR operations.
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, op: RenderOpIR) { self.list.push(op); }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpIR>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpIR> { self.list.iter() }
}

impl From<Vec<RenderOpIR>> for RenderOpsIR {
    fn from(ops: Vec<RenderOpIR>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpIR> for RenderOpsIR {
    fn from_iter<I: IntoIterator<Item = RenderOpIR>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}

/// Collection of terminal output operations for backend rendering.
///
/// This type wraps `RenderOpOutput` values and provides ergonomic collection methods.
/// Used by backend converters and the terminal execution layer.
#[derive(Clone, Default, PartialEq, Eq)]
pub struct RenderOpsOutput {
    pub list: InlineVec<RenderOpOutput>,
}

impl RenderOpsOutput {
    /// Create a new empty collection of output operations.
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    /// Add a single operation to the collection.
    pub fn push(&mut self, op: RenderOpOutput) { self.list.push(op); }

    /// Add multiple operations to the collection.
    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpOutput>) {
        self.list.extend(ops);
    }

    /// Get the number of operations in the collection.
    pub fn len(&self) -> usize { self.list.len() }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool { self.list.is_empty() }

    /// Iterate over the operations.
    pub fn iter(&self) -> impl Iterator<Item = &RenderOpOutput> { self.list.iter() }
}

impl From<Vec<RenderOpOutput>> for RenderOpsOutput {
    fn from(ops: Vec<RenderOpOutput>) -> Self { Self { list: ops.into() } }
}

impl FromIterator<RenderOpOutput> for RenderOpsOutput {
    fn from_iter<I: IntoIterator<Item = RenderOpOutput>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}


#[derive(Debug, Clone, Copy)]
pub enum FlushKind {
    JustFlush,
    ClearBeforeFlush,
}

/// Trait for controlling terminal output flushing behavior.
///
/// This trait provides methods to flush pending terminal output and optionally
/// clear the terminal before flushing. Essential for ensuring that render
/// operations are actually displayed on the terminal.
pub trait Flush {
    /// Flushes pending output to the terminal.
    ///
    /// This method ensures that all buffered terminal output is written
    /// and displayed immediately.
    fn flush(&mut self, locked_output_device: LockedOutputDevice<'_>);

    /// Clears the terminal before flushing output.
    ///
    /// This method first clears the terminal screen, then flushes any
    /// pending output. Useful for ensuring a clean display state.
    fn clear_before_flush(&mut self, locked_output_device: LockedOutputDevice<'_>);
}

/// Trait for formatting [`RenderOpCommon`] instances in debug output.
///
/// This trait abstracts debug formatting logic, allowing different
/// terminal backends to provide their own specialized debug representations
/// of common render operations.
pub trait DebugFormatRenderOp {
    /// Formats the `RenderOpCommon` for debug output.
    ///
    /// # Errors
    ///
    /// Returns a formatting error if writing to the formatter fails.
    fn fmt_debug(&self, this: &RenderOpCommon, f: &mut Formatter<'_>) -> Result;
}
