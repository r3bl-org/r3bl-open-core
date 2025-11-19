// Copyright (c) 2022-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! # Pipeline Stage 3: Compositor
//!
//! # You Are Here: **Stage 3** (Rendering to Buffer)
//!
//! ```text
//! [Stage 1: App/Component]
//!   ↓
//! [Stage 2: Pipeline]
//!   ↓
//! [Stage 3: Compositor] ← YOU ARE HERE
//!   ↓
//! [Stage 4: Backend Converter]
//!   ↓
//! [Stage 5: Backend Executor]
//!   ↓
//! [Stage 6: Terminal]
//! ```
//!
//! **Input**: [`RenderPipeline`] organized by Z-order
//! **Output**: [`OffscreenBuffer`] (virtual 2D terminal grid with styled pixels)
//! **Role**: Execute rendering operations to create a frame-sized virtual terminal buffer
//!
//! <div class="warning">
//!
//! **For the complete 6-stage rendering pipeline with visual diagrams and stage
//! reference table**, see the [rendering pipeline overview].
//!
//! </div>
//!
//! ## Navigation
//! - **Previous stage**: [`render_pipeline`] (Stage 2: Organization)
//! - **Next stage (Stage 4: Shared)**: [`offscreen_buffer::paint_impl` mod docs] (Backend
//!   Converter - shared by both Crossterm and DirectToAnsi)
//! - **Stage 5 options**:
//!   - [`crossterm_backend::crossterm_paint_render_op_impl` mod docs] (Crossterm
//!     executor)
//!   - [`direct_to_ansi` mod docs] (DirectToAnsi executor - generates ANSI escape
//!     sequences)
//!
//! ## What This Stage Does
//!
//! The Compositor is the **rendering engine** of R3BL TUI. It processes the organized
//! render operations from the pipeline and writes the results to an offscreen buffer—a 2D
//! grid representing the entire terminal screen.
//!
//! ### Key Responsibilities
//! - **Execute Operations**: Process [`RenderOpIR`] sequentially in Z-order
//! - **Apply Clipping**: Respect bounds to prevent off-screen writes
//! - **Handle Unicode**: Correctly calculate display width for emoji and wide characters
//! - **Manage State**: Track cursor position, colors, and terminal modes
//! - **Write Pixels**: Store styled characters in the [`OffscreenBuffer`]
//!
//! ### The Offscreen Buffer
//! The output is a 2D grid (not a stream) where each cell contains:
//! - The character to display
//! - Foreground color
//! - Background color
//! - Text attributes (bold, italic, underline, etc.)
//!
//! ### Diff Optimization
//! The buffer can be compared with the previous frame to determine what changed,
//! enabling selective redraw in the next stage.
//!
//! [`crossterm_backend::crossterm_paint_render_op_impl` mod docs]: mod@crate::tui::terminal_lib_backends::crossterm_backend::crossterm_paint_render_op_impl
//! [`direct_to_ansi` mod docs]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
//! [`offscreen_buffer::paint_impl` mod docs]: mod@crate::tui::terminal_lib_backends::offscreen_buffer::paint_impl
//! [`render_pipeline`]: mod@crate::tui::terminal_lib_backends::render_pipeline
//! [rendering pipeline overview]: mod@crate::terminal_lib_backends#rendering-pipeline-architecture

use super::{AlternateScreenState, BracketedPasteState, MouseTrackingState,
            OffscreenBuffer, RawModeState, RenderOpCommon, RenderOpIR, RenderPipeline,
            sanitize_and_save_abs_pos};
use crate::{ColWidth, CommonError, CommonErrorType, CommonResult, DEBUG_TUI_COMPOSITOR,
            GCStringOwned, MemoizedLenMap, PixelChar, PixelCharLine, Pos,
            RenderOpsLocalData, Size, StringLength, TuiStyle, ZOrder, ch,
            glyphs::{self, SPACER_GLYPH},
            inline_string, usize, width};

impl RenderPipeline {
    /// Convert the render pipeline to an offscreen buffer.
    ///
    /// 1. This does not require any specific implementation of crossterm or termion.
    /// 2. This is the intermediate representation (IR) of a [`RenderPipeline`]. In order
    ///    to turn this IR into actual paint commands for the terminal, you must use the
    ///    [`super::OffscreenBufferPaint`] trait implementations.
    pub fn compose_render_ops_into_ofs_buf(
        &self,
        window_size: Size,
        ofs_buf: &mut OffscreenBuffer, /* Pass in the locked buffer. */
        memoized_len_map: &mut MemoizedLenMap, /* Memoized text width calculations. */
    ) {
        let mut render_local_data = RenderOpsLocalData::default();

        for z_order in &ZOrder::get_render_order() {
            if let Some(render_ops_vec) = self.get(z_order) {
                for render_ops in render_ops_vec {
                    for render_op in render_ops.iter() {
                        process_render_op(
                            render_op,
                            window_size,
                            ofs_buf,
                            &mut render_local_data,
                            memoized_len_map,
                        );
                    }
                }
            }
        }

        DEBUG_TUI_COMPOSITOR.then(|| {
            // % is Display, ? is Debug.
            tracing::info!(
                message = %inline_string!("offscreen_buffer {ch}", ch = glyphs::SCREEN_BUFFER_GLYPH),
                offscreen_buffer = ?ofs_buf
            );
        });
    }
}

#[allow(clippy::too_many_lines)]
pub fn process_render_op(
    render_op_ir: &RenderOpIR,
    window_size: Size,
    ofs_buf: &mut OffscreenBuffer,
    render_local_data: &mut RenderOpsLocalData,
    memoized_len_map: &mut MemoizedLenMap,
) {
    match render_op_ir {
        // IR-specific: PaintTextWithAttributes handles clipping and Unicode width
        RenderOpIR::PaintTextWithAttributes(arg_text_ref, maybe_style_ref) => {
            let result_new_pos = print_text_with_attributes(
                arg_text_ref,
                maybe_style_ref.as_ref(),
                ofs_buf,
                None,
                render_local_data,
            );
            if let Ok(new_pos) = result_new_pos {
                ofs_buf.cursor_pos =
                    sanitize_and_save_abs_pos(new_pos, window_size, render_local_data);
            }
        }
        // Common operations: shared between IR and Output
        RenderOpIR::Common(common_op) => {
            process_common_render_op(
                common_op,
                window_size,
                ofs_buf,
                render_local_data,
                memoized_len_map,
            );
        }
    }
}

/// Process a common render operation in the offscreen buffer.
///
/// These operations work identically in both IR and Output contexts.
#[allow(clippy::too_many_lines)]
fn process_common_render_op(
    common_op: &RenderOpCommon,
    window_size: Size,
    ofs_buf: &mut OffscreenBuffer,
    render_local_data: &mut RenderOpsLocalData,
    memoized_len_map: &mut MemoizedLenMap,
) {
    match common_op {
        // ===== Terminal Mode State Operations =====
        // These operations update the OffscreenBuffer's terminal mode state while also
        // being executed by the terminal backend to affect actual terminal behavior.
        RenderOpCommon::EnterRawMode => {
            ofs_buf.terminal_mode.raw_mode = RawModeState::Enabled;
        }
        RenderOpCommon::ExitRawMode => {
            ofs_buf.terminal_mode.raw_mode = RawModeState::Disabled;
        }
        RenderOpCommon::EnterAlternateScreen => {
            ofs_buf.terminal_mode.alternate_screen = AlternateScreenState::Active;
        }
        RenderOpCommon::ExitAlternateScreen => {
            ofs_buf.terminal_mode.alternate_screen = AlternateScreenState::Inactive;
        }
        RenderOpCommon::EnableMouseTracking => {
            ofs_buf.terminal_mode.mouse_tracking = MouseTrackingState::Enabled;
        }
        RenderOpCommon::DisableMouseTracking => {
            ofs_buf.terminal_mode.mouse_tracking = MouseTrackingState::Disabled;
        }
        RenderOpCommon::EnableBracketedPaste => {
            ofs_buf.terminal_mode.bracketed_paste = BracketedPasteState::Enabled;
        }
        RenderOpCommon::DisableBracketedPaste => {
            ofs_buf.terminal_mode.bracketed_paste = BracketedPasteState::Disabled;
        }
        // ===== Incremental Rendering Operations - Complete implementation
        // These operations are executed both by the backend AND need to
        // update buffer state for consistency and future extensibility (e.g., if
        // choose() or readline_async migrates to use OffscreenBuffer).
        RenderOpCommon::MoveCursorToColumn(col_index) => {
            ofs_buf.cursor_pos.col_index = *col_index;
        }
        RenderOpCommon::MoveCursorToNextLine(row_height) => {
            ofs_buf.cursor_pos.row_index += *row_height;
            ofs_buf.cursor_pos.col_index = crate::col(0);
        }
        RenderOpCommon::MoveCursorToPreviousLine(row_height) => {
            ofs_buf.cursor_pos.row_index -= *row_height;
            ofs_buf.cursor_pos.col_index = crate::col(0);
        }
        RenderOpCommon::ClearCurrentLine => {
            // Clear the current line in the buffer
            let row_idx = ofs_buf.cursor_pos.row_index.as_usize();
            if let Some(line) = ofs_buf.buffer.get_mut(row_idx) {
                for pixel_char in line.iter_mut() {
                    *pixel_char = PixelChar::Spacer;
                }
            }
        }
        RenderOpCommon::ClearToEndOfLine => {
            // Clear from cursor position to end of current line
            let row_idx = ofs_buf.cursor_pos.row_index.as_usize();
            let col_idx = ofs_buf.cursor_pos.col_index.as_usize();
            if let Some(line) = ofs_buf.buffer.get_mut(row_idx) {
                for col in col_idx..line.len() {
                    if let Some(pixel_char) = line.get_mut(col) {
                        *pixel_char = PixelChar::Spacer;
                    }
                }
            }
        }
        RenderOpCommon::ClearToStartOfLine => {
            // Clear from start of current line to cursor position (inclusive)
            let row_idx = ofs_buf.cursor_pos.row_index.as_usize();
            let col_idx = ofs_buf.cursor_pos.col_index.as_usize();
            if let Some(line) = ofs_buf.buffer.get_mut(row_idx) {
                for col in 0..=col_idx {
                    if let Some(pixel_char) = line.get_mut(col) {
                        *pixel_char = PixelChar::Spacer;
                    }
                }
            }
        }
        RenderOpCommon::PrintStyledText(text) => {
            // This operation is for pre-styled text that already contains ANSI escape
            // codes. We update the cursor position based on the visible character width
            // AFTER stripping ANSI codes, but don't try to parse the ANSI codes back
            // into PixelChar styles (that would be backwards). The actual
            // buffer content update happens at the backend level
            // when the ANSI sequences are interpreted.
            //
            // This approach ensures the cursor position stays in sync with actual output,
            // which is important for consistency and future use cases (e.g., if choose()
            // or readline_async migrates to use OffscreenBuffer).

            // Calculate visible text width using memoized ANSI stripping (70x speedup).
            // StringLength::StripAnsi strips ANSI codes and calculates Unicode width,
            // with memoization for repeated text patterns.
            let text_width =
                StringLength::StripAnsi.calculate(text.as_str(), memoized_len_map);
            ofs_buf.cursor_pos.col_index += text_width;

            // Validate and clamp cursor position to window bounds
            sanitize_and_save_abs_pos(ofs_buf.cursor_pos, window_size, render_local_data);
        }
        // Terminal-only state operations and no-op operations - no buffer effect needed
        RenderOpCommon::Noop
        | RenderOpCommon::ShowCursor
        | RenderOpCommon::HideCursor
        | RenderOpCommon::SaveCursorPosition
        | RenderOpCommon::RestoreCursorPosition => {}
        // Do process these.
        RenderOpCommon::ClearScreen => {
            ofs_buf.clear();
        }
        RenderOpCommon::MoveCursorPositionAbs(new_abs_pos) => {
            ofs_buf.cursor_pos =
                sanitize_and_save_abs_pos(*new_abs_pos, window_size, render_local_data);
        }
        RenderOpCommon::MoveCursorPositionRelTo(
            box_origin_pos_ref,
            content_rel_pos_ref,
        ) => {
            let new_abs_pos = *box_origin_pos_ref + *content_rel_pos_ref;
            ofs_buf.cursor_pos =
                sanitize_and_save_abs_pos(new_abs_pos, window_size, render_local_data);
        }
        RenderOpCommon::SetFgColor(fg_color_ref) => {
            render_local_data.fg_color = Some(*fg_color_ref);
        }
        RenderOpCommon::SetBgColor(bg_color_ref) => {
            render_local_data.bg_color = Some(*bg_color_ref);
        }
        RenderOpCommon::ResetColor => {
            render_local_data.fg_color = None;
            render_local_data.bg_color = None;
        }
        RenderOpCommon::ApplyColors(maybe_style_ref) => {
            if let Some(style_ref) = maybe_style_ref {
                render_local_data.fg_color = style_ref.color_fg;
                render_local_data.bg_color = style_ref.color_bg;
            }
        }
    }
}

/// Render text with optional styling to an offscreen buffer with Unicode-aware handling.
///
/// This function is the core text rendering primitive for the TUI system, responsible for
/// converting text strings into [`PixelChar`] elements in the offscreen buffer. It
/// handles both plain text and styled text, with comprehensive support for Unicode
/// grapheme clusters, wide characters (like emoji), and terminal-specific display width
/// calculations.
///
/// # Core Functionality
///
/// - **Unicode-safe rendering**: Proper handling of grapheme clusters, emoji, and wide
///   characters
/// - **Style composition**: Merges provided styles with buffer-level colors
/// - **Intelligent clipping**: Two-stage clipping system for optimal text fitting
/// - **Position tracking**: Updates buffer position based on actual rendered width
/// - **Wide character support**: Handles multi-column characters with void padding
///
/// # Parameters
///
/// This will modify the `my_offscreen_buffer` argument. For plain text it supports
/// counting [`crate::Seg`]s. The display width of each segment is
/// taken into account when filling the offscreen buffer.
///
/// # Clipping behavior
///
/// This diagram shows what happens per line of text. Each line can be found here:
/// `my_offscreen_buffer[my_pos.row_index]`.
///
/// The function uses a two-stage clipping system:
///
/// 1. **Parameter clipping**: If `maybe_max_display_col_count` is provided, text is
///    clipped to fit.
/// 2. **Window clipping**: Text is further clipped to fit within the actual window
///    boundaries.
///
/// This ensures text never overflows the visible area while respecting explicit width
/// constraints.
///
/// ```text
///             my_pos.col_index
///             ↓
/// C0123456789012345678901234567890123456789012345678901234567890
///             <------------------ usable space ----------------->
/// <---------------- maybe_max_display_col_count ---------------->
/// ```
/// # Returns
///
/// Returns `Ok(Pos)` with the new cursor position after rendering. Column overflow is
/// handled gracefully by stopping rendering.
///
/// # Errors
///
/// Returns [`CommonErrorType::DisplaySizeTooSmall`] if the target row index exceeds
/// the offscreen buffer's available rows (i.e., when
/// `ofs_buf.my_pos.row_index` is greater than or equal to the number of rows
/// in `ofs_buf.buffer`).
pub fn print_text_with_attributes(
    string: &str,
    maybe_style_ref: Option<&TuiStyle>,
    ofs_buf: &mut OffscreenBuffer,
    maybe_max_display_col_count: Option<ColWidth>,
    render_local_data: &RenderOpsLocalData,
) -> CommonResult<Pos> {
    // Get col and row index from `my_pos`.
    let display_col_index = usize(ofs_buf.cursor_pos.col_index);
    let display_row_index = usize(ofs_buf.cursor_pos.row_index);

    // Clip text to bounds using helper function.
    let text_gcs = print_text_with_attributes_helper::clip_text_to_bounds(
        string,
        display_col_index,
        maybe_max_display_col_count,
        ofs_buf.window_size.col_width,
    );

    // Try to get the line at `row_index`.
    let mut line_copy = {
        if let Some(line) = ofs_buf.buffer.get(display_row_index) {
            Ok(line.clone())
        } else {
            // Clip vertically.
            CommonError::new_error_result_with_only_type(
                CommonErrorType::DisplaySizeTooSmall,
            )
        }
    }?;

    DEBUG_TUI_COMPOSITOR.then(|| {
            // % is Display, ? is Debug.
            tracing::info! {
            message = %inline_string!(
                "print_plain_text() {ar} {ch}",
                ar = glyphs::RIGHT_ARROW_GLYPH,
                ch = glyphs::PAINT_GLYPH,
            ),
            details = %inline_string!(
                "insertion at: display_row_index: {a}, display_col_index: {b}, window_size: {c:?},
                text: '{d}',
                width: {e:?}",
                a = display_row_index,
                b = display_col_index,
                c = ofs_buf.window_size,
                d = text_gcs.string,
                e = text_gcs.display_width,
            )
        };
    });

    // Insert clipped `text_ref_gcs` into `line` at `insertion_col_index`. Ok to use
    // `line_copy[insertion_col_index]` syntax because we know that row and col
    // indices are valid.
    let insertion_col_index = display_col_index;

    // Compose style using helper function.
    let maybe_style = print_text_with_attributes_helper::compose_style(
        maybe_style_ref,
        render_local_data,
    );

    DEBUG_TUI_COMPOSITOR.then(|| {
        // % is Display, ? is Debug.
        tracing::debug!(
            message = %match maybe_style {
                Some(style) => {
                    inline_string!(
                        "{ch} [row: {row}, col: {col}] - style: {style:?}",
                        ch = glyphs::BOX_FILL_GLYPH,
                        row = display_row_index,
                        col = display_col_index,
                        style = style
                    )
                }
                None => {
                    inline_string!(
                        "{ch} [row: {row}, col: {col}] - style: None",
                        ch = glyphs::BOX_EMPTY_GLYPH,
                        row = display_row_index,
                        col = display_col_index,
                    )
                }
            }
        );
    });

    // Process character segments using helper function.
    let (updated_insertion_col_index, mut already_inserted_display_width) =
        print_text_with_attributes_helper::process_character_segments(
            &text_gcs,
            maybe_style,
            &mut line_copy,
            insertion_col_index,
        );

    // Add spacer padding using helper function.
    already_inserted_display_width =
        print_text_with_attributes_helper::add_spacer_padding(
            &mut line_copy,
            updated_insertion_col_index,
            already_inserted_display_width,
            display_col_index,
            maybe_max_display_col_count,
        );

    // Mimic what stdout does and move the position.col_index forward by the width of
    // the text that was added to display.
    let new_pos = ofs_buf.cursor_pos.add_col(already_inserted_display_width);

    // Replace the line in `my_offscreen_buffer` with the new line.
    ofs_buf.buffer[display_row_index] = line_copy;

    Ok(new_pos)
}

mod print_text_with_attributes_helper {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Clips the input string based on max display column count and window bounds.
    /// Returns the final clipped string as a grapheme cluster string.
    ///
    /// # Performance Considerations
    ///
    /// This function can be a significant performance bottleneck in the rendering
    /// pipeline, as it is called for every text rendering operation. The optimization
    /// implemented here uses a fast-path approach to minimize `GCStringOwned`
    /// allocations:
    ///
    /// - **Fast path**: Uses `GCStringOwned::width()` to check string width without
    ///   creating a `GCStringOwned` instance
    /// - **Early return**: When text fits within bounds, creates `GCStringOwned` only
    ///   once
    /// - **Slow path**: Only creates `GCStringOwned` for truncation when absolutely
    ///   necessary
    ///
    /// This optimization addresses the performance bottleneck identified in flamegraph
    /// analysis, where `GCStringOwned` creation in the rendering pipeline was consuming
    /// 8.61% of total execution time. See `/docs/tui_perf_optimize.md` for detailed
    /// performance analysis and optimization results.
    ///
    /// # Benchmarks
    ///
    /// Performance impact is measured by comprehensive benchmarks in the `bench_tests`
    /// module at the bottom of this file. The `bench_tests` module contains the original
    /// implementation (`clip_text_to_bounds_old`) that serves as the baseline for
    /// performance comparison against this optimized version. The benchmarks show
    /// significant improvements:
    /// - No clipping needed: 64.8% faster (2.84x speedup)
    /// - With clipping: 7.8% faster
    /// - Unicode content: 19.3% faster
    /// - Repeated calls: 32.1% faster
    pub fn clip_text_to_bounds(
        string: &str,
        display_col_index: usize,
        maybe_max_display_col_count: Option<ColWidth>,
        window_max_display_col_count: ColWidth,
    ) -> GCStringOwned {
        // Fast path: calculate string width without creating GCStringOwned.
        let string_width = GCStringOwned::from(string).width();

        // Calculate the effective max width considering parameter and window constraints.
        let param_max = maybe_max_display_col_count
            .map_or(*string_width, |max| *max - ch(display_col_index));
        let window_max = *window_max_display_col_count - ch(display_col_index);
        let effective_max = param_max.min(window_max);

        // If the string already fits, create and return GCStringOwned only once.
        if *string_width <= effective_max {
            return string.into();
        }

        // Slow path: create GCStringOwned for truncation only when necessary.
        let string_gcs = GCStringOwned::from(string);
        let truncated_str = string_gcs.trunc_end_to_fit(width(effective_max));
        truncated_str.into()
    }

    /// Composes the final style by merging provided style with local colors.
    pub fn compose_style(
        maybe_style_ref: Option<&TuiStyle>,
        render_local_data: &RenderOpsLocalData,
    ) -> Option<TuiStyle> {
        if let Some(maybe_style) = maybe_style_ref {
            // We get the attributes from `maybe_style_ref`.
            let mut it = *maybe_style;
            // We get the colors from local render data.
            it.color_fg = render_local_data.fg_color;
            it.color_bg = render_local_data.bg_color;
            Some(it)
        } else if render_local_data.fg_color.is_some()
            || render_local_data.bg_color.is_some()
        {
            Some(TuiStyle {
                color_fg: render_local_data.fg_color,
                color_bg: render_local_data.bg_color,
                ..Default::default()
            })
        } else {
            None
        }
    }

    /// Processes and renders individual character segments to the line buffer.
    /// Returns the updated insertion column index and total inserted display width.
    pub fn process_character_segments(
        text_gcs: &GCStringOwned,
        maybe_style: Option<TuiStyle>,
        line_copy: &mut PixelCharLine,
        mut insertion_col_index: usize,
    ) -> (usize, crate::ChUnit) {
        let mut already_inserted_display_width = ch(0);

        // Loop over each grapheme cluster segment (the character) in `text_ref_gcs` (text
        // in a line). For each GraphemeClusterSegment, create a PixelChar.
        for seg in text_gcs.seg_iter() {
            let segment_display_width = usize(*seg.display_width);
            if segment_display_width == 0 {
                continue;
            }

            // Set the `PixelChar` at `insertion_col_index`.
            if line_copy.get(insertion_col_index).is_some() {
                let pixel_char = {
                    let seg_text: &str = seg.get_str(text_gcs);
                    if let (None, SPACER_GLYPH) = (maybe_style, seg_text) {
                        PixelChar::Spacer
                    } else {
                        // Convert the segment text to a single char.
                        let display_char = if seg_text.chars().count() == 1 {
                            seg_text.chars().next().unwrap()
                        } else {
                            // For multi-char segments, use the first char.
                            seg_text
                                .chars()
                                .next()
                                .unwrap_or(crate::UNICODE_REPLACEMENT_CHAR)
                        };
                        PixelChar::PlainText {
                            display_char,
                            style: maybe_style.unwrap_or_default(),
                        }
                    }
                };

                if line_copy.get(insertion_col_index).is_some() {
                    line_copy[insertion_col_index] = pixel_char;
                }

                // Deal w/ the display width of the `PixelChar` > 1. This is the
                // equivalent of `jump_cursor()` in PaintRenderOpImplCrossterm.
                //
                // Move cursor "manually" to cover "extra" (display) width of a single
                // character. This is a necessary precautionary measure, to make
                // sure the behavior is the same on all terminals. In practice
                // this means that terminals will be "broken" in the same way
                // across multiple terminal emulators and OSes.
                // 1. Terminals vary in their support of complex grapheme clusters (joined
                //    emoji). This code uses the crate unicode_width to display a given
                //    UTF-8 character "correctly" in all terminals. The number reported by
                //    this crate and the actual display width that the specific terminal
                //    emulator + OS combo will display may be different.
                // 2. This means that in some terminals, the caret itself has to be
                //    manually "jumped" to covert the special case of a really wide UTF-8
                //    character. This happens by adding Void pixel chars.
                // 3. The insertion_col_index is calculated & updated based on the
                //    unicode_width crate values.
                let segment_display_width = usize(*seg.display_width);
                if segment_display_width > 1 {
                    // Deal w/ `gc_segment` display width that is > 1 => pad w/ Void.
                    let num_of_extra_display_cols_to_inject_void_into =
                        segment_display_width - 1; // Safe subtract.
                    for _ in 0..num_of_extra_display_cols_to_inject_void_into {
                        // Make sure insertion_col_index is safe to access.
                        if line_copy.get(insertion_col_index + 1).is_some() {
                            // Move insertion_col_index forward & inject a.
                            // PixelChar::Void.
                            insertion_col_index += 1;
                            line_copy[insertion_col_index] = PixelChar::Void;
                        }
                    }
                    // Move insertion_col_index forward.
                    insertion_col_index += 1;
                } else {
                    // `gc_segment` width is 1 => move `insertion_col_index` forward by 1.
                    insertion_col_index += 1;
                }

                already_inserted_display_width += *seg.display_width;
            } else {
                // Run out of space in the line of the offscreen buffer.
                break;
            }
        }

        (insertion_col_index, already_inserted_display_width)
    }

    /// Adds spacer padding to the end of the line if max display column count is
    /// specified.
    pub fn add_spacer_padding(
        line_copy: &mut PixelCharLine,
        mut insertion_col_index: usize,
        mut already_inserted_display_width: crate::ChUnit,
        display_col_index: usize,
        maybe_max_display_col_count: Option<ColWidth>,
    ) -> crate::ChUnit {
        // 🥊Deal w/ padding SPACERs padding to end of line (if
        // `maybe_max_display_col_count` is some).
        if let Some(max_display_col_count) = maybe_max_display_col_count {
            let adj_max = *max_display_col_count - ch(display_col_index);
            while already_inserted_display_width < adj_max {
                if line_copy.get(insertion_col_index).is_some() {
                    line_copy[insertion_col_index] = PixelChar::Spacer;
                    insertion_col_index += 1;
                    already_inserted_display_width += ch(1);
                } else {
                    break;
                }
            }
        }
        already_inserted_display_width
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assert_eq2, col, height, new_style, render_pipeline, row, tui_color};
    use std::collections::HashMap;

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_print_plain_text_render_path_reuse_buffer() {
        let window_size = width(10) + height(2);
        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);

        // Input:  R0 "hello12345😃"
        //            C0123456789
        // Output: R0 "hello12345"
        //            C0123456789
        {
            let text = "hello12345😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!(dim bold italic color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold italic color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold italic color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold italic color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][9],
                PixelChar::PlainText {
                    display_char: '5',
                    style: new_style!(dim bold italic color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
        }

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        {
            let text = "hello1234😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Spacer);
        }
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn test_print_plain_text_render_path_new_buffer_for_each_paint() {
        let window_size = width(10) + height(2);

        // Input:  R0 "hello12345😃"
        //            C0123456789
        // Output: R0 "hello12345"
        //            C0123456789
        {
            let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
            let text = "hello12345😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][9],
                PixelChar::PlainText {
                    display_char: '5',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
        }

        // Input:  R0 "hello1234😃"
        //            C0123456789
        // Output: R0 "hello1234╳"
        //            C0123456789
        {
            let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
            let text = "hello1234😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Spacer);
        }

        // R0 "hello123😃"
        //    C0123456789
        {
            let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
            let text = "hello123😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!( dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][8],
                PixelChar::PlainText {
                    display_char: '😃',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Void);
        }

        // R0 "hello12😃"
        //    C0123456789
        {
            let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
            let text = "hello12😃";
            // The style colors should be overwritten by fg_color and bg_color.
            let maybe_style = Some(
                new_style!(dim bold color_fg:{tui_color!(cyan)} color_bg:{tui_color!(cyan)}),
            );
            ofs_buf.cursor_pos = col(0) + row(0);
            let render_local_data = RenderOpsLocalData {
                fg_color: Some(tui_color!(green)),
                bg_color: Some(tui_color!(blue)),
                ..Default::default()
            };
            let maybe_max_display_col_count = Some(width(10));

            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                &mut ofs_buf,
                maybe_max_display_col_count,
                &render_local_data,
            )
            .ok();

            // my_offscreen_buffer:
            // window_size: [width:10, height:2],
            // row_index: [0]
            // 	0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
            // padding: 0 }) 	1: "e" Some(Style { _id + bold + dim | fg:
            // Some(green) | bg: Some(blue) | padding: 0 }) 	2: "l" Some(Style
            // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
            // padding: 0 }) 	4: "o" Some(Style { _id + bold + dim | fg:
            // Some(green) | bg: Some(blue) | padding: 0 }) 	5: "1" Some(Style
            // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
            // 	6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
            // padding: 0 }) 	7: "😃" Some(Style { _id + bold + dim | fg:
            // Some(green) | bg: Some(blue) | padding: 0 }) 	8: ❯
            // 	9: ╳
            // row_index: [1]
            // 	0: ╳.
            // 	9: ╳

            // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

            assert_eq2!(
                ofs_buf.buffer[0][0],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][4],
                PixelChar::PlainText {
                    display_char: 'o',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][5],
                PixelChar::PlainText {
                    display_char: '1',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][7],
                PixelChar::PlainText {
                    display_char: '😃',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[0][8], PixelChar::Void);
            assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Spacer);
        }
    }

    #[test]
    fn test_convert() {
        let window_size = width(10) + height(2);

        // Create a RenderPipeline.
        // render_ops:
        //     - RenderOpIRVec.len(): 10
        //       - [ResetColor]
        //       - [SetFgColor(green)]
        //       - [SetBgColor(blue)]
        //       - [MoveCursorPositionAbs([col:0, row:0])]
        //       - PrintTextWithAttributes(8 bytes, Style { _id + bold + dim | fg: None |
        //         bg: None | padding: 0 }), postfix pad to 10, ↑ "hello12😃" + "╳╳" ←
        //         postfix padding C01234567......89 ↑ This pixel char takes up 2 display
        //         cols. There are 2 extra PixelChar::Empty at display cols 8 & 9.
        //       - [ResetColor]
        let pipeline = render_pipeline!(@new ZOrder::Normal =>
            RenderOpIR::Common(RenderOpCommon::ClearScreen),
            RenderOpIR::Common(RenderOpCommon::ResetColor),
            RenderOpIR::Common(RenderOpCommon::SetFgColor(tui_color!(green))),
            RenderOpIR::Common(RenderOpCommon::SetBgColor(tui_color!(blue))),
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(0) + row(0))),
            RenderOpIR::PaintTextWithAttributes(
                "hello12😃".into(), Some(new_style!(dim bold))),
            RenderOpIR::Common(RenderOpCommon::ResetColor)
        );
        // println!("pipeline: \n{:#?}", pipeline.get_all_render_op_in(ZOrder::Normal));

        // Convert it into an OffscreenBuffer.
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //     0: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | \
        //       padding: 0 })     1: "e" Some(Style { _id + bold + dim | fg: \
        //       Some(green) | bg: Some(blue) | padding: 0 })     2: "l" Some(Style { \
        //       _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     3: "l" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | \
        //       padding: 0 })     4: "o" Some(Style { _id + bold + dim | fg: \
        //       Some(green) | bg: Some(blue) | padding: 0 })     5: "1" Some(Style { \
        //       _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //     6: "2" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) | \
        //       padding: 0 })     7: "😃" Some(Style { _id + bold + dim | fg: \
        //       Some(green) | bg: Some(blue) | padding: 0 })     8: ❯
        //     9: ╳
        // row_index: [1]
        //     0: ╳ .
        //     9: ╳

        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
        let mut memoized_len_map = HashMap::new();
        pipeline.compose_render_ops_into_ofs_buf(
            window_size,
            &mut ofs_buf,
            &mut memoized_len_map,
        );

        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);
        assert_eq2!(ofs_buf.buffer.len(), 2);
        assert_eq2!(
            ofs_buf.buffer[0][0],
            PixelChar::PlainText {
                display_char: 'h',
                style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
            }
        );
        assert_eq2!(
            ofs_buf.buffer[0][7],
            PixelChar::PlainText {
                display_char: '😃',
                style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
            }
        );
        assert_eq2!(ofs_buf.buffer[0][8], PixelChar::Void);
        assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Spacer);
    }

    #[test]
    fn test_convert_non_zero_pos() {
        let window_size = width(10) + height(2);

        // pipeline:
        // Some(
        //     [
        //         ClearScreen,
        //         ResetColor,
        //         SetFgColor(green),
        //         SetBgColor(blue),
        //         MoveCursorPositionAbs([col:2, row:0]),
        //         PrintTextWithAttributes(9 bytes, Style { _id + bold + dim | fg: None |
        // bg: None | padding: 0 }), pad to width 10 col count,
        //         ResetColor,
        //         SetFgColor(green),
        //         SetBgColor(blue),
        //         MoveCursorPositionAbs([col:4, row:1]),
        //         PrintTextWithAttributes(5 bytes, Style { _id + bold + dim | fg: None |
        // bg: None | padding: 0 }), pad to width 10 col count,
        //         ResetColor,
        //     ],
        // )
        let pipeline = render_pipeline!(@new ZOrder::Normal =>
            RenderOpIR::Common(RenderOpCommon::ClearScreen),
            RenderOpIR::Common(RenderOpCommon::ResetColor),
            RenderOpIR::Common(RenderOpCommon::SetFgColor(tui_color!(green))),
            RenderOpIR::Common(RenderOpCommon::SetBgColor(tui_color!(blue))),
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(2) + row(0))),
            RenderOpIR::PaintTextWithAttributes(
                "hello😃".into(), Some(new_style!(dim bold))),
            RenderOpIR::Common(RenderOpCommon::ResetColor),
            RenderOpIR::Common(RenderOpCommon::SetFgColor(tui_color!(green))),
            RenderOpIR::Common(RenderOpCommon::SetBgColor(tui_color!(blue))),
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(4) + row(1))),
            RenderOpIR::PaintTextWithAttributes(
                "world".into(), Some(new_style!(dim bold))),
            RenderOpIR::Common(RenderOpCommon::ResetColor),
        );
        // println!("pipeline: \n{:#?}", pipeline.get_all_render_op_in(ZOrder::Normal));

        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
        let mut memoized_len_map = HashMap::new();
        pipeline.compose_render_ops_into_ofs_buf(
            window_size,
            &mut ofs_buf,
            &mut memoized_len_map,
        );
        // my_offscreen_buffer:
        // window_size: [width:10, height:2],
        // row_index: [0]
        //   0: ╳
        //   1: ╳
        //   2: "h" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   3: "e" Some(Style { _id + bold + dim | fg: Some(green)
        // | bg: Some(blue) | padding: 0 })   4: "l" Some(Style { _id + bold + dim
        // | fg: Some(green) | bg: Some(blue) | padding: 0 })   5: "l" Some(Style
        // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   6: "o" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   7: "😃" Some(Style { _id + bold + dim | fg: Some(green)
        // | bg: Some(blue) | padding: 0 })   8: ❯
        //   9: ╳
        // row_index: [1]
        //   0: ╳
        //   1: ╳
        //   2: ╳
        //   3: ╳
        //   4: "w" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   5: "o" Some(Style { _id + bold + dim | fg: Some(green)
        // | bg: Some(blue) | padding: 0 })   6: "r" Some(Style { _id + bold + dim
        // | fg: Some(green) | bg: Some(blue) | padding: 0 })   7: "l" Some(Style
        // { _id + bold + dim | fg: Some(green) | bg: Some(blue) | padding: 0 })
        //   8: "d" Some(Style { _id + bold + dim | fg: Some(green) | bg: Some(blue) |
        // padding: 0 })   9: ╳

        // println!("my_offscreen_buffer: \n{:#?}", my_offscreen_buffer);

        // Contains 2 lines.
        assert_eq2!(ofs_buf.buffer.len(), 2);

        // Line 1 (row_index = 0).
        {
            assert_eq2!(ofs_buf.buffer[0][0], PixelChar::Spacer);
            assert_eq2!(ofs_buf.buffer[0][1], PixelChar::Spacer);
            assert_eq2!(
                ofs_buf.buffer[0][2],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[0][7],
                PixelChar::PlainText {
                    display_char: '😃',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[0][8], PixelChar::Void);
            assert_eq2!(ofs_buf.buffer[0][9], PixelChar::Spacer);
        }

        // Line 2 (row_index = 1)
        {
            assert_eq2!(ofs_buf.buffer[1][0], PixelChar::Spacer);
            assert_eq2!(ofs_buf.buffer[1][1], PixelChar::Spacer);
            assert_eq2!(ofs_buf.buffer[1][2], PixelChar::Spacer);
            assert_eq2!(ofs_buf.buffer[1][3], PixelChar::Spacer);
            assert_eq2!(
                ofs_buf.buffer[1][4],
                PixelChar::PlainText {
                    display_char: 'w',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(
                ofs_buf.buffer[1][8],
                PixelChar::PlainText {
                    display_char: 'd',
                    style: new_style!(dim bold color_fg:{tui_color!(green)} color_bg:{tui_color!(blue)}),
                }
            );
            assert_eq2!(ofs_buf.buffer[1][9], PixelChar::Spacer);
        }
    }

    #[test]
    fn test_sanitize_and_save_abs_pos() {
        let max_col = 8;
        let max_row = 2;
        let window_size = width(max_col) + height(max_row);

        let pipeline = render_pipeline!(@new ZOrder::Normal =>
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(max_col) + row(0))),
            RenderOpIR::PaintTextWithAttributes(
                "h".into(), Some(new_style! ( dim bold ))),
            RenderOpIR::Common(RenderOpCommon::ResetColor),
            RenderOpIR::Common(RenderOpCommon::MoveCursorPositionAbs(col(max_col+1) + row(1))),
            RenderOpIR::PaintTextWithAttributes(
                "i".into(), Some(new_style! ( dim bold ))),
            RenderOpIR::Common(RenderOpCommon::ResetColor)
        );

        println!(
            "pipeline: \n{:#?}",
            pipeline.get_all_render_op_in(ZOrder::Normal)
        );

        let mut ofs_buf = OffscreenBuffer::new_empty(window_size);
        let mut memoized_len_map = HashMap::new();
        pipeline.compose_render_ops_into_ofs_buf(
            window_size,
            &mut ofs_buf,
            &mut memoized_len_map,
        );
        println!(
            "ofs_buf:
{ofs_buf:#?}"
        );

        // Line 1 (row_index = 7)
        {
            assert_eq2!(
                ofs_buf.buffer[0][max_col - 1],
                PixelChar::PlainText {
                    display_char: 'h',
                    style: new_style! ( dim bold ),
                }
            );
        }
        // Line 2 (row_index = 7)
        {
            assert_eq2!(
                ofs_buf.buffer[1][max_col - 1],
                PixelChar::PlainText {
                    display_char: 'i',
                    style: new_style! ( dim bold ),
                }
            );
        }
    }
}

#[cfg(test)]
mod bench_tests {
    use super::*;

    extern crate test;
    use test::Bencher;

    mod clip_text_to_bounds_benchmarks {
        use super::{print_text_with_attributes_helper::clip_text_to_bounds, *};

        // Old implementation for comparison.
        fn clip_text_to_bounds_old(
            string: &str,
            display_col_index: usize,
            maybe_max_display_col_count: Option<ColWidth>,
            window_max_display_col_count: ColWidth,
        ) -> GCStringOwned {
            // ✂️Clip `arg_text_ref` (if needed) and make `text`.
            let string_gcs: GCStringOwned = string.into();
            let clip_1_str =
                if let Some(max_display_col_count) = maybe_max_display_col_count {
                    let adj_max = *max_display_col_count - ch(display_col_index);
                    string_gcs.trunc_end_to_fit(width(adj_max))
                } else {
                    string
                };
            let clip_1_gcs: GCStringOwned = clip_1_str.into();

            // ✂️Clip `text` (if needed) to the max display col count of the window.
            let text_fits_in_window = *clip_1_gcs.display_width
                <= *window_max_display_col_count - ch(display_col_index);
            let clip_2_str = if text_fits_in_window {
                clip_1_str
            } else {
                let adj_max = *window_max_display_col_count - ch(display_col_index);
                clip_1_gcs.trunc_end_to_fit(width(adj_max))
            };

            clip_2_str.into()
        }

        #[bench]
        fn bench_clip_text_no_clipping_new(b: &mut Bencher) {
            let text = "Hello, World!";
            b.iter(|| clip_text_to_bounds(text, 0, None, width(100)));
        }

        #[bench]
        fn bench_clip_text_no_clipping_old(b: &mut Bencher) {
            let text = "Hello, World!";
            b.iter(|| clip_text_to_bounds_old(text, 0, None, width(100)));
        }

        #[bench]
        fn bench_clip_text_with_clipping_new(b: &mut Bencher) {
            let text = "This is a very long string that needs to be clipped to fit within bounds";
            b.iter(|| clip_text_to_bounds(text, 10, Some(width(20)), width(80)));
        }

        #[bench]
        fn bench_clip_text_with_clipping_old(b: &mut Bencher) {
            let text = "This is a very long string that needs to be clipped to fit within bounds";
            b.iter(|| clip_text_to_bounds_old(text, 10, Some(width(20)), width(80)));
        }

        #[bench]
        fn bench_clip_text_unicode_new(b: &mut Bencher) {
            let text = "Hello 世界! 😀 This is a test with emoji and unicode 🚀";
            b.iter(|| clip_text_to_bounds(text, 5, Some(width(30)), width(50)));
        }

        #[bench]
        fn bench_clip_text_unicode_old(b: &mut Bencher) {
            let text = "Hello 世界! 😀 This is a test with emoji and unicode 🚀";
            b.iter(|| clip_text_to_bounds_old(text, 5, Some(width(30)), width(50)));
        }

        #[bench]
        fn bench_clip_text_repeated_calls_new(b: &mut Bencher) {
            let texts = vec![
                "Short",
                "Medium length string here",
                "This is a much longer string that will definitely need clipping",
                "Unicode: 你好世界 🌍",
                "Mixed content with numbers 12345 and symbols !@#$%",
            ];

            b.iter(|| {
                for text in &texts {
                    clip_text_to_bounds(text, 0, Some(width(20)), width(80));
                }
            });
        }

        #[bench]
        fn bench_clip_text_repeated_calls_old(b: &mut Bencher) {
            let texts = vec![
                "Short",
                "Medium length string here",
                "This is a much longer string that will definitely need clipping",
                "Unicode: 你好世界 🌍",
                "Mixed content with numbers 12345 and symbols !@#$%",
            ];

            b.iter(|| {
                for text in &texts {
                    clip_text_to_bounds_old(text, 0, Some(width(20)), width(80));
                }
            });
        }
    }
}
