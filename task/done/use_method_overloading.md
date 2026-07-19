# Task: Implement Coordinate Projection Extension Traits using Method Overloading

## Background

In Rust, traditional method overloading (defining multiple methods with the same name but
different parameter types on a single struct) is not supported.

To overcome this, R3BL TUI established the **trait-based ad-hoc polymorphism** pattern in
[`CanvasCameraExt`](file:///home/nazmul/github/roc/tui/src/core/coordinates/canvas/canvas_camera_ext.rs).
That trait parameterizes `CanvasCameraExt<TargetCoord>` over different coordinate types
(`CRow`, `CCol`, `CPos`), enabling unified method names like
`viewport.pan_to_keep_coord_in_view(target)` and `viewport.to_vp(target)`.

Currently,
[`Viewport`](file:///home/nazmul/github/roc/tui/src/core/coordinates/canvas/viewport.rs)
relies on separate type-suffixed methods for coordinate projection:

- `to_c_row` vs `to_c_col` vs `to_c_pos`
- `to_canvas_row_range` vs `to_canvas_col_range`
- `to_viewport_row` vs `to_viewport_col` vs `to_viewport_pos`
- `to_viewport_row_range` vs `to_viewport_col_range`

This task introduces
[`ViewportToCanvasExt`](file:///home/nazmul/github/roc/tui/src/core/coordinates/canvas/canvas_projection_ext.rs)
and
[`CanvasToViewportExt`](file:///home/nazmul/github/roc/tui/src/core/coordinates/canvas/canvas_projection_ext.rs)
to unify coordinate conversion and bounds validation on `Viewport` using trait-based
method overloading, completely removing the legacy type-suffixed inherent methods to
eliminate redundant dead code.

## Proposed Design

Create two complementary extension traits in
`tui/src/core/coordinates/canvas/canvas_projection_ext.rs`:

```rust
pub trait ViewportToCanvasExt<TargetCoord> {
    type CanvasResult;

    /// Projects a Viewport coordinate (or range) into a Canvas coordinate (or range).
    fn to_canvas(&self, target: TargetCoord) -> Self::CanvasResult;
}

pub trait CanvasToViewportExt<TargetCoord> {
    type ViewportResult;

    /// Projects a Canvas coordinate (or range) into a Viewport coordinate (or range).
    ///
    /// Returns `None` if the target coordinate falls outside the visible viewport window.
    fn to_viewport(&self, target: TargetCoord) -> Option<Self::ViewportResult>;
}
```

### Trait Implementations for `Viewport`

1. **`VPRow` / `CRow`**:
    - `to_canvas(VPRow) -> CRow` via `ViewportToCanvasExt<VPRow>`
    - `to_viewport(CRow) -> Option<VPRow>` via `CanvasToViewportExt<CRow>`

2. **`VPCol` / `CCol`**:
    - `to_canvas(VPCol) -> CCol` via `ViewportToCanvasExt<VPCol>`
    - `to_viewport(CCol) -> Option<VPCol>` via `CanvasToViewportExt<CCol>`

3. **`VPPos` / `CPos`**:
    - `to_canvas(VPPos) -> CPos` via `ViewportToCanvasExt<VPPos>`
    - `to_viewport(CPos) -> Option<VPPos>` via `CanvasToViewportExt<CPos>`

4. **`Range<VPRow>` / `Range<CRow>`**:
    - `to_canvas(&Range<VPRow>) -> Range<CRow>` via
      `ViewportToCanvasExt<RangeExclusive<VPRow>>`
    - `to_viewport(&Range<CRow>) -> Option<Range<VPRow>>` via
      `CanvasToViewportExt<RangeExclusive<CRow>>`

5. **`Range<VPCol>` / `Range<CCol>`**:
    - `to_canvas(&Range<VPCol>) -> Range<CCol>` via
      `ViewportToCanvasExt<RangeExclusive<VPCol>>`
    - `to_viewport(&Range<CCol>) -> Option<Range<VPCol>>` via
      `CanvasToViewportExt<RangeExclusive<CCol>>`

## Execution Plan

### Phase 1: Move Projection Implementation Logic into `canvas_projection_ext.rs`

- [x] Create `tui/src/core/coordinates/canvas/canvas_projection_ext.rs`.
- [x] Implement `ViewportToCanvasExt` and `CanvasToViewportExt` directly with core
      calculation logic for `Viewport` across `VPRow`/`CRow`, `VPCol`/`CCol`,
      `VPPos`/`CPos`, `Range<VPRow>`/`Range<CRow>`, and `Range<VPCol>`/`Range<CCol>`.
- [x] Add comprehensive unit tests in `canvas_projection_ext.rs` testing all coordinate
      variants and range projections.
- [x] Mandatory manual review for Phase 1:
    - [x] `tui/src/core/coordinates/canvas/canvas_projection_ext.rs`

### Phase 2: Remove Legacy Inherent Methods from `viewport.rs`

- [x] Delete all 10 type-suffixed inherent methods from `Viewport`: `to_c_row`,
      `to_c_col`, `to_c_pos`, `to_canvas_row_range`, `to_canvas_col_range`,
      `to_viewport_row`, `to_viewport_col`, `to_viewport_pos`, `to_viewport_row_range`,
      `to_viewport_col_range`.
- [x] Update unit tests in `viewport.rs` to use `.to_canvas(...)` and `.to_viewport(...)`.
- [x] Mandatory manual review for Phase 2:
    - [x] `tui/src/core/coordinates/canvas/viewport.rs`

### Phase 3: Migrate Internal Call Sites Across the Repository

- [x] Migrate
      `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs` to
      `.to_canvas(...)`.
- [x] Migrate `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
      to `.to_canvas(...)`.
- [x] Migrate `tui/src/core/pty/pty_mux/scrollback_amount.rs` to `.to_canvas(...)` and
      `.to_viewport(...)`.
- [x] Migrate `tui/src/core/pty/pty_mux/output_renderer.rs` to `.to_canvas(...)`.
- [x] Mandatory manual review for Phase 3:
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
    - [x] `tui/src/core/pty/pty_mux/output_renderer.rs`

### Phase 4: Module Export, Documentation & Comprehensive Verification

- [x] Declare and re-export `canvas_projection_ext` in
      `tui/src/core/coordinates/canvas/mod.rs`.
- [x] Re-export `ViewportToCanvasExt` and `CanvasToViewportExt` in
      `tui/src/core/coordinates/mod.rs` and crate root `tui/src/lib.rs`.
- [x] Update rustdoc documentation cross-references in `viewport.rs` and `canvas/mod.rs`.
- [x] Run `./check.fish --full` to verify typecheck, build, clippy, unit tests, doctests,
      doc build, and cross-platform compilation.
- [x] Mandatory manual review for Phase 4:
    - [x] `task/use_method_overloading.md`
