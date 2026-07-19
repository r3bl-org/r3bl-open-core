# Task: Unify Viewport Coordinate Constructors

## Overview

Eliminate redundant unprefixed constructor aliases (`width`, `height`, `col`, `row`) for
viewport coordinate types ([`VPWidth`], [`VPHeight`], [`VPCol`], [`VPRow`]) in favor of
explicit, type-safe prefixed constructors (`vp_width`, `vp_height`, `vp_col`, `vp_row`).
This aligns viewport coordinates with canvas coordinates (`c_width`, `c_height`, etc.) and
VT-100 coordinates (`term_col`, `term_row`), removing confusing constructor aliases and
boilerplate manual helper functions.

## Implementation plan

### Phase 1: Update viewport coordinate macro invocations

- [x] Update macro invocations in `col_width.rs`, `col_index.rs`, `row_height.rs`,
      `row_index.rs` to generate `vp_width`, `vp_col`, `vp_height`, `vp_row`.
- [x] Remove redundant manual `pub fn vp_*` definitions from `col_width.rs`,
      `col_index.rs`, `row_height.rs`, `row_index.rs`.

### Phase 2: Systematic file-by-file refactoring across workspace

- [x] Manually migrate call sites and imports from `width(`, `height(`, `col(`, `row(` to
      `vp_width(`, `vp_height(`, `vp_col(`, `vp_row(` across all subsystems using native
      file editing tools.
- [x] Verify each subsystem incrementally with `./check.fish --check`,
      `./check.fish --test`, and `./check.fish --clippy`.
- [x] Run full workspace validation via `./check.fish --full`.

### Phase 3: Audit that this was done correctly

- [ ] Audit the git staged/unstaged (changed) files to double check the work that we have
      done so far. Report your findings and lets discuss before you implement the changes.
      Here are the code smells you are looking for:
    1. strange casts from the same type to the same type
    2. needless casts from one type to another
    3. strange widening or narrowing (unless they are semantic/domain correct)
    4. strange domain leaps / changes (viewport, canvas flip/swap that is out of place)
    5. any un-necessary casts
    6. any broken ascii diagrams
    7. any test code that were modified (when compared to the git head) to make broken
       code run (allowing broken production code to masquerade as working code due to
       corrupted test code)

### Phase 4: Remove semantically invalid point conversions (VPCol/VPRow/VPPos <-> CCol/CRow/CPos)

- [x] Remove implicit `From` and `TryFrom` trait implementations between Viewport Point
      types and Canvas Point types from
      `tui/src/core/coordinates/canvas/canvas_coords.rs`:
    - Remove `From<VPCol> for CCol` and `TryFrom<CCol> for VPCol`.
    - Remove `From<VPRow> for CRow` and `TryFrom<CRow> for VPRow`.
    - Remove `From<VPPos> for CPos` and `TryFrom<CPos> for VPPos`.
    - Remove `From<VPIndex> for CIndex`.
- [x] Retain dimension/span conversions (`VPWidth` <-> `CWidth`, `VPHeight` <-> `CHeight`,
      `VPSize` <-> `CSize`, `VPLength` <-> `CLength`) as scalar dimensions are
      translation-invariant.
- [x] Enforce the canonical spatial projection rule: ALL coordinate transformations
      between Viewport Space and Canvas Space must explicitly use a `Viewport` instance
      via `viewport.to_c_col(...)`, `viewport.to_c_row(...)`, `viewport.to_c_pos(...)`,
      `viewport.to_viewport_col(...)`, `viewport.to_viewport_row(...)`, and
      `viewport.to_viewport_pos(...)`.
- [x] Update any broken call sites across the workspace to use `Viewport` transformations
      instead of implicit 0-origin point conversions.
- [x] Run full automated validation via `./check.fish --check`, `./check.fish --test`,
      `./check.fish --clippy`, and `./check.fish --quick-doc`.

### Phase 5: Curated manual review

- [x] **Mandatory manual review:** Verify curated high-signal critical files for correct
      implementation, architectural integrity, and ensure no regressions.
    - [x] `tui/src/core/coordinates/viewport_coords/index_and_length_impl_macros.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/col_width.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/col_index.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/row_height.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/row_index.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_coords.rs`
    - [x] `tui/src/tui/layout/surface.rs`
    - [x] `tui/src/tui/layout/props.rs`
    - [x] `tui/src/core/coordinates/canvas/viewport.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_camera_ext.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
    - [x] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
    - [x] `tui/src/readline_async/readline_async_impl/line_state/cursor.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`

### Phase 6: Add generate_canvas_index_type_impl! macro for `C*` types

- [x] Create `generate_canvas_index_type_impl!` and `generate_canvas_length_type_impl!`
      macros in `tui/src/core/coordinates/canvas/canvas_coords.rs` (or dedicated macro
      module) to standardize Canvas space types (`CRow`, `CCol`, `CIndex`, `CWidth`,
      `CHeight`, `CLength`).
- [x] Ensure macro-generated Canvas types automatically implement:
    - Core accessors: `new()`, `get()`, `set()`, `as_usize()`.
    - `NarrowingCastToU16` for `as_u16_narrowing()`.
    - Numeric `From` traits (`From<usize>`, `From<u16>`, `From<i32>`,
      `From<CanvasType> for usize`).
    - Arithmetic operators: `Add`/`Sub`/`AddAssign`/`SubAssign` for `usize`, `i32`, and
      `Self` / paired length types.
- [x] Refactor `CRow`, `CCol`, `CIndex`, `CWidth`, `CHeight`, and `CLength` definitions in
      `tui/src/core/coordinates/canvas/canvas_coords.rs` to use the new macros.
- [x] Run full validation suite (`./check.fish --check`, `./check.fish --test`,
      `./check.fish --clippy`, `./check.fish --quick-doc`).

- [x] **Mandatory manual review:** Verify modified files for Phase 6.
    - [x] `tui/src/core/coordinates/canvas/canvas_coords.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
