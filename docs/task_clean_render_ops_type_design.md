# Task: Clean RenderOps Type Design

## Overview

Refactor the single `RenderOp` enum into a nested enum architecture that provides type safety by distinguishing between:
- **RenderOpCommon**: 27 operations used in both contexts (shared)
- **RenderOpIR**: App/Component-level operations (Intermediate Representation)
- **RenderOpOutput**: Terminal/Backend-level operations (Output to terminal)

This prevents accidentally using compositor-only operations in app code and vice versa.

## Problem Statement

Currently, a single `RenderOp` enum is used in two different contexts:

1. **App/Component context**: High-level operations from components
   - Uses: `PaintTextWithAttributes` (handles clipping, Unicode, emoji)

2. **Terminal/Backend context**: Low-level optimized operations for terminal output
   - Uses: `CompositorNoClipTruncPaintTextWithAttributes` (skips clipping, already done)

This creates confusion and allows wrong-context operations to be used accidentally.

### Current Flow

```
App → Component → RenderOp (#1) → OffscreenBuffer → RenderOp (#2) → Terminal
                   ↑                                    ↑
            High-level semantic                 Low-level optimized
            operations from app                 operations for backend
```

## Solution: Nested Enum Pattern

### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    NEW RENDER PIPELINE                          │
├─────────────────────────────────────────────────────────────────┤
│ 1. APP & COMPONENTS                                             │
│    Component::render()                                          │
│    └─> Returns: RenderPipeline containing RenderOpsIR          │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ 2. COMPOSITOR (RenderOpsIR → OffscreenBuffer)                  │
│    compose_render_ops_into_ofs_buf()                            │
│    └─> EXECUTES RenderOpIR by WRITING to OffscreenBuffer       │
│    └─> Output: Populated OffscreenBuffer (2D PixelChar grid)   │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ 3. BACKEND CONVERTER (OffscreenBuffer → RenderOpsOutput)       │
│    OffscreenBufferPaint::render() or render_diff()              │
│    └─> SCANS OffscreenBuffer pixel grid                        │
│    └─> Returns: RenderOpsOutput - optimized for terminal       │
│                                                                 │
├─────────────────────────────────────────────────────────────────┤
│ 4. TERMINAL EXECUTOR (RenderOpsOutput → Terminal)              │
│    PaintRenderOp::paint()                                       │
│    └─> EXECUTES each RenderOpOutput against terminal           │
└─────────────────────────────────────────────────────────────────┘
```

## Design Decisions

### ✅ Confirmed Approach

1. **Three enums**: `RenderOpCommon`, `RenderOpIR`, `RenderOpOutput`
2. **Trait for helpers**: `RenderOpCommonExt` with 27 helper methods (no duplication)
3. **No macros**: Drop `render_ops!` macro, use `vec![]` + constructors
4. **Big bang refactor**: No backward compatibility
5. **Separate collection types**: `RenderOpsIR` and `RenderOpsOutput` (no generics)

### Naming Rationale

- **RenderOpCommon**: Shared operations that work identically in both contexts
- **RenderOpIR**: "Intermediate Representation" - high-level operations from app/component layer
- **RenderOpOutput**: Backend output operations - low-level optimized operations for terminal

## Implementation Plan

### Phase 1: Define New Enum Types

**File**: `tui/src/tui/terminal_lib_backends/render_op.rs`

#### 1.1 Extract Common Operations into RenderOpCommon

Create `RenderOpCommon` enum with 27 variants:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RenderOpCommon {
    // Terminal mode operations
    EnterRawMode,
    ExitRawMode,

    // Cursor movement
    MoveCursorPositionAbs(Pos),
    MoveCursorPositionRelTo(Pos, Pos),
    MoveCursorToColumn(ColIndex),
    MoveCursorToNextLine(RowHeight),
    MoveCursorToPreviousLine(RowHeight),

    // Screen clearing
    ClearScreen,
    ClearCurrentLine,
    ClearToEndOfLine,
    ClearToStartOfLine,

    // Color operations
    SetFgColor(TuiColor),
    SetBgColor(TuiColor),
    ResetColor,
    ApplyColors(Option<TuiStyle>),

    // Text output
    PrintStyledText(InlineString),

    // Cursor visibility
    ShowCursor,
    HideCursor,

    // Cursor position save/restore
    SaveCursorPosition,
    RestoreCursorPosition,

    // Alternate screen
    EnterAlternateScreen,
    ExitAlternateScreen,

    // Mouse and paste modes
    EnableMouseTracking,
    DisableMouseTracking,
    EnableBracketedPaste,
    DisableBracketedPaste,

    // No-op
    Noop,
}
```

#### 1.2 Create RenderOpIR for App/Component Context

```rust
/// Intermediate Representation operations from app/component layer.
/// These are high-level operations that will be processed by the compositor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpIR {
    /// Shared operations that work in both contexts
    Common(RenderOpCommon),

    /// Paint text with attributes (handles clipping, Unicode, emoji).
    /// Used by app components.
    PaintTextWithAttributes(InlineString, Option<TuiStyle>),
}
```

#### 1.3 Create RenderOpOutput for Terminal/Backend Context

```rust
/// Terminal output operations for backend rendering.
/// These are low-level optimized operations for terminal execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderOpOutput {
    /// Shared operations that work in both contexts
    Common(RenderOpCommon),

    /// Paint text without clipping/truncation (already handled by compositor).
    /// **Internal use only** - used by backend converters after OffscreenBuffer processing.
    CompositorNoClipTruncPaintTextWithAttributes(InlineString, Option<TuiStyle>),
}
```

#### 1.4 Add Trait for Helper Methods (No Duplication)

```rust
/// Trait providing ergonomic helper methods for common operations.
/// Implemented by both RenderOpIR and RenderOpOutput to avoid code duplication.
trait RenderOpCommonExt: Sized {
    /// Convert a common operation into the specific type
    fn from_common(common: RenderOpCommon) -> Self;

    // === Terminal Mode Operations ===

    fn enter_raw_mode() -> Self {
        Self::from_common(RenderOpCommon::EnterRawMode)
    }

    fn exit_raw_mode() -> Self {
        Self::from_common(RenderOpCommon::ExitRawMode)
    }

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

    fn clear_screen() -> Self {
        Self::from_common(RenderOpCommon::ClearScreen)
    }

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

    fn reset_color() -> Self {
        Self::from_common(RenderOpCommon::ResetColor)
    }

    fn apply_colors(style: Option<TuiStyle>) -> Self {
        Self::from_common(RenderOpCommon::ApplyColors(style))
    }

    // === Text Output Operations ===

    fn print_styled_text(text: InlineString) -> Self {
        Self::from_common(RenderOpCommon::PrintStyledText(text))
    }

    // === Cursor Visibility Operations ===

    fn show_cursor() -> Self {
        Self::from_common(RenderOpCommon::ShowCursor)
    }

    fn hide_cursor() -> Self {
        Self::from_common(RenderOpCommon::HideCursor)
    }

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

    fn noop() -> Self {
        Self::from_common(RenderOpCommon::Noop)
    }
}

// Implement trait for RenderOpIR
impl RenderOpCommonExt for RenderOpIR {
    fn from_common(common: RenderOpCommon) -> Self {
        RenderOpIR::Common(common)
    }
}

// Implement trait for RenderOpOutput
impl RenderOpCommonExt for RenderOpOutput {
    fn from_common(common: RenderOpCommon) -> Self {
        RenderOpOutput::Common(common)
    }
}
```

### Phase 2: Create Collection Types

**File**: `tui/src/tui/terminal_lib_backends/render_op.rs`

#### 2.1 Create RenderOpsIR Collection

```rust
/// Collection of IR-level render operations from app/component layer.
#[derive(Debug, Clone, Default)]
pub struct RenderOpsIR {
    pub list: InlineVec<RenderOpIR>,
}

impl RenderOpsIR {
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    pub fn push(&mut self, op: RenderOpIR) {
        self.list.push(op);
    }

    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpIR>) {
        self.list.extend(ops);
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderOpIR> {
        self.list.iter()
    }
}

impl From<Vec<RenderOpIR>> for RenderOpsIR {
    fn from(ops: Vec<RenderOpIR>) -> Self {
        Self { list: ops.into() }
    }
}

impl FromIterator<RenderOpIR> for RenderOpsIR {
    fn from_iter<I: IntoIterator<Item = RenderOpIR>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}
```

#### 2.2 Create RenderOpsOutput Collection

```rust
/// Collection of terminal output operations for backend rendering.
#[derive(Debug, Clone, Default)]
pub struct RenderOpsOutput {
    pub list: InlineVec<RenderOpOutput>,
}

impl RenderOpsOutput {
    pub fn new() -> Self {
        Self {
            list: InlineVec::new(),
        }
    }

    pub fn push(&mut self, op: RenderOpOutput) {
        self.list.push(op);
    }

    pub fn extend(&mut self, ops: impl IntoIterator<Item = RenderOpOutput>) {
        self.list.extend(ops);
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &RenderOpOutput> {
        self.list.iter()
    }
}

impl From<Vec<RenderOpOutput>> for RenderOpsOutput {
    fn from(ops: Vec<RenderOpOutput>) -> Self {
        Self { list: ops.into() }
    }
}

impl FromIterator<RenderOpOutput> for RenderOpsOutput {
    fn from_iter<I: IntoIterator<Item = RenderOpOutput>>(iter: I) -> Self {
        Self {
            list: iter.into_iter().collect(),
        }
    }
}
```

### Phase 3: Update RenderPipeline

**File**: `tui/src/tui/terminal_lib_backends/render_pipeline.rs`

```rust
/// Update RenderPipeline to use RenderOpsIR
pub struct RenderPipeline {
    pub map: HashMap<ZOrder, Vec<RenderOpIR>>,
}

// Update all methods to work with RenderOpIR
impl RenderPipeline {
    pub fn push(&mut self, z_order: ZOrder, ops: RenderOpsIR) {
        self.map.entry(z_order).or_default().extend(ops.list);
    }

    // ... update other methods
}
```

### Phase 4: Update Compositor

**File**: `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`

#### 4.1 Update compose_render_ops_into_ofs_buf Signature

```rust
impl RenderPipeline {
    pub fn compose_render_ops_into_ofs_buf(
        &self,
        window_size: Size,
        ofs_buf: &mut OffscreenBuffer,
        memoized_text_widths: &mut MemoizedTextWidths,
    ) {
        let mut render_local_data = RenderOpsLocalData::default();

        for z_order in [ZOrder::Background, ZOrder::Normal, ZOrder::Glass] {
            if let Some(vec_render_op) = self.map.get(&z_order) {
                for render_op_ir in vec_render_op {
                    process_render_op_ir(
                        render_op_ir,
                        ofs_buf,
                        &mut render_local_data,
                        window_size,
                        memoized_text_widths,
                    ).ok();
                }
            }
        }
    }
}
```

#### 4.2 Update process_render_op to Accept RenderOpIR

```rust
fn process_render_op_ir(
    render_op: &RenderOpIR,
    ofs_buf: &mut OffscreenBuffer,
    render_local_data: &mut RenderOpsLocalData,
    window_size: Size,
    memoized_text_widths: &mut MemoizedTextWidths,
) -> CommonResult<()> {
    match render_op {
        RenderOpIR::Common(common) => {
            process_common_render_op(common, ofs_buf, render_local_data)?;
        }
        RenderOpIR::PaintTextWithAttributes(text, maybe_style) => {
            print_text_with_attributes(
                text,
                maybe_style.as_ref(),
                ofs_buf,
                None, // maybe_max_display_col_count
                render_local_data,
            )?;
        }
    }
    Ok(())
}
```

#### 4.3 Create Common Operation Handler

```rust
fn process_common_render_op(
    common_op: &RenderOpCommon,
    ofs_buf: &mut OffscreenBuffer,
    render_local_data: &mut RenderOpsLocalData,
) -> CommonResult<()> {
    match common_op {
        RenderOpCommon::EnterRawMode => {
            ofs_buf.terminal_mode.raw_mode = true;
        }
        RenderOpCommon::ExitRawMode => {
            ofs_buf.terminal_mode.raw_mode = false;
        }
        RenderOpCommon::MoveCursorPositionAbs(abs_pos) => {
            ofs_buf.cursor_pos = sanitize_and_save_abs_pos(
                *abs_pos,
                ofs_buf.window_size,
                render_local_data,
            );
        }
        RenderOpCommon::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
            let new_abs_pos = *box_origin_pos + *content_rel_pos;
            ofs_buf.cursor_pos = sanitize_and_save_abs_pos(
                new_abs_pos,
                ofs_buf.window_size,
                render_local_data,
            );
        }
        RenderOpCommon::SetFgColor(color) => {
            render_local_data.fg_color = Some(*color);
        }
        RenderOpCommon::SetBgColor(color) => {
            render_local_data.bg_color = Some(*color);
        }
        RenderOpCommon::ResetColor => {
            render_local_data.fg_color = None;
            render_local_data.bg_color = None;
        }
        RenderOpCommon::ApplyColors(maybe_style) => {
            if let Some(style) = maybe_style {
                render_local_data.fg_color = style.color_fg;
                render_local_data.bg_color = style.color_bg;
            }
        }
        // ... handle all other common operations
        _ => {
            // Handle remaining operations or no-op
        }
    }
    Ok(())
}
```

### Phase 5: Update Backend Converters

**File**: `tui/src/tui/terminal_lib_backends/crossterm_backend/offscreen_buffer_paint_impl.rs`

#### 5.1 Update OffscreenBufferPaint Trait

```rust
pub trait OffscreenBufferPaint {
    fn render(&mut self, offscreen_buffer: &OffscreenBuffer) -> RenderOpsOutput;

    fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOpsOutput;

    fn paint(
        &mut self,
        render_ops: RenderOpsOutput,
        flush_kind: FlushKind,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );

    fn paint_diff(
        &mut self,
        render_ops: RenderOpsOutput,
        window_size: Size,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
```

#### 5.2 Update render() Implementation

```rust
impl OffscreenBufferPaint for OffscreenBufferPaintImplCrossterm {
    fn render(&mut self, ofs_buf: &OffscreenBuffer) -> RenderOpsOutput {
        use render_helper::Context;

        let mut context = Context::new();

        // For each line in the offscreen buffer
        for (row_index, line) in ofs_buf.buffer.iter().enumerate() {
            context.clear_for_new_line(row(row_index));

            // For each pixel char in the line
            for (pixel_char_index, pixel_char) in line.iter().enumerate() {
                // ... existing logic ...

                // Generate output operations
                if !is_style_same_as_prev {
                    render_helper::flush_all_buffers(&mut context);
                }

                context.buffer_plain_text.push_str(&pixel_char_content);

                if is_at_end_of_line {
                    render_helper::flush_all_buffers(&mut context);
                }
            }
        }

        if !context.buffer_plain_text.is_empty() {
            render_helper::flush_all_buffers(&mut context);
        }

        context.render_ops
    }

    fn render_diff(&mut self, diff_chunks: &PixelCharDiffChunks) -> RenderOpsOutput {
        let mut ops = RenderOpsOutput::new();

        for (position, pixel_char) in diff_chunks.iter() {
            ops.push(RenderOpOutput::move_cursor(*position));
            ops.push(RenderOpOutput::reset_color());

            match pixel_char {
                PixelChar::Void => { /* continue */ }
                PixelChar::Spacer => {
                    ops.push(RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                        SPACER_GLYPH.into(),
                        None,
                    ));
                }
                PixelChar::PlainText { display_char, style, .. } => {
                    ops.push(RenderOpOutput::apply_colors(Some(*style)));
                    ops.push(RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
                        InlineString::from_str(&display_char.to_string()),
                        Some(*style),
                    ));
                }
            }
        }

        ops
    }
}
```

#### 5.3 Update render_helper Module

```rust
mod render_helper {
    pub struct Context {
        pub display_col_index_for_line: ColIndex,
        pub display_row_index: RowIndex,
        pub buffer_plain_text: InlineString,
        pub prev_style: Option<TuiStyle>,
        pub render_ops: RenderOpsOutput,  // ← Changed from RenderOps
    }

    impl Context {
        pub fn new() -> Self {
            Context {
                display_col_index_for_line: col(0),
                buffer_plain_text: InlineString::new(),
                render_ops: RenderOpsOutput::new(),  // ← Changed
                display_row_index: row(0),
                prev_style: None,
            }
        }
    }

    pub fn flush_plain_text_line_buffer(context: &mut Context) {
        let pos = context.display_col_index_for_line + context.display_row_index;

        context.render_ops.push(RenderOpOutput::move_cursor(pos));
        context.render_ops.push(RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(
            context.buffer_plain_text.clone(),
            context.prev_style,
        ));

        let display_width = GCStringOwned::from(&context.buffer_plain_text).width();
        *context.display_col_index_for_line += *display_width;

        context.buffer_plain_text.clear();
    }
}
```

### Phase 6: Update Backend Executors

**File**: `tui/src/tui/terminal_lib_backends/crossterm_backend/paint_render_op_impl.rs`

#### 6.1 Update PaintRenderOp Trait

```rust
pub trait PaintRenderOp {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        render_op: &RenderOpOutput,  // ← Changed
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    );
}
```

#### 6.2 Update paint() Implementation

```rust
impl PaintRenderOp for PaintRenderOpImplCrossterm {
    fn paint(
        &mut self,
        skip_flush: &mut bool,
        command_ref: &RenderOpOutput,  // ← Changed
        window_size: Size,
        render_local_data: &mut RenderOpsLocalData,
        locked_output_device: LockedOutputDevice<'_>,
        is_mock: bool,
    ) {
        match command_ref {
            RenderOpOutput::Common(common) => {
                process_common_output_op(
                    common,
                    skip_flush,
                    window_size,
                    render_local_data,
                    locked_output_device,
                    is_mock,
                );
            }
            RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(text, maybe_style) => {
                PaintRenderOpImplCrossterm::paint_text_with_attributes(
                    text,
                    *maybe_style,
                    window_size,
                    render_local_data,
                    locked_output_device,
                );
            }
        }
    }
}
```

#### 6.3 Create Common Output Handler

```rust
fn process_common_output_op(
    common_op: &RenderOpCommon,
    skip_flush: &mut bool,
    window_size: Size,
    render_local_data: &mut RenderOpsLocalData,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    match common_op {
        RenderOpCommon::EnterRawMode => {
            PaintRenderOpImplCrossterm::raw_mode_enter(
                skip_flush,
                locked_output_device,
                is_mock,
            );
        }
        RenderOpCommon::ExitRawMode => {
            PaintRenderOpImplCrossterm::raw_mode_exit(
                skip_flush,
                locked_output_device,
                is_mock,
            );
        }
        RenderOpCommon::MoveCursorPositionAbs(abs_pos) => {
            PaintRenderOpImplCrossterm::move_cursor_position_abs(
                *abs_pos,
                window_size,
                render_local_data,
                locked_output_device,
            );
        }
        RenderOpCommon::MoveCursorPositionRelTo(box_origin_pos, content_rel_pos) => {
            PaintRenderOpImplCrossterm::move_cursor_position_rel_to(
                *box_origin_pos,
                *content_rel_pos,
                window_size,
                render_local_data,
                locked_output_device,
            );
        }
        RenderOpCommon::ClearScreen => {
            queue_terminal_command!(
                locked_output_device,
                "ClearScreen",
                Clear(ClearType::All),
            );
        }
        RenderOpCommon::SetFgColor(color) => {
            PaintRenderOpImplCrossterm::set_fg_color(*color, locked_output_device);
        }
        RenderOpCommon::SetBgColor(color) => {
            PaintRenderOpImplCrossterm::set_bg_color(*color, locked_output_device);
        }
        RenderOpCommon::ResetColor => {
            queue_terminal_command!(locked_output_device, "ResetColor", ResetColor);
        }
        // ... handle all other common operations
        _ => { /* Handle remaining operations */ }
    }
}
```

### Phase 7: Update paint.rs Orchestration

**File**: `tui/src/tui/terminal_lib_backends/paint.rs`

```rust
fn perform_full_paint(
    ofs_buf: &OffscreenBuffer,
    flush_kind: FlushKind,
    window_size: Size,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
            let render_ops: RenderOpsOutput = crossterm_impl.render(ofs_buf);  // ← Explicit type
            crossterm_impl.paint(
                render_ops,
                flush_kind,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
        TerminalLibBackend::Termion => unimplemented!(),
    }
}

fn perform_diff_paint(
    diff_chunks: &PixelCharDiffChunks,
    window_size: Size,
    locked_output_device: LockedOutputDevice<'_>,
    is_mock: bool,
) {
    match TERMINAL_LIB_BACKEND {
        TerminalLibBackend::Crossterm => {
            let mut crossterm_impl = OffscreenBufferPaintImplCrossterm {};
            let render_ops: RenderOpsOutput = crossterm_impl.render_diff(diff_chunks);  // ← Explicit type
            crossterm_impl.paint_diff(
                render_ops,
                window_size,
                locked_output_device,
                is_mock,
            );
        }
        TerminalLibBackend::Termion => unimplemented!(),
    }
}
```

### Phase 8: Update Component API & App Code

**Files**: All component implementations (~9 files)

#### 8.1 Component Trait (No Changes Needed)

```rust
pub trait Component {
    fn render(
        &mut self,
        global_data: &mut GlobalData,
        current_box: FlexBox,
        surface_bounds: SurfaceBounds,
        has_focus: &mut HasFocus,
    ) -> CommonResult<RenderPipeline>;  // Already uses RenderPipeline with RenderOpsIR
}
```

#### 8.2 Update Component Implementations

**Before:**
```rust
let ops = render_ops!(@new
    RenderOp::MoveCursorPositionAbs(pos),
    RenderOp::SetFgColor(color),
    RenderOp::PaintTextWithAttributes("Hello", Some(style)),
);
```

**After:**
```rust
let ops = RenderOpsIR::from(vec![
    RenderOpIR::move_cursor(pos),
    RenderOpIR::set_fg_color(color),
    RenderOpIR::PaintTextWithAttributes("Hello".into(), Some(style)),
]);
```

Or incrementally:
```rust
let mut ops = RenderOpsIR::new();
ops.push(RenderOpIR::move_cursor(pos));
ops.push(RenderOpIR::set_fg_color(color));
ops.push(RenderOpIR::PaintTextWithAttributes("Hello".into(), Some(style)));
```

#### 8.3 Update render_tui_styled_texts Function

**File**: `tui/src/tui/terminal_lib_backends/render_tui_styled_texts.rs`

```rust
pub fn render_tui_styled_texts_into(
    styled_texts: &TuiStyledTexts,
    render_ops: &mut RenderOpsIR,  // ← Changed type
) {
    for styled_text in &styled_texts.items {
        render_ops.push(RenderOpIR::PaintTextWithAttributes(
            styled_text.text.clone(),
            Some(styled_text.style),
        ));
    }
}
```

### Phase 9: Remove Old Code

#### 9.1 Remove render_ops! Macro

**File**: `tui/src/tui/terminal_lib_backends/render_op.rs`

Delete the entire `render_ops!` macro definition (lines 60-133).

#### 9.2 Remove Old RenderOp Enum

Delete the old `RenderOp` enum with 29 variants.

#### 9.3 Remove Old RenderOps Collection

Delete the old `RenderOps` struct that wrapped `InlineVec<RenderOp>`.

### Phase 10: Update Tests

**Files**:
- `tui/src/tui/terminal_lib_backends/test_render_pipeline.rs`
- `tui/src/tui/terminal_lib_backends/render_op_bench.rs`
- `tui/src/tui/terminal_lib_backends/crossterm_backend/offscreen_buffer_paint_impl.rs` (test module)

Update all test code to use new types:

```rust
// Before:
let ops = render_ops!(@new RenderOp::ClearScreen);

// After:
let ops = RenderOpsIR::from(vec![RenderOpIR::clear_screen()]);
```

Add type safety tests:

```rust
#[test]
fn test_type_safety_ir_vs_output() {
    // IR operations
    let ir_ops = RenderOpsIR::from(vec![
        RenderOpIR::move_cursor(Pos::default()),
        RenderOpIR::PaintTextWithAttributes("test".into(), None),
    ]);

    // Output operations
    let output_ops = RenderOpsOutput::from(vec![
        RenderOpOutput::move_cursor(Pos::default()),
        RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes("test".into(), None),
    ]);

    // These should be different types and not interchangeable
    // (This test just documents the type safety, actual mixing won't compile)
}
```

## Migration Guide

### For Component Authors

**Old code:**
```rust
render_ops!(@new
    RenderOp::MoveCursorPositionAbs(pos),
    RenderOp::SetFgColor(TuiColor::Green),
    RenderOp::PaintTextWithAttributes("Hello", Some(style)),
)
```

**New code:**
```rust
RenderOpsIR::from(vec![
    RenderOpIR::move_cursor(pos),
    RenderOpIR::set_fg_color(TuiColor::Green),
    RenderOpIR::PaintTextWithAttributes("Hello".into(), Some(style)),
])
```

### For Backend Implementers

**Old code:**
```rust
let mut ops = render_ops!();
ops.push(RenderOp::MoveCursor(pos));
ops.push(RenderOp::CompositorNoClipTruncPaintTextWithAttributes(text, style));
```

**New code:**
```rust
let mut ops = RenderOpsOutput::new();
ops.push(RenderOpOutput::move_cursor(pos));
ops.push(RenderOpOutput::CompositorNoClipTruncPaintTextWithAttributes(text, style));
```

## Benefits

### Type Safety
- ✅ Compiler prevents using compositor operations in app code
- ✅ Compiler prevents using app operations in backend code
- ✅ Clear separation of concerns enforced at compile time

### Maintainability
- ✅ Self-documenting: Function signatures show operation context
- ✅ No code duplication: Common operations defined once
- ✅ Single source of truth for each operation type

### Ergonomics
- ✅ Clean call sites with helper methods
- ✅ Standard Rust patterns (`vec![]`, `new()`, `push()`)
- ✅ Better IDE support (no macro expansion, better autocomplete)

### Architecture
- ✅ Clear pipeline stages with distinct types
- ✅ Explicit conversions between IR and Output
- ✅ Future-proof for additional operation types

## Expected Impact

### Files to Modify (~17 files)

**Core infrastructure:**
- `render_op.rs` - Define new enums + trait (~700 lines)
- `render_pipeline.rs` - Update pipeline types (~20 lines)
- `compositor_render_ops_to_ofs_buf.rs` - Accept RenderOpIR (~150 lines)
- `crossterm_backend/offscreen_buffer_paint_impl.rs` - Return RenderOpsOutput (~80 lines)
- `crossterm_backend/paint_render_op_impl.rs` - Accept RenderOpOutput (~200 lines)
- `paint.rs` - Update orchestration (~40 lines)

**Components:**
- `dialog_engine_api.rs` - Update dialog rendering (~40 lines)
- `editor_engine/engine_public_api.rs` - Update editor rendering (~30 lines)
- `main_event_loop.rs` - Update event loop rendering (~20 lines)
- `render_tui_styled_texts.rs` - Update styled text helper (~10 lines)
- Example components (~30 lines each)

**Tests & supporting code:**
- `test_render_pipeline.rs` - Update tests (~50 lines)
- `render_op_bench.rs` - Update benchmarks (~30 lines)
- `crossterm_backend/debug.rs` - Update debug formatting (~20 lines)

### Estimated Effort
- **Phase 1-2** (New types + collections): 3-4 hours
- **Phase 3-4** (Pipeline + compositor): 3-4 hours
- **Phase 5-7** (Backend converter + executor + orchestration): 4-5 hours
- **Phase 8** (Components): 3-4 hours
- **Phase 9-10** (Cleanup + tests): 2-3 hours
- **Total**: ~15-20 hours of focused work

## Testing Checklist

- [ ] All existing tests pass with new types
- [ ] Type safety: wrong-context operations fail to compile
- [ ] Pattern matching coverage for all 29 variants (27 common + 2 specific)
- [ ] Helper methods generate correct nested enums
- [ ] Collection methods (`new()`, `push()`, `extend()`, `from()`) work correctly
- [ ] Performance benchmarks unchanged (verify no regression)
- [ ] All examples compile and run correctly
- [ ] Edge cases: empty operations, Noop operations, etc.

## Documentation Updates

- [ ] **render_op.rs**: Module-level docs explaining three-enum architecture
- [ ] **render_op.rs**: Doc comments on when to use IR vs Output
- [ ] **CLAUDE.md**: Update rendering pipeline section with new flow
- [ ] **Architecture diagram**: Visual showing IR → OffscreenBuffer → Output
- [ ] **Migration guide**: For users updating their components
- [ ] **Helper method docs**: Examples showing ergonomic usage

## Success Criteria

1. ✅ All tests pass
2. ✅ No compiler errors or warnings
3. ✅ All examples run correctly
4. ✅ Type safety enforced (wrong-context ops don't compile)
5. ✅ Performance unchanged (benchmark comparison)
6. ✅ Documentation updated
7. ✅ Code follows project style guidelines

## Rollback Plan

If issues arise during implementation:

1. **Keep old code**: Keep old `RenderOp` alongside new types temporarily
2. **Gradual migration**: Migrate one module at a time
3. **Feature flag**: Use conditional compilation if needed
4. **Full revert**: Git history allows complete rollback if necessary

## Future Enhancements

After this refactor, future improvements become easier:

1. **Additional operation types**: Can add IR-only or Output-only operations
2. **Backend-specific operations**: Output enum can have backend variants
3. **Validation**: Add compile-time checks for operation sequences
4. **Optimization**: Different optimization strategies per context
5. **Serialization**: Separate serialization for IR vs Output if needed
