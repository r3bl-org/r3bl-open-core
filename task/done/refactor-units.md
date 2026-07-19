<!-- cspell:words ofsbuf -->

# Task: Refactor Coordinate Traits and Storage Types

## Overview

This refactor will:

1. Introduce `ScreenCoordinate` for `u16`-backed screen / display dimensions.
2. Introduce `StorageCoordinate` for `usize`-backed memory and canvas dimensions.
3. Integrate existing `usize`-backed storage types into this new trait hierarchy.

Currently, the `r3bl_tui` crate forces every coordinate and dimension type to use 16-bit
integers (`u16`). Any type implementing the core numeric traits (`NumericConversions` and
`NumericValue`) must provide a way to convert to and from a `u16`.

While this 16-bit limit works perfectly for physical terminal screens (which never exceed
65,535 rows or columns), it creates a major problem for data storage. Internal memory
structures (like text documents or infinite scrollback histories) use standard 64-bit
sizes (`usize`) because they can easily hold more than 65,535 lines or bytes.

### Exhaustive Taxonomy of Unit Types

#### 1. Storage, Canvas & Document Memory Types :

`usize`-backed, primitive types implementing `StorageCoordinate`:

- **`ScrollbackAmount(pub usize)`**: Scrollback buffer history tracking (100,000+ lines).
- **`CanvasRowIndex(pub usize)`**: Absolute row index on the continuous storage buffer
  (refactored from `RowIndex` (`u16`), removing the 65,535 line storage cap).
- **`CanvasColIndex(pub usize)`**: Absolute column index on the continuous storage buffer
  (refactored from `ColIndex` (`u16`)).
- **`CanvasRowHeight(pub usize)`**: Absolute row height/extent on continuous storage
  buffer.
- **`CanvasColWidth(pub usize)`**: Absolute column width/extent on continuous storage
  buffer.
- **`CanvasPos { pub col_index: CanvasColIndex, pub row_index: CanvasRowIndex }`**:
  Absolute 2D position on the continuous storage buffer (refactored from `Pos`
  (`u16, u16`), allowing canvas storage positions beyond row 65,535).
- **`StorageLineLimit::Fixed(usize)`**: Storage history line capacity policy (refactored
  from `Length` (`u16`), removing the 65,535 line capacity cap).
- **`ByteIndex(pub usize)`**: UTF-8 string byte offsets in documents (>64 KB).
- **`ByteLength(pub usize)`**: UTF-8 string byte length measurements (>64 KB).
- **`ByteOffset(pub usize)`**: Byte boundary offsets in buffer segments (>64 KB).

#### 2. Physical Screen, Viewport & Terminal Hardware Types

`u16`-backed, implementing `ScreenCoordinate`:

- **Screen Window Indices**: `RowIndex`, `ColIndex`, `Index`, `SegIndex`.
- **Screen Window Dimensions**: `RowHeight`, `ColWidth`, `Length`, `SegLength`, `ChUnit`.
- **Viewport Relative Decorators**: `ViewportRowIndex`, `ViewportColIndex`,
  `ViewportPos { pub col_index: ViewportColIndex, pub row_index: ViewportRowIndex }`.
- **Terminal Hardware & ANSI Primitives**: `TermRow`, `TermCol`, `TermRowDelta`,
  `TermColDelta`, `CsiCount` (Note: `TermRow`, `TermCol`, and `CsiCount` implement
  `NumericConversions` directly, excluding `From<u16>` to respect `NonZeroU16`
  invariants).

## Implementation Plan

### [x] Phase 1: Trait Hierarchy Refactoring & `ScreenCoordinate` Definition

- [x] Update `NumericConversions` in
      `tui/src/core/coordinates/bounds_check/numeric_value.rs`: Remove mandatory
      `as_u16(&self) -> u16;` method; add default `try_as_u16(&self) -> Option<u16>`.
- [x] Update `NumericValue` in `tui/src/core/coordinates/bounds_check/numeric_value.rs`:
      Remove mandatory `+ From<u16>` and `+ From<usize>` trait bounds.
- [x] Define `ScreenCoordinate` sub-trait in `numeric_value.rs`:
      `pub trait ScreenCoordinate: NumericValue + From<u16> { fn as_u16(&self) -> u16; }`.
- [x] Define `StorageCoordinate` sub-trait in `numeric_value.rs`:
      `pub trait StorageCoordinate: NumericValue + From<usize> {}`.
- [x] Re-export `ScreenCoordinate` and `StorageCoordinate` in
      `tui/src/core/coordinates/mod.rs`, `tui/src/core/coordinates/bounds_check/mod.rs`,
      and `tui/src/lib.rs`.
- [x] Implement `ScreenCoordinate` for 16-bit screen primitives: `RowIndex`, `ColIndex`,
      `RowHeight`, `ColWidth`, `Length`, `Index`, `SegIndex`, `SegLength`, `ChUnit`,
      `ViewportRowIndex`, and `ViewportColIndex`. (Keep `TermRow`, `TermCol`,
      `TermRowDelta`, `TermColDelta`, and `CsiCount` implementing `NumericConversions`
      directly to respect `NonZeroU16` invariants, and implement `TryFrom<u16>` for them).
- [x] Remove `as_u16` from all existing `impl NumericConversions` blocks (in
      `index_and_length_impl_macros.rs`, `term_row.rs`, `term_col.rs`, `csi_count.rs`).
      Convert them to inherent methods where necessary if they aren't covered by
      `ScreenCoordinate`.
- [x] Replace `From<i32>` with `TryFrom<i32>` for `ChUnit` in `primitives/ch_unit.rs`.
- [x] Update `RangeIndexType` trait bound in
      `tui/src/core/coordinates/bounds_check/range_ext.rs`: Change
      `type IndexType: From<usize>` to `type IndexType: TryFrom<usize>` and update
      `as_index_iter()` to use `.try_from(v).unwrap()`.
- [x] Update `tui/src/core/coordinates/bounds_check/mod.rs` rustdocs so that the hierarchy
      of the traits, their relationships, and the structs that impl them are clearly
      described using the following ASCII diagram, and ensure that the rustdocs for the
      traits have intra doc links pointing to the exact heading of this diagram:
- [x] Run `./check.fish --full` to verify the code works and compiles cleanly before
      manual review by the user.
- [x] **Mandatory manual review:** Verify `ScreenCoordinate` definition and trait
      hierarchy compilation.
    - [x] `task/refactor-units.md`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/erase_mode.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/csi_codes/sequence.rs`
    - [x] `tui/src/core/coordinates/bounds_check/cursor_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/index_ops.rs`
    - [x] `tui/src/core/coordinates/bounds_check/length_ops.rs`
    - [x] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [x] `tui/src/core/coordinates/bounds_check/numeric_value.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range_bounds_check_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range_convert_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range_ext.rs`
    - [x] `tui/src/core/coordinates/buffer_coords/index_and_length_impl_macros.rs`
    - [x] `tui/src/core/coordinates/byte/byte_index.rs`
    - [x] `tui/src/core/coordinates/byte/byte_length.rs`
    - [x] `tui/src/core/coordinates/mod.rs`
    - [x] `tui/src/core/coordinates/primitives/ch_unit.rs`
    - [x] `tui/src/core/coordinates/vt_100_ansi_coords/csi_count.rs`
    - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_col.rs`
    - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_col_delta.rs`
    - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_row.rs`
    - [x] `tui/src/core/coordinates/vt_100_ansi_coords/term_row_delta.rs`
    - [x] `tui/src/lib.rs`
    - [x] `tui/src/core/common/common_math.rs`
    - [x] `tui/src/core/coordinates/percent_spec/pc.rs`
    - [x] `tui/src/core/pty/pty_mux/constants.rs`
    - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection_range.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection_support.rs`
    - [x] `tui/src/tui/syntax_highlighting/intermediate_types.rs`

### [x] Phase 2: Canvas, Storage & Byte Types Integration

- [x] Refactor `CanvasRowIndex(pub usize)` and `CanvasColIndex(pub usize)` in
      `canvas_coords.rs`: Wrap `usize` directly. Define paired
      `CanvasRowHeight(pub usize)` and `CanvasColWidth(pub usize)` to satisfy `IndexOps`
      and `LengthOps` associated types. Define
      `CanvasSize { pub col_width: CanvasColWidth, pub row_height: CanvasRowHeight }`.
      Implement `TryFrom<CanvasSize> for Size`, `From<Size> for CanvasSize`, and
      arithmetic operators (`Add`, `Sub`, `Mul`). Remove infallible
      `From<CanvasRowIndex> for RowIndex` and `From<CanvasColIndex> for ColIndex`.
      Implement `TryFrom<CanvasRowIndex> for RowIndex` and
      `TryFrom<CanvasColIndex> for ColIndex`. Implement `NumericConversions`,
      `NumericValue`, and `ArrayBoundsCheck` on all four types. Implement `IndexOps` for
      `CanvasRowIndex` and `CanvasColIndex`, and `LengthOps` for `CanvasRowHeight` and
      `CanvasColWidth`. Implement explicit arithmetic operators (`Add`, `Sub`,
      `AddAssign`, `SubAssign`) for `CanvasRowIndex` and `CanvasColIndex` with `usize` and
      `i32`.
- [x] Refactor `CanvasPos` in `canvas_coords.rs`: Define as
      `pub struct CanvasPos { pub col_index: CanvasColIndex, pub row_index: CanvasRowIndex }`.
      Update `canvas_pos()` helper constructor, remove `Deref<Target = Pos>`, implement
      `TryFrom<CanvasPos> for Pos`, implement
      `Add<CanvasRowIndex> for CanvasColIndex -> CanvasPos`, implement tuple conversions
      `From<(CanvasRowIndex, CanvasColIndex)>` and
      `From<(CanvasColIndex, CanvasRowIndex)>`, and implement `Add`/`Sub` arithmetic
      operators (`CanvasPos + CanvasPos`, `CanvasPos - CanvasPos`,
      `CanvasPos + CanvasColWidth`, `CanvasPos + CanvasRowHeight`,
      `CanvasPos + CanvasSize`, `CanvasPos - CanvasSize`).
- [x] Refactor `ViewportPos` in `viewport_coords.rs`: Define as
      `pub struct ViewportPos { pub col_index: ViewportColIndex, pub row_index: ViewportRowIndex }`.
      Update `vp_pos()` helper constructor, remove `Deref<Target = Pos>`, implement
      `From<ViewportPos> for Pos`, implement
      `Add<ViewportRowIndex> for ViewportColIndex -> ViewportPos`, implement tuple
      conversions `From<(ViewportRowIndex, ViewportColIndex)>` and
      `From<(ViewportColIndex, ViewportRowIndex)>`, and implement `Add`/`Sub` arithmetic
      operators (`ViewportPos + ViewportPos`, `ViewportPos - ViewportPos`,
      `ViewportPos + Size`, `ViewportPos + ColWidth`, `ViewportPos + RowHeight`).
- [x] Implement `NumericConversions`, `NumericValue`, `IndexOps`, `ArrayBoundsCheck`, and
      `ScreenCoordinate` on `ViewportRowIndex` and `ViewportColIndex` in
      `viewport_coords.rs`.
- [x] Implement `NumericConversions`, `NumericValue` (including requisite arithmetic
      traits `Add`, `Sub`, `CheckedAdd`, `CheckedSub`, `AddAssign`, `SubAssign`,
      `From<usize>`, `Ord`), `IndexOps`, `LengthOps`, `ArrayBoundsCheck`, and
      `StorageCoordinate` on `ScrollbackAmount` in `scrollback_amount.rs`.
- [x] Refactor `StorageLineLimit::Fixed(usize)` in `storage_line_limit.rs` to wrap
      `usize`. Update `calc_max_line_count` on `StorageLineLimit` to return
      `Option<usize>`. Update call sites in `growable_buffer.rs` and
      `scrollback_buffer.rs`.
- [x] Refactor `Viewport` struct in
      `tui/src/core/coordinates/canvas_viewport_coords/viewport.rs`: Remove `inner: Dims`.
      Store `origin_pos: CanvasPos` and `size: Size` directly. Update `get_origin_pos` to
      return `CanvasPos` and `set_origin_pos` to accept `&mut CanvasPos`. Implement
      `to_canvas_pos(&self, vp_pos: ViewportPos) -> CanvasPos` and
      `to_viewport_pos(&self, canvas_pos: CanvasPos) -> Option<ViewportPos>`. Update
      `OfsBufStorage::try_pan_viewport_to` in `storage/core.rs` and backends to accept
      `CanvasPos`.
- [x] Refactor `ByteIndex`, `ByteLength`, and `ByteOffset` in `byte_index.rs`,
      `byte_length.rs`, and `byte_offset.rs`: Inherit default `try_as_u16()` from
      `NumericConversions` and remove lossy
      `#[allow(clippy::cast_possible_truncation)] fn as_u16()` implementations. Remove
      lossy `From<ByteIndex> for Index`, `From<ByteIndex> for Length`, and
      `From<ByteLength> for Length` downcasts, replacing with `TryFrom` or `as_usize()`
      logic where appropriate.
- [x] Implement `StorageCoordinate` for `CanvasRowIndex`, `CanvasColIndex`,
      `CanvasRowHeight`, `CanvasColWidth`, `ByteIndex`, `ByteLength`, and `ByteOffset`.
- [x] Update macro generators in `index_and_length_impl_macros.rs`: Ensure
      `generate_index_type_impl!` generates `ScreenCoordinate` for 16-bit screen types.
      Remove infallible `From<usize>` and `From<i32>` from both macros, replacing with
      `TryFrom<usize>` and `TryFrom<i32>`. Ensure `#[doc = concat!(...)]` is used where
      appropriate on generated code, particularly for `convert_to_length` and
      `convert_to_index` trait methods.
- [x] Run `./check.fish --full` to verify the code works and compiles cleanly before
      manual review by the user.
- [x] **Mandatory manual review:** Verify storage coordinate, line limit & byte index
      implementations.
    - [x] `tui/src/core/coordinates/buffer_coords/index_and_length_impl_macros.rs`
    - [x] `tui/src/core/coordinates/canvas_viewport_coords/canvas_coords.rs`
    - [x] `tui/src/core/coordinates/canvas_viewport_coords/viewport_coords.rs`
    - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/storage_line_limit.rs`
    - [x] `tui/src/core/coordinates/canvas_viewport_coords/viewport.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/core.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/growable_buffer.rs`
    - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/flat_2d_array.rs`
    - [x] `tui/src/core/coordinates/byte/byte_index.rs`
    - [x] `tui/src/core/coordinates/byte/byte_length.rs`
    - [x] `tui/src/core/coordinates/byte/byte_offset.rs`
    - [x] `tui/src/lib.rs`

### [x] Phase 2.5: Primitive Casting Best Practices & Audit (clippy::as_conversions)

- [x] Rename `SaturatingCastTo...` traits to `NarrowingCastTo...` in
      `primitive_casting.rs` to better reflect bounds-checking/clamping logic.
- [x] Implement `WideningCastToU64` in `primitive_casting.rs` for `u8`, `u16`, `u32` to
      allow safe widening to `u64`.
- [x] Establish and document bulk refactoring threshold in `AGENTS.md` and
      `.agents/skills/batch-refactor-with-sub-agents/SKILL.md` (Native editors for 1-5
      files, custom Rust script in `tmpfs` for 6+ files. `sed`/`awk`/`perl`/`bash` are
      strictly prohibited).
- [x] Fix all `clippy::as_conversions` errors globally by replacing raw `as` casts with
      safe `Widening` / `Narrowing` trait methods across the repository.
- [x] Refactor `primitive_casting.rs` module structure: Localize
      `#[allow(clippy::as_conversions)]` to specific internal modules (`impl_lossy`,
      `impl_narrowing`), standardize `// XMARK` comments, update rustdocs for
      `WideningCastTo*`, and add tests for `WideningCastToU64`.
- [x] **Mandatory manual review:** Verify `XMARK` placement and trait correctness.
    - [x] `tui/src/core/common/primitive_casting.rs`
    - [x] `tui/src/tui/global_constants.rs`
    - [x] `AGENTS.md`

### [x] Phase 3: Range Extensions, Redundancy Elimination & Call Site Alignment

- [x] Implement `RangeIndexType` for `CanvasRowIndex`, `CanvasColIndex`,
      `ViewportRowIndex`, and `ViewportColIndex` in
      `tui/src/core/coordinates/bounds_check/range_ext.rs`.
- [x] Update `DecoratorRangeExt` in `decorator_range_ext.rs`: Update both `Range` and
      `RangeInclusive` implementations for `CanvasRowIndex` and `CanvasColIndex` to return
      `Range<usize>` and `RangeInclusive<usize>`.
- [x] Update `RangeConvertExt` in `range_convert_ext.rs`: Remove the blanket
      `impl<I: IndexOps>` entirely. Instead of a standalone macro, inject the
      `RangeConvertExt` implementation directly into the existing
      `generate_index_type_impl!` (using `1u16`) and the storage type macro (using
      `1usize`) to handle index arithmetic natively.
- [x] Update `range_construct_ext.rs`: Update `impl_range_construct_ext` macro invocations
      to pair `CanvasRowIndex` with `CanvasRowHeight` and `CanvasColIndex` with
      `CanvasColWidth`.
- [x] Remove ad-hoc `ScrollbackAmount::overflows()` in `scrollback_amount.rs` (replaced by
      `ArrayBoundsCheck::overflows`).
- [x] **Audit `.as_u16()` Call Sites (~50+ files)**: - Update ANSI generators
      (`ansi_input.rs`, `dsr.rs`, etc.) to handle `TryFrom<u16>` or `.as_u16()`
      gracefully. - Update VT-100 parser ops and CSI sequence generation. - Update
      terminal compositor (`compositor_render_ops_to_ofs_buf.rs`) and `paint.rs`. - Ensure
      screen types use `.as_u16()` and storage types use `.as_usize()` or
      `.try_as_u16().unwrap()`.
- [x] **Simplify Complex Trait Bounds**: Review trait bounds in the bounds checking
      modules and simplify fully qualified paths (e.g.,
      `<Self::LengthType as LengthOps>::IndexType` to `Self::IndexType`) leveraging
      bidirectional constraints, and ensure they are documented using the right-aligned
      block comment style.
- [x] Update module-level documentation, taxonomy tables, and visual diagrams in
      `tui/src/core/coordinates/bounds_check/mod.rs` and `canvas_viewport_coords/mod.rs`.
- [x] **Reorganize Range Module Structure**: - Rename `range_bounds_check_ext.rs` ->
      `range_bounds_check.rs`. - Create sub-module
      `tui/src/core/coordinates/bounds_check/range/` and move range files into it
      (`range_bounds_check.rs`, `range_construct_ext.rs`, `range_convert_ext.rs`,
      `range_ext.rs`). - Create `tui/src/core/coordinates/bounds_check/range/mod.rs` with
      public re-exports. - Re-export `range` sub-module in
      `tui/src/core/coordinates/bounds_check/mod.rs`.
- [x] **Rename Canvas and Viewport Module**: - Rename `canvas_viewport_coords` module to
      `canvas`. - Rename `wrapper_range_ext.rs` to `canvas_range_ext.rs` and
      `WrapperRangeExt` to `CanvasRangeExt`.
- [x] Run `./check.fish --full` to verify the code works and compiles cleanly before
      manual review by the user.
- [x] **Mandatory manual review:** Verify range module reorganization, code cleanup, and
      redundancy removal.
    - [x] `tui/src/core/coordinates/bounds_check/range/mod.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_bounds_check.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_construct_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_convert_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/range/range_ext.rs`
    - [x] `tui/src/core/coordinates/bounds_check/mod.rs`
    - [x] `tui/src/core/pty/pty_mux/scrollback_amount.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_range_ext.rs`
    - [x] `tui/src/core/coordinates/canvas/mod.rs`

### [x] Phase 4: Enforce `ViewportPos` & `CanvasPos` Across 2D Storage, Input, Layout & Rendering APIs

Update 2D character-level, cursor state, origin, layout, mouse input, and rendering
operations to enforce `ViewportPos` (for viewport-relative operations) and `CanvasPos`
(for canvas-absolute offsets):

- [x] Update `OfsBuf` character and line-level operations (`get_char`, `set_char`,
      `copy_char_range`, `bulk_ops`) to accept `ViewportPos`.
- [x] Update `OfsBufStorage` traits and implementations (`core.rs`, `types.rs`,
      `flat_2d_array_impl.rs`, `growable_buffer.rs`) to enforce `ViewportPos` for cursor
      tracking and `CanvasPos` for viewport panning math.
- [x] Update `ActiveBufferRouting` trait methods to accept `ViewportPos` in
      `active_buffer_routing.rs`.
- [x] Update `Viewport` math boundaries (`get_origin_pos`, `set_origin_pos`,
      `check_pan_validity`) to enforce `CanvasPos` in `viewport.rs` and
      `viewport_pan_validity.rs`.
- [x] Update `MouseInput::pos` to `ViewportPos` in `mouse_input.rs`, and update the PTY
      translation layer in `mouse_command.rs`.
- [x] Update Layout and Dialog structs (`FlexBox`, `RenderSurface`, `DialogBuffer`,
      `DialogEngine`) and their property definitions (`props.rs`) to strictly use
      `ViewportPos` for absolute screen positioning.
- [x] Update the core rendering instruction set (`RenderOpCommon`) variants like
      `MoveCursorPositionAbs` to strictly accept `ViewportPos` in `render_op_common.rs`.
- [x] Update backend painters and compositors (`crossterm`, `direct_to_ansi`,
      `compositor_render_ops_to_ofs_buf.rs`, `paint_impl.rs`) to accept and process
      `ViewportPos`.
- [x] Update all VT-100 performer implementations (`ops_impl_ofs_buf/`,
      `parser_global.rs`) to resolve ANSI cursor escape sequences against `ViewportPos`
      bounds.
- [x] Update `DialogEngine` state variables (`selected_row_index`,
      `scroll_offset_row_index`) to strictly use `CanvasRowIndex` in
      `dialog_engine_struct.rs` and update rendering logic in `dialog_engine_api.rs` to
      remove `.as_u16_narrowing()` cast.
- [x] Update 1D `RenderOpCommon` variants like `MoveCursorToColumn` to accept
      `ViewportColIndex` (and update `RenderOpCommonExt`).
- [x] Remove intermediate raw `ColIndex`/`RowIndex` downcasts inside backend painters
      (e.g., `crossterm_paint_render_op_impl.rs`), calling `.as_u16()` natively on the
      semantic types.
- [x] Refine the PTY translation layer in `mouse_command.rs` to pass
      `Viewport[Row/Col]Index` deeper into the input routing logic rather than immediately
      stripping to raw types.
- [x] Update `test_fixtures_ofs_buf_vt_100.rs` mock functions (`get_row`, `get_line`,
      etc.) to use `ViewportRowIndex` for alignment with `OfsBuf` line-level ops.
- [x] Fix incorrect `[column]: crate::ColIndex` and `[row]: crate::RowIndex` rustdoc links
      in `surface.rs` and `vt_100_impl_ansi_scroll_helper.rs`.
- [x] Update dependent call sites, mechanical shims, and unit test suites.

<!-- 🎯 wp1 -->

- [x] Add `VPRow` support in `ofs_buf/paint_impl.rs` (`RenderOpOutput` struct and
      `clear_for_new_line`).
- [x] Remove internal raw downcasts to `RowIndex`/`ColIndex` in `ofs_buf/char_ops.rs` and
      `ofs_buf/bulk_ops.rs`.
- [x] Update `vt_100_impl_clear_ops.rs` mathematical operations to use semantic `VPRow`
      instead of raw indices.
- [x] Update `render_op_common.rs` to use `VPHeight` and `VPWidth` type aliases.
- [x] Update `flat_2d_array` and its implementations to use Canvas coordinates (`CRow`,
      `CCol`, `CSize`), establishing a strict Canvas API.
- [x] Update `OfsBuf` boundary to correctly map Viewport coordinates (`VPPos`) into Canvas
      coordinates (`CPos`) when talking to `flat_2d_array`.
- [x] Remove `Viewport::size()` method and migrate all call sites to `get_width()` /
      `get_height()` (`VPWidth` / `VPHeight`).

<!-- 🎯 z1 -->

- [x] **Mandatory manual review:** Verify 2D `ViewportPos` and `CanvasPos` API
      enforcement. Focus primarily on translation layers (Layout and Compositors).
    - **High Impact Translation & Layout**
        - [x] `tui/src/tui/terminal_lib_backends/render_op/render_op_common.rs` (verify
              enum defs)
        - [x] `tui/src/core/coordinates/canvas/viewport.rs` (verify origin_pos bounds)
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/growable_buffer.rs`
              (verify panning)
        - [x] `tui/src/tui/layout/surface.rs` (verify origin_pos math)
        - [x] `tui/src/tui/layout/surface.rs` (fix rustdocs)
        - [x] `tui/src/tui/layout/flex_box.rs`
        - [x] `tui/src/tui/layout/partial_flex_box.rs`
        - [x] `tui/src/tui/layout/layout_and_positioning_traits.rs`
        - [x] `tui/src/tui/layout/props.rs`
        - [x] `tui/src/tui/terminal_lib_backends/render_op/render_op_common_ext.rs`
        - [x] `tui/src/core/coordinates/canvas/viewport.rs`
    - **Terminal Backends & Dialogs**
        - [x] `tui/src/tui/terminal_lib_backends/crossterm_backend/crossterm_paint_render_op_impl.rs`
        - [x] `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs`
        - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/direct_to_ansi_paint_render_op_impl.rs`
        - [x] `tui/src/tui/dialog/dialog_engine/dialog_engine_api.rs`
        - [x] `tui/src/tui/dialog/dialog_buffer/dialog_buffer_struct.rs`
        - [x] `tui/src/tui/dialog/dialog_engine/dialog_engine_struct.rs`
    - **OfsBuf Core & Operations**
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/core.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/types.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/storage/impls/flat_2d_array_impl.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/char_ops.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/line_level_ops.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/bulk_ops.rs`
        - [x] `tui/src/tui/terminal_lib_backends/ofs_buf/paint_impl.rs`
    - **Input & VT-100 Parsers**
        - [x] `tui/src/core/ansi/generator/ansi_output.rs`
        - [x] `tui/src/core/terminal_io/output_device.rs`
        - [x] `tui/src/core/pty/pty_mux/mux.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/protocols/params_ext.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_ansi_scroll_helper.rs`
        - [x] `tui/src/core/terminal_io/mouse_input.rs`
        - [x] `tui/src/core/pty/pty_mux/input_router/mouse_command.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/parser_state/parser_global.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/test_fixtures_ofs_buf_vt_100.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ofs_buf_vt_100/active_buffer_routing.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_char_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_control_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_cursor_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_da_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_dsr_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_line_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_margin_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_mode_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_osc_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_scroll_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_sgr_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_line_ops.rs`
        - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_cursor_ops.rs`

### [x] Phase 5: Verification & Quality Checks

- [x] Run `./check.fish --check` to verify typecheck.
- [x] Run `./check.fish --build` to verify compilation.
- [x] Run `./check.fish --clippy` to verify zero lint warnings.
- [x] Run `./check.fish --fmt` to format changed files.
- [x] Run `./check.fish --test` to verify full test suite passes.
- [x] Run `./check.fish --quick-doc` to verify documentation builds cleanly.
- [x] **Mandatory manual review:** Verify full test suite and quality checks pass.
    - [x] `task/refactor-units.md`
