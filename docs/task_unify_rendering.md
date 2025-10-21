<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Task: Unify ASText and TuiStyledText Rendering Paths](#task-unify-astext-and-tuistyledtext-rendering-paths)
  - [Overview](#overview)
  - [Current State Analysis](#current-state-analysis)
    - [Path 1: Full TUI Rendering](#path-1-full-tui-rendering)
    - [Path 2: Direct ASText Rendering](#path-2-direct-astext-rendering)
    - [Why the Fork Exists](#why-the-fork-exists)
  - [Unified Architecture: PixelChar-based Rendering](#unified-architecture-pixelchar-based-rendering)
    - [Core Design Principles](#core-design-principles)
    - [Architecture Overview](#architecture-overview)
  - [Current Progress (Commits: e0d25552, 4b5c8de1)](#current-progress-commits-e0d25552-4b5c8de1)
    - [✅ Foundational Consolidation Complete](#-foundational-consolidation-complete)
    - [Benefits Already Realized](#benefits-already-realized)
  - [Implementation Plan](#implementation-plan)
    - [Phase 0: ✅ COMPLETE - Consolidate Style Attributes (Committed)](#phase-0--complete---consolidate-style-attributes-committed)
    - [Phase 1: Extend ASText PixelChar Support (NEXT)](#phase-1-extend-astext-pixelchar-support-next)
    - [Phase 2: Create Unified ANSI Generator (NEXT after Phase 1)](#phase-2-create-unified-ansi-generator-next-after-phase-1)
    - [Phase 3: Create Flexible Buffer Types (NEXT after Phase 2)](#phase-3-create-flexible-buffer-types-next-after-phase-2)
    - [Phase 4: Update ASText Rendering (NEXT after Phase 3)](#phase-4-update-astext-rendering-next-after-phase-3)
    - [Phase 5: Update choose() Implementation (NEXT after Phase 4)](#phase-5-update-choose-implementation-next-after-phase-4)
    - [Phase 6: Update RenderOp Implementation (FINAL - after Phase 5)](#phase-6-update-renderop-implementation-final---after-phase-5)
  - [Integration with Direct ANSI Plan](#integration-with-direct-ansi-plan)
    - [Shared Components](#shared-components)
    - [Migration Path](#migration-path)
  - [Benefits](#benefits)
    - [Performance](#performance)
    - [Architecture](#architecture)
    - [Developer Experience](#developer-experience)
  - [Testing Strategy](#testing-strategy)
    - [Unit Tests](#unit-tests)
    - [Integration Tests](#integration-tests)
    - [Visual Tests](#visual-tests)
  - [Migration Strategy](#migration-strategy)
    - [Phase 1: Parallel Implementation](#phase-1-parallel-implementation)
    - [Phase 2: Gradual Migration](#phase-2-gradual-migration)
    - [Phase 3: Cleanup](#phase-3-cleanup)
  - [Success Metrics](#success-metrics)
  - [Risks and Mitigation](#risks-and-mitigation)
  - [Conclusion](#conclusion)
  - [Status Update (October 21, 2025)](#status-update-october-21-2025)
    - [✅ Phase 0 Complete - Foundation Laid](#-phase-0-complete---foundation-laid)
    - [🎯 Next Steps](#-next-steps)
    - [Key Insight](#key-insight)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Task: Unify ASText and TuiStyledText Rendering Paths

## Overview

This document outlines the plan to unify the two text rendering paths in the TUI crate:

1. **Path 1**: App → RenderOps → OffscreenBuffer → crossterm → stdout (used by full TUI)
2. **Path 2**: ASText → crossterm → stdout (used by choose() API)

The goal is to create a single, optimized rendering pipeline that works for both use cases while
preparing for the future removal of crossterm dependency. The overhead of maintaining two separate
rendering paths is significant, and unifying them will simplify the codebase and make it easier to
maintain. Also we are planning to [remove crossterm in the future](task_remove_crossterm.md), so
this unification will also prepare for that.

## Current State Analysis

### Path 1: Full TUI Rendering

- **TuiStyledText**: Primary text type for TUI framework
- **RenderOps**: Command pattern for rendering operations
- **OffscreenBuffer**: Grid of `PixelChar` structs containing styled characters
- **Features**: Compositor, z-ordering, diffing, caching, clipping

### Path 2: Direct ASText Rendering

- **AnsiStyledText (ASText)**: Lightweight styled text type
- **Direct rendering**: Bypasses full TUI pipeline for performance
- **Used by**: choose() API and other simple text output needs
- **Implementation**: Display trait that generates ANSI escape sequences

### Why the Fork Exists

1. **Historical evolution**: The full TUI framework predates ASText
   - ASText was created later just for choose() with a requirement not to depend on r3bl_tui crate,
     in the r3bl_tuify crate
   - In late 2024 / early 2025 r3bl_tuify was removed and the code for choose() moved into the
     r3bl_tui crate, along with many other crates which were removed after their functionality was
     integrated into r3bl_tui. The deprecated creates are archived in the `r3bl-open-core-archive`
     repo.
2. **Performance requirements**: choose() needs minimal overhead, and it does not have the same
   performance requirements as the full TUI framework.
3. **Different use cases**: Full TUI needs compositing; choose() doesn't

## Unified Architecture: PixelChar-based Rendering

### Core Design Principles

1. **PixelChar as universal IR**: Both text types convert to PixelChar arrays
2. **Single ANSI generator**: One module responsible for PixelChar → ANSI conversion
3. **Flexible buffer types**: Lightweight for choose(), full-featured for TUI
4. **Direct ANSI ready**: Designed for future crossterm removal

### Architecture Overview

```
ASText        ─┐
               ├─→ PixelChar[] ─→ PixelCharRenderer ─→ ANSI sequences ─→ stdout
TuiStyledText ─┘
```

## Current Progress (Commits: e0d25552, 4b5c8de1)

### ✅ Foundational Consolidation Complete

The following work has been completed to lay the groundwork for unified rendering:

#### **Style Attributes Consolidation**

- **Unified style representation**: Replaced redundant `StyleAttribute` enum with unified `TuiStyleAttribs`
  - All ANSI styling attributes now consolidated into a single type
  - Both ASText and TuiStyledText now use the same internal style system
  - Removed intermediate type conversions in the parser layer

- **Enhanced TuiStyleAttribs**:
  - Added `BlinkMode` enum (`Slow` | `Rapid`) for proper ANSI blink distinction
  - Added `Overline` field for complete attribute support
  - Unified trait operations (`Add`, `AddAssign`, `From`) across all attributes

#### **Parser Layer Refactoring**

- **Three-layer architecture cleanup**:
  - Shim layer: Parameter translation (vt_100_shim_sgr_ops)
  - Implementation layer: Business logic (vt_100_impl_sgr_ops)
  - Test layer: Full pipeline validation (vt_100_test_sgr_ops)

- **Direct TuiStyleAttribs usage**: Parser now directly uses `TuiStyleAttribs` without intermediates
  - Proper SGR code routing (e.g., SGR 5 → Slow, SGR 6 → Rapid)
  - Simplified implementation without intermediate enum conversions

#### **Test Coverage**
- All 2,058 tests passing with new consolidated style system
- ANSI compliance verified for all blink modes and attributes
- Parser integration tests validate full pipeline

### Benefits Already Realized

1. **Reduced complexity**: Single unified style type eliminates redundancy
2. **Better maintainability**: Changes to styling only need to happen in one place
3. **Foundation laid**: Ready for Phase 1-2 of rendering unification

## Implementation Plan

### Phase 0: ✅ COMPLETE - Consolidate Style Attributes (Committed)

Foundation work completed:
- Unified `TuiStyleAttribs` as the canonical style representation
- Removed `StyleAttribute` intermediate enum
- Enhanced with `BlinkMode` and `Overline`
- Parser layer refactored to use unified types
- All tests passing (2,058/2,058)

**Ready for**: Phase 1 - Unified rendering implementation

### Phase 1: Extend ASText PixelChar Support (NEXT)

ASText already has a `convert()` method that generates PixelChar arrays. We'll make this the primary
rendering path and ensure it uses the unified `TuiStyleAttribs`.

```rust
// Existing method we'll build upon
impl AnsiStyledText {
    pub fn convert(&self, options: impl Into<ASTextConvertOptions>) -> InlineVec<PixelChar> {
        // Already converts text + styles to PixelChar array
        // Now guaranteed to use unified TuiStyleAttribs internally
    }
}
```

**Current state**: ASText uses internal `ASTStyle` vector - should standardize on `TuiStyleAttribs` for consistency

### Phase 2: Create Unified ANSI Generator (NEXT after Phase 1)

Create a new module responsible for converting PixelChar arrays to ANSI sequences.
Will leverage the unified `TuiStyleAttribs` to generate consistent ANSI codes:

```rust
// New module: tui/terminal_lib_backends/direct_ansi/pixel_char_renderer.rs
pub struct PixelCharRenderer {
    buffer: Vec<u8>,           // Pre-allocated ANSI sequence buffer
    current_style: Option<TuiStyle>, // Track style to minimize escape sequences
}

impl PixelCharRenderer {
    pub fn new() -> Self {
        Self {
            buffer: Vec::with_capacity(4096), // Reasonable default
            current_style: None,
        }
    }

    /// Render a line of PixelChars to ANSI escape sequences
    pub fn render_line(&mut self, pixels: &[PixelChar]) -> &[u8] {
        self.buffer.clear();

        for pixel in pixels {
            match pixel {
                PixelChar::PlainText { display_char, maybe_style } => {
                    // Only emit style changes when necessary
                    if maybe_style != &self.current_style {
                        self.apply_style_change(&self.current_style, maybe_style);
                        self.current_style = *maybe_style;
                    }

                    // Write the character
                    let mut char_buf = [0u8; 4];
                    let char_str = display_char.encode_utf8(&mut char_buf);
                    self.buffer.extend_from_slice(char_str.as_bytes());
                }
                PixelChar::Spacer => {
                    self.buffer.push(b' ');
                }
                PixelChar::Void => {
                    // Skip - already accounted for in positioning
                }
            }
        }

        &self.buffer
    }

    /// Smart style diffing - only emit necessary ANSI codes
    fn apply_style_change(&mut self, from: &Option<TuiStyle>, to: &Option<TuiStyle>) {
        match (from, to) {
            (None, None) => {} // No change
            (Some(_), None) => {
                // Reset all attributes
                self.buffer.extend_from_slice(b"\x1b[0m");
            }
            (None, Some(new_style)) | (Some(old_style), Some(new_style)) => {
                // Optimize: only reset if necessary
                if from.is_some() && Self::needs_reset(old_style.unwrap(), *new_style) {
                    self.buffer.extend_from_slice(b"\x1b[0m");
                }

                // Apply new style attributes
                self.apply_style(new_style);
            }
        }
    }

    fn apply_style(&mut self, style: &TuiStyle) {
        // Apply colors
        if let Some(fg) = style.color_fg {
            self.apply_fg_color(fg);
        }
        if let Some(bg) = style.color_bg {
            self.apply_bg_color(bg);
        }

        // Apply attributes
        if style.bold.is_some() {
            self.buffer.extend_from_slice(b"\x1b[1m");
        }
        if style.dim.is_some() {
            self.buffer.extend_from_slice(b"\x1b[2m");
        }
        if style.italic.is_some() {
            self.buffer.extend_from_slice(b"\x1b[3m");
        }
        if style.underline.is_some() {
            self.buffer.extend_from_slice(b"\x1b[4m");
        }
        // ... other attributes
    }

    fn apply_fg_color(&mut self, color: TuiColor) {
        // Reuse existing optimized color conversion logic
        let sgr = color_to_sgr_code(color, true);
        sgr.write_to_buf(&mut self.buffer).ok();
    }

    fn apply_bg_color(&mut self, color: TuiColor) {
        let sgr = color_to_sgr_code(color, false);
        sgr.write_to_buf(&mut self.buffer).ok();
    }
}
```

### Phase 3: Create Flexible Buffer Types (NEXT after Phase 2)

Support both lightweight (choose) and full-featured (TUI) use cases.
Both will use the unified `TuiStyleAttribs` for style management:

```rust
pub enum OffscreenBufferMode {
    /// Full-featured buffer with all TUI capabilities
    Full {
        buffer: PixelCharLines,
        window_size: Size,
        my_pos: Pos,
        my_fg_color: Option<TuiColor>,
        my_bg_color: Option<TuiColor>,
        memory_size_calc_cache: MemoizedMemorySize,
    },

    /// Lightweight buffer for simple rendering (choose, etc.)
    Lightweight {
        lines: Vec<Vec<PixelChar>>, // Simple Vec, no smallvec overhead
        width: usize,
    }
}

impl OffscreenBufferMode {
    /// Render buffer contents to ANSI using unified renderer
    pub fn render_to_ansi(&self, renderer: &mut PixelCharRenderer) -> Vec<u8> {
        let mut output = Vec::new();

        match self {
            OffscreenBufferMode::Full { buffer, .. } => {
                for (row_idx, line) in buffer.lines.iter().enumerate() {
                    if row_idx > 0 {
                        output.extend_from_slice(b"\r\n");
                    }
                    let ansi_line = renderer.render_line(&line.pixel_chars);
                    output.extend_from_slice(ansi_line);
                }
            }
            OffscreenBufferMode::Lightweight { lines, .. } => {
                for (row_idx, line) in lines.iter().enumerate() {
                    if row_idx > 0 {
                        output.extend_from_slice(b"\r\n");
                    }
                    let ansi_line = renderer.render_line(line);
                    output.extend_from_slice(ansi_line);
                }
            }
        }

        // Reset style at end if needed
        if renderer.current_style.is_some() {
            output.extend_from_slice(b"\x1b[0m");
            renderer.current_style = None;
        }

        output
    }
}
```

### Phase 4: Update ASText Rendering (NEXT after Phase 3)

Modify ASText to use the new unified renderer and `TuiStyleAttribs`:

```rust
impl Display for ASText {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        // Convert to PixelChar array
        let pixels = self.convert(ASTextConvertOptions::default());

        // Use unified renderer
        let mut renderer = PixelCharRenderer::new();
        let ansi_output = renderer.render_line(&pixels);

        // Write to formatter
        f.write_str(std::str::from_utf8(ansi_output).unwrap())
    }
}
```

### Phase 5: Update choose() Implementation (NEXT after Phase 4)

Migrate SelectComponent to use the unified rendering pipeline with `TuiStyleAttribs`:

```rust
impl FunctionComponent<State> for SelectComponent {
    fn render(&mut self, state: &mut State) -> CommonResult<()> {
        // Create lightweight buffer
        let mut buffer = OffscreenBufferMode::Lightweight {
            lines: Vec::with_capacity(state.items.len() + 1),
            width: viewport_width.as_usize(),
        };

        // Render header using ASText
        match &state.header {
            Header::SingleLine(text) => {
                let header_ast = ast(text, self.style.header_style.into());
                let pixels = header_ast.convert(viewport_width);
                buffer.lines.push(pixels.into_vec());
            }
            Header::MultiLine(lines) => {
                for line in lines {
                    let mut line_pixels = Vec::new();
                    for ast in line {
                        let pixels = ast.convert(ASTextConvertOptions::default());
                        line_pixels.extend(pixels);
                    }
                    buffer.lines.push(line_pixels);
                }
            }
        }

        // Render items
        for (idx, item) in state.visible_items().enumerate() {
            let style = determine_item_style(idx, state, &self.style);
            let prefix = create_item_prefix(idx, state);
            let item_ast = ast(&format!("{}{}", prefix, item), style.into());
            let pixels = item_ast.convert(viewport_width);
            buffer.lines.push(pixels.into_vec());
        }

        // Render to ANSI
        let mut renderer = PixelCharRenderer::new();
        let ansi_output = buffer.render_to_ansi(&mut renderer);

        // Write directly to output device
        self.output_device.write_all(&ansi_output)?;
        self.output_device.flush()?;

        Ok(())
    }
}
```

### Phase 6: Update RenderOp Implementation (FINAL - after Phase 5)

Modify RenderOp::PaintTextWithAttributes to use the unified renderer and `TuiStyleAttribs`:

```rust
impl PaintRenderOp for RenderOpImplCrossterm {
    fn paint(&mut self, /* params */) {
        match render_op {
            RenderOp::PaintTextWithAttributes(text, maybe_style) => {
                // Create ASText from the text and style
                let ast = ASText {
                    text: text.clone(),
                    styles: maybe_style.map(|s| s.into()).unwrap_or_default(),
                };

                // Convert to PixelChar
                let pixels = ast.convert(ASTextConvertOptions::default());

                // Render using unified renderer
                let mut renderer = PixelCharRenderer::new();
                let ansi_output = renderer.render_line(&pixels);

                // Write to output device
                locked_output_device.write_all(ansi_output).ok();
            }
            // ... other ops
        }
    }
}
```

## Integration with Direct ANSI Plan

This unification perfectly aligns with the crossterm removal plan:

### Shared Components

The `PixelCharRenderer` will become the core of the direct ANSI backend:

- No crossterm dependency in the renderer
- Direct ANSI escape sequence generation
- Platform-agnostic text rendering

### Migration Path

1. Implement unified rendering with crossterm still in place
2. Switch `PixelCharRenderer` to direct ANSI when ready
3. All text rendering automatically uses direct ANSI

## Benefits

### Performance

- **Single optimization point**: All ANSI generation in one place
- **Smart style diffing**: Minimize escape sequences
- **Pre-allocated buffers**: Reduce allocations
- **Lightweight path for choose()**: No unnecessary overhead

### Architecture

- **Unified pipeline**: Easier to understand and maintain
- **Clear separation**: Text representation vs. rendering
- **Future-proof**: Ready for direct ANSI migration
- **Testable**: Can test ANSI output directly

### Developer Experience

- **Consistent behavior**: All text renders the same way
- **Single API**: PixelChar as universal representation
- **Easier debugging**: One rendering path to trace

## Testing Strategy

### Unit Tests

1. **ASText rendering**: Compare old vs. new output
2. **Style transitions**: Verify optimal ANSI sequences
3. **PixelChar conversion**: Test all text types
4. **Buffer modes**: Test both lightweight and full modes

### Integration Tests

1. **choose() functionality**: Ensure no visual changes
2. **Full TUI rendering**: Verify no regressions
3. **Performance benchmarks**: Measure improvements
4. **Memory usage**: Verify lightweight mode efficiency

### Visual Tests

1. Side-by-side comparison of old vs. new rendering
2. Test on multiple terminals
3. Verify style attributes work correctly

## Migration Strategy

### Phase 1: Parallel Implementation

- Build new system alongside existing code
- Feature flag: `unified-rendering`
- No breaking changes

### Phase 2: Gradual Migration

- Migrate choose() first (lower risk)
- Then migrate ASText Display impl
- Finally update RenderOp

### Phase 3: Cleanup

- Remove old rendering code
- Make unified rendering the default
- Update documentation

## Success Metrics

1. **Performance**: No regression in rendering speed
2. **Correctness**: Pixel-perfect compatibility
3. **Memory**: Lightweight mode uses less memory than current choose()
4. **Code reduction**: Net decrease in code complexity
5. **Test coverage**: 100% coverage of rendering paths

## Risks and Mitigation

| Risk                   | Mitigation                         |
| ---------------------- | ---------------------------------- |
| Performance regression | Benchmark before/after each phase  |
| Visual differences     | Comprehensive visual testing suite |
| Breaking changes       | Feature flags for gradual rollout  |
| Complexity increase    | Keep phases small and focused      |

## Conclusion

Unifying the rendering paths through PixelChar provides a clean, performant architecture that's
ready for the future direct ANSI implementation. The phased approach ensures we can migrate safely
while maintaining compatibility and performance.

## Status Update (October 21, 2025)

### ✅ Phase 0 Complete - Foundation Laid

The groundwork for unified rendering has been established with commit e0d25552:

- **Unified style representation**: `TuiStyleAttribs` is now the canonical style type
- **Consolidated attributes**: `BlinkMode` and `Overline` fully integrated
- **Parser refactored**: Direct use of `TuiStyleAttribs` eliminates intermediate types
- **Test coverage**: All 2,058 tests passing - ANSI compliance verified

### 🎯 Next Steps

1. **Phase 1**: Standardize ASText's `ASTStyle` to use `TuiStyleAttribs` for full consolidation
2. **Phase 2**: Build `PixelCharRenderer` module for unified ANSI generation
3. **Phases 3-6**: Implement flexible buffer types and update rendering paths

### Key Insight

The consolidation of style attributes into a unified type has removed a major architectural redundancy.
Both ASText and TuiStyledText now have a clear path to share the same underlying style system, paving
the way for the unified PixelChar-based rendering architecture.
