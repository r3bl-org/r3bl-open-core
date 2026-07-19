Created At: 2026-08-04T13:06:05-05:00

# Task: Modernize Editor Using New Units (CPos & VPPos)

## Overview & Guiding Principle (The Domain Boundary)

Modernize the Editor subsystem by replacing legacy coordinate abstractions with standard
`CPos` and `VPPos` coordinate types. This eradicates domain-crossing math and simplifies
caret tracking, selection ranges, render caching, and scrolling math across
`EditorContent`, `EditorBufferMut`, and `EditorEngine`.

### The Core Principle - Separate Canvas vs Viewport Domains

- **Canvas Domain (`ZeroCopyGapBuffer`, `GapBufferLine`, `CPos`, `CCol`, `CRow`):** These
  represent the absolute, infinite underlying storage. They use 64-bit `usize` because
  lines can extend well beyond 65,535 columns.
- **Viewport Domain (`GCStringOwned`, `GraphemeString`, `VPPos`, `VPCol`):** These
  represent a finite, 16-bit projection of the Canvas that gets rendered to the screen.
- **The Boundary:** The Canvas and Viewport domains must **never** be mixed. A Canvas
  entity (`GapBufferLine`, `ZeroCopyGapBuffer`) cannot implement a Viewport trait
  (`GraphemeString`, `GraphemeDoc`) because an entire document line (or the document
  itself) might not fit into a 16-bit projection. The boundary occurs exclusively when the
  Editor Engine extracts a 16-bit `GCStringOwned` from the Canvas storage during
  rendering.

### Historical Context & Architectural Cleanup

Early versions of this Editor component used a simple `Vec<GCStringOwned>` (a list of
16-bit Viewport strings) as its primary text storage. To improve performance and memory
usage, `ZeroCopyGapBuffer` was introduced as a flat, highly-optimized byte array.

To allow the Editor to support both storage engines simultaneously, the `GraphemeDoc` and
`GraphemeString` traits were created to make the storage layer "pluggable". However,
because these traits were originally designed around `GCStringOwned`, they hardcoded
16-bit Viewport types (`VPRow`, `VPWidth`, etc.). When `ZeroCopyGapBuffer` (a massive
64-bit Canvas entity) implemented these traits, it was forced into a 16-bit straitjacket,
creating the 64K column limit bug.

Today, `EditorContent` has fully hardcoded `ZeroCopyGapBuffer` as its singular, concrete
storage engine. The "pluggable" abstraction is no longer used or needed. Therefore, as
part of this task, we are safely dropping the vestigial traits entirely:

1. `GraphemeDoc` and
2. `GraphemeDocMut`.

And removing the 16-bit `impl GraphemeString` block from `GapBufferLine`, finally freeing
the storage engine to use native 64-bit Canvas math. We are retaining `GraphemeString`,
`GraphemeStringMut`, and `GCStringOwned` since it is used extensively in the codebase.

### **Performance Impact of Dropping Traits:**

There will be **zero performance degradation and no new memory allocations (no memcpys)**
as a result of dropping these traits. The Editor Engine accesses the gap buffer directly,
which yields a `GapBufferLine` containing a zero-copy contiguous `&str` reference to the
text. Instead of calling a trait method (`GraphemeString::clip`), the Editor Engine will
simply call a native struct method (`GapBufferLine::clip_to_viewport`) which returns the
exact same zero-copy string slice. The only difference is that the native method will
safely accept a 64-bit `CCol` index rather than overflowing a 16-bit `VPCol`.

### **Architectural Justification: Memory and Scale**

This coordinate bifurcation is mathematically essential for scale:

1. **Canvas Domain (`usize`)**: By migrating the storage layer (`ZeroCopyGapBuffer`) to
   `usize`, the document size is constrained only by physical RAM. In Rust on a 64-bit
   machine, the absolute theoretical cap for a single allocation (like a `Vec` or
   `ZeroCopyGapBuffer`'s internal arrays) is `isize::MAX`, which is exactly **8
   Exabytes**! Because the Canvas storage is sparse (allocating memory only for the actual
   text bytes), a 128 GB workstation could load a massive ~100 GB log file containing tens
   of millions of lines without the 64-bit coordinate math ever overflowing.
2. **Viewport Domain (`u16`)**: The 16-bit Viewport limit acts as a critical safety bound
   for the dense terminal rendering grid (`OfsBuf`). In a dense visual grid, every
   coordinate cell must be allocated. If an optimized cell requires ~32 bytes (storing
   grapheme clusters, styles, foreground/background colors), rendering a theoretical
   `u16::MAX × u16::MAX` terminal screen would require `65,535 × 65,535 × 32 bytes` ≈
   **137.4 Gigabytes of RAM** just for a single frame! Thus, `u16` is the perfect
   scientific upper bound for dense terminal grids on modern hardware.

---

## Implementation Plan

### [x] Phase 0: Decouple `ZeroCopyGapBuffer` from the Viewport Domain

Before touching the editor coordinate math, we must fix the root cause of the 64K column
limit per line: `ZeroCopyGapBuffer`'s reliance on 16-bit Viewport structs (`Seg`,
`Length`, `VPWidth`) for its internal document metadata.

- [x] **Create `CIndex` and `CLength`**: Define generic 1D Canvas-domain equivalents of
      `Index` and `Length` in `tui/src/core/coordinates/canvas/canvas_coords.rs`.
    - They will be `usize`-backed.
    - Provide helper constructors `c_index()` and `c_len()`.
    - They will implement `NumericConversions`, `NumericValue`, and `StorageCoordinate`.
    - They will implement `IndexOps` and `LengthOps` to relate to each other.
    - Adhere to the file's architectural structure by placing implementations in inner
      modules (e.g., `mod impl_canvas_index { ... }` and
      `mod impl_canvas_length { ... }`).
    - Implement arithmetic operators within those modules using saturating math:
        - `CIndex - CIndex = CLength`
        - `CIndex + CLength = CIndex`
        - `CIndex - CLength = CIndex`
        - `CIndex + CIndex = CIndex`
        - `CLength + CLength = CLength`
        - `CLength - CLength = CLength`
        - `CLength * usize = CLength`
        - Standard `Add`, `Sub`, `AddAssign`, and `SubAssign` for `usize` and `i32`
          interactions.
    - Add tests for all the new code added here

- [x] **Create Canvas-Native Metadata Structs**: Define a new `DocSeg` (e.g. in
      `zcgb_line_metadata.rs`) that uses `usize`-backed Canvas types:
    ```rust
    pub struct DocSeg {
        pub start_byte_index: ByteIndex,
        pub end_byte_index: ByteIndex,
        pub display_width: CWidth,
        pub seg_index: CIndex,
        pub bytes_size: ByteLength,
        pub start_display_col_index: CCol,
    }
    ```
- [x] **Rewrite `LineMetadata`**: Update the internal storage metadata to use
      heavily-typed `usize`-backed aliases instead of 16-bit wrappers:
    ```rust
    pub struct LineMetadata {
        pub buffer_start: ByteIndex,
        pub content_byte_len: ByteLength,
        pub capacity: ByteLength,
        pub grapheme_segments: Vec<DocSeg>,
        pub display_width: CWidth,
        pub grapheme_count: CLength,
    }
    ```
- [x] **Refactor Segmentation Logic**: Update `ZeroCopyGapBuffer` internals (across
      `zcgb_*.rs`) to populate `DocSeg` without relying on `GCStringOwned`'s 16-bit
      segment array logic.

> ⚠️ **Build Status for Phase 0**: Do **NOT** run `./check.fish --check` or `cargo build`
> at the end of Phase 0. Changing core coordinate and storage types will cause temporary
> downstream compilation errors in Phase 2 and Phase 3 code.

### [x] Phase 1: Eliminate Type Aliases in Favor of Short Names

Prioritize Low Cognitive Load by removing type aliases. Use the short, elegant names as
the official primary structs. Suffixes inherently convey semantic meaning (Index vs
Length).

- [x] **Canvas Domain Updates** (`tui/src/core/coordinates/canvas/canvas_coords.rs`):
    - Rename `CanvasColIndex` to `CCol` and `CanvasRowIndex` to `CRow`.
    - Rename `CanvasColWidth` to `CWidth` and `CanvasRowHeight` to `CHeight`.
    - Set short names (`CRow`, `CCol`, `CWidth`, `CHeight`, `CPos`, `CSize`, `CIndex`,
      `CLength`) as primary structs with legacy type aliases.
- [x] **Viewport Domain Updates** (`tui/src/core/coordinates/viewport_coords/`):
    - Rename `ViewportColIndex` to `VPCol` and `ViewportRowIndex` to `VPRow`.
    - Rename `ViewportColWidth` to `VPWidth` and `ViewportRowHeight` to `VPHeight`.
    - Set short names as primary structs with legacy type aliases.
- [x] **Rustdoc Links**: Update primary struct definitions across coordinate subsystem.

> ⚠️ **Build Status for Phase 1**: Do **NOT** run `./check.fish --check` or `cargo build`
> at the end of Phase 1. The rename will cause massive downstream compiler errors until
> the rest of the Editor codebase is updated in subsequent phases.

---

### [x] Phase 2: Core Coordinate Translation Math & Buffer Trait Storage

Before touching the editor, establish the foundational math in the core coordinate system
and storage traits.

- [x] **Modernize Caret Types in `tui/src/core/coordinates/viewport_coords/caret.rs`**:
    - Update `CCaret` to wrap 64-bit `CPos` instead of 16-bit `VPPos`:

        ```rust
        pub struct VPCaret(pub VPPos);
        pub struct CCaret(pub CPos);

        pub fn vp_caret(arg_viewport_caret: impl Into<VPCaret>) -> VPCaret {
            arg_viewport_caret.into()
        }

        pub fn c_caret(arg_canvas_caret: impl Into<CCaret>) -> CCaret {
            arg_canvas_caret.into()
        }
        ```

- [x] **Implement Caret Operators & Conversions in `caret.rs`**:
    - `VPCaret + CPos` (origin) -> `CCaret` (widen `VPPos` to `CPos` and add)
    - `CPos + VPCaret` (origin) -> `CCaret` (commutative add)
    - `CCaret::to_viewport_caret(&self, viewport_origin: CPos, viewport_size: VPSize) -> Option<VPCaret>`:
      Method for safe off-screen caret checking using range containment instead of raw
      subtraction (do **NOT** implement `Sub` for `CCaret - CPos` since saturating
      subtract creates invalid on-screen positions for off-screen carets):

        ```rust
        pub fn to_viewport_caret(
            &self,
            viewport_origin: CPos,
            viewport_size: VPSize,
        ) -> Option<VPCaret> {
            let c_size = CSize::from(viewport_size);
            let col_range = viewport_origin.col_index
                ..(viewport_origin.col_index + c_size.col_width);
            let row_range = viewport_origin.row_index
                ..(viewport_origin.row_index + c_size.row_height);

            if col_range.contains(&self.0.col_index) && row_range.contains(&self.0.row_index) {
                let rel_col = self.0.col_index - viewport_origin.col_index;
                let rel_row = self.0.row_index - viewport_origin.row_index;

                let vp_col: u16 = rel_col.as_usize().try_into().ok()?;
                let vp_row: u16 = rel_row.as_usize().try_into().ok()?;

                Some(VPCaret(vp_pos(vp_col, vp_row)))
            } else {
                None
            }
        }
        ```

- [x] **API Boundary Decision & Storage Modernization**:
    - **The Dilemma:** `ZeroCopyGapBuffer` and `GapBufferLine` are **Canvas** entities
      (64-bit). `GraphemeString` and `GraphemeDoc` were **Viewport** traits (16-bit).
      Implementing these traits forced 64-bit storage into 16-bit limits, causing the 64K
      column limit bug.
    - [x] **Delete `GraphemeDoc` & `GraphemeDocMut` Traits**: Remove
          `tui/src/core/graphemes/traits/grapheme_doc.rs` entirely, and clean up exports
          in `tui/src/core/graphemes/traits/mod.rs` and `tui/src/core/graphemes/mod.rs`.
          `EditorContent` hardcodes `ZeroCopyGapBuffer`, so trait abstraction is
          vestigial.
    - [x] **Drop `impl GraphemeString` from `GapBufferLine`**: Remove
          `impl GraphemeString for GapBufferLine` from
          `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line.rs`. `GapBufferLine` is a
          64-bit Canvas line view and should not implement 16-bit Viewport traits.
    - [x] **Keep UI Viewport Traits**: Retain `GraphemeString`, `GraphemeStringMut`, and
          `GCStringOwned` in the codebase for standard UI components (inputs, dialogs).
    - [x] **Upgrade `ZeroCopyGapBuffer.line_count`**: Update `line_count` field in
          `ZeroCopyGapBuffer` (`zcgb_core.rs`) from 16-bit `Length` (`u16`) to 64-bit
          `CHeight` (`usize`).
    - [x] **Update `zcgb_*.rs` Method Signatures**: Update public method signatures across
          `zcgb_access_ops.rs`, `zcgb_basic_ops.rs`, `zcgb_insert_ops.rs`, and
          `zcgb_delete_ops.rs` to take and return Canvas types (`CRow`, `CCol`, `CIndex`,
          `CLength`, `CWidth`, `DocSeg`) instead of `VPRow`, `VPCol`, or `SegIndex`.
- [x] **Coordinate Construction**: When constructing dimensions, strictly use the helper
      functions `c_width()`, `c_height()`, `c_index()`, `c_len()`, `c_pos()`, `c_row()`,
      `c_col()`, `c_size()`, `c_caret()`, and `vp_caret()`.
- [x] **Verification**: 1. Check for dropped `.as_index_iter()` calls on coordinate
      ranges. 2. Check for `crate::<T>` prefix removal, ensuring clean imports instead.

> ⚠️ **Build Status for Phase 2**: Do **NOT** run `./check.fish --check` or `cargo build`
> at the end of Phase 2. Changing core coordinate and storage types will cause temporary
> downstream compilation errors in subsequent phases.

---

### [x] Phase 3: Editor Content, State & Mutation Validation Modernization

Update core state structures and the mutation validation layer.

- [x] **`buffer_struct.rs` (`EditorContent`)**: Define and synchronize components:
    1. `canvas_caret: CCaret` (`usize` absolute canvas position stored)
    2. `viewport_origin: CPos` (`usize` scrolling offset)
    - `viewport_caret` should be derived on demand via `get_viewport_caret(&self)` (which
      projects `canvas_caret` using `viewport_origin`), rather than stored as a third
      field.
- [x] **`validate_buffer_mut.rs` (`EditorBufferMut`, `EditorBufferMutWithDrop`,
      `EditorBufferMutNoDrop`)**:
    - Refactor `EditorBufferMut` fields from `viewport_caret: &'a mut VPCaret` to
      `canvas_caret: &'a mut CCaret`.
    - Update `perform_validation_checks_after_mutation()`,
      `adjust_caret_col_if_not_in_bounds_of_line()`,
      `adjust_caret_col_if_not_in_middle_of_grapheme_cluster()`, and
      `is_scroll_offset_in_middle_of_grapheme_cluster()` to operate on `canvas_caret`
      using type-safe bounds checking (`overflows()`, `underflows()`, and `Range`
      containment) rather than raw `<` or `>` operators.
- [x] **`selection_range.rs`, `selection_support.rs` & `selection_list.rs`**:
    - Update `SelectionListItem` key type to `CRow` (`usize`).
    - Update `SelectionList` methods (`get`, `insert`, `remove`, `locate_row`,
      `get_selected_lines`) to take `CRow`.
    - Update `SelectionRange::start()`, `SelectionRange::end()`, and `as_tuple()` return
      types to `CCol` (`usize`).
- [x] **`clipboard_support.rs`**: Update clipboard extraction logic to use the new `CCol`
      and `CRow` types.
- [x] **`sizing.rs`**: Update sizing calculations to use and return `CSize` / `CRow`
      rather than legacy `ChUnit` bounds.
- [x] **`render_cache.rs`**: Ensure cache keys use proper `CPos` types instead of legacy
      positional assumptions.
- [x] Run `./check.fish --full` to verify everything builds, tests pass, and documentation
      compiles cleanly.

---

### [x] Phase 4: Engine Math Migration & Ripple Effect

Refactor engine code and caret location helpers to consume `CPos` and `CCaret`.

**Strict Rule:** Do NOT cast down to `u16` (`.as_u16_narrowing()`) to make the math
compile. Instead, bubble the `CPos` (`usize`) types up the call stack by changing function
signatures. Do NOT use raw `<`, `>`, `<=`, `>=` operators for bounds or position checking;
use type-safe `underflows()`, `overflows()`, and `Range` containment.

- [x] **`caret_locate.rs`**: Update `locate_col()`, `locate_row()`, `col_is_at_start()`,
      `col_is_at_end()`, `row_is_at_top()`, and `row_is_at_bottom()` to accept `CCaret`
      and `CRow`.
- [x] **`caret_mut.rs` & `content_mut.rs`**: Rewrite local arithmetic and bounds checking
      using type-safe coordinate math (`underflows()`, `overflows()`, and `Range`
      containment).
- [x] **`scroll_editor_content.rs`**: Eradicate manual `ChUnit` bounds math for scrolling.
      Use `CanvasPanningExt::pan_to_include` and coordinate translation methods. Update
      `set_caret_col_to()` and `set_caret_row_to()` signatures to accept `CCol` and `CRow`
      respectively. You MUST convert `VPSize` bounds to `CSize` using
      `CSize::from(vp_size)` before comparing against `CCaret` or panning.
- [x] **`validate_scroll_on_resize.rs`**: Update scroll adjustment logic on window resize
      to operate on `CCaret`.
- [x] **`select_mode.rs`**: Update selection anchor logic to track positions via `CCaret`.
- [x] **`engine_internal_api.rs`, `engine_public_api.rs` & `editor_event.rs`**: Update
      event handling. Audit `EditorEvent` to ensure it carries screen-relative `VPPos`
      positions. When mouse click events arrive, you MUST translate these by adding
      `viewport_origin` to construct a `CCaret` before passing to the engine.
- [x] **`engine_public_api.rs` (Rendering Pipeline)**: Update the rendering loop. The
      engine MUST translate `CCaret` into `VPCaret` to draw the cursor. Cursor drawing
      must be gated behind
      `if let Some(screen_caret) = canvas_caret.to_viewport_caret(viewport_origin, viewport_size)`
      to prevent crashing or mis-rendering off-screen carets.
- [x] Run `./check.fish --full` to verify everything builds, tests pass, and documentation
      compiles cleanly.

---

### [x] Phase 5: Architectural Audits, Documentation & Polish

- [x] **Audit Narrowing Casts**: Perform a global audit of the git diff to explicitly
      verify that no `.as_u16_narrowing()` or `.try_as_u16().unwrap()` shortcuts were
      inserted in the editor logic or coordinate math. Narrowing is ONLY permitted at the
      absolute boundary layer where `RenderOp`s are emitted to the terminal backend.
- [x] **Audit Domain Boundary Cleanliness**: Verify strict Separation of Concerns (SOC)
      between Canvas and Viewport domains:
    - Canvas entities (`ZeroCopyGapBuffer`, `EditorContent`, `LineMetadata`) MUST only
      consume/return Canvas types (`CPos`, `CCaret`, `CCol`, `CRow`, `CWidth`, `CHeight`).
    - Viewport types (`VPPos`, `VPCaret`, `VPCol`, `VPRow`, `VPWidth`) MUST be strictly
      isolated to rendering output and terminal mouse event handling.
- [x] **Audit Redundant `.as_usize()` & Unwrapping**: Search for redundant `.as_usize()`
      calls on newtype wrappers where math operators (`Add`, `Sub`, `IndexOps`) or helper
      constructors (`c_col`, `c_width`, `c_caret`) already exist.
- [x] **Audit & Flatten Multi-Step Conversions**: Search for roundabout conversions (e.g.,
      `CCol` -> `usize` -> `u16` -> `VPCol`) and flatten them to direct type conversions
      (`CSize::from(vp_size)`, `canvas_caret.to_viewport_caret(origin, size)`).
- [x] **Audit Suspicious Boundary-Crossing Math**: Search for manual coordinate arithmetic
      (e.g., `caret - origin`) performed outside dedicated conversion/panning methods.
      Ensure all panning uses `CanvasPanningExt::pan_to_include`.
- [x] **Audit Mixed Length/Index Math**: Ensure there is no conceptual mixing of `Index`
      and `Length` types (e.g., initializing a countdown `Length` tracker with an `Index`
      value).
- [x] **Audit Type-Safe Bounds Checks**: Ensure code strictly uses type-safe bounds checks
      (`is_overflowed_by`, `overflows`, `check_range_is_valid_for_length`,
      `RangeBoundsExt`) instead of raw comparators (`<`, `>`, `<=`, `>=`) which are
      inelegant and hard to read, understand, and debug.
- [x] **Audit Clean Imports**: Ensure no inline absolute crate paths (`crate::CPos`,
      `crate::CCaret`) litter function signatures or bodies; verify clean `use` statements
      at top of files.
- [x] **Audit RAII Guard Lifetimes**: Verify that all mutation operations using
      `buffer.get_mut(...)` or `EditorBufferMutWithDrop` enclose mutations inside explicit
      `{}` scope blocks so drop-time validation assertions execute cleanly before
      subsequent caret adjustments or queries.
- [x] **Audit Relative Cursor Movement Safety**: Verify relative cursor movement code
      strictly uses type-safe `TermRowDelta` and `TermColDelta` types instead of raw
      signed/unsigned integer math to prevent CSI zero-movement bugs (`\x1b[0C`).
- [x] **Audit Zero-Allocation Hot Paths**: Verify that rendering pipelines
      (`engine_public_api.rs`) and zero-copy gap buffer lookups (`zcgb_access_ops.rs`) do
      not perform temporary heap allocations (e.g., `format!`, intermediate `String`s, or
      `Vec`s) during rendering or caret navigation.
- [x] **Audit Unit Test & Doctest Coverage**: Verify that every new coordinate struct,
      conversion method (`to_viewport_caret`), and spatial lookahead helper
      (`lookahead_wide_segment_to_right`) has comprehensive unit tests covering edge cases
      (e.g., origin at (0,0), off-screen carets, distant emojis, multi-byte graphemes).
- [x] **`tui/src/lib.rs`**: Update the _Canvas vs Viewport Architecture_ section to
      document how the editor component leverages the type system to prevent coordinate
      space mixing.

### [x] Phase 6: Fix problems found in audit

#### [x] 1. Narrowing Casts (`as_u16_narrowing`)

**Finding:** Violation. `.as_u16_narrowing()` is heavily used across the
`zero_copy_gap_buffer` module (`zcgb_adapters.rs`, `zcgb_basic_ops.rs`, `zcgb_core.rs`)
and `cur_index.rs`.

**Details:** The author's intent behind these casts was to force native `usize` lengths
(like `versions.len()` or `line_index`) into the coordinate newtypes (which were
originally designed around `u16` terminal limits) to satisfy the compiler without having
to refactor the internal types. For example: `len((versions.len()).as_u16_narrowing())`.
This is incorrect because Canvas internals should operate on unbounded `usize` natively,
not terminal limits.

**Plan:** Remove all `.as_u16_narrowing()` calls from `zero_copy_gap_buffer` and
`editor_engine` logic. Canvas should strictly use `usize` (via newtypes) internally and
only narrow to `u16` at the `engine_public_api.rs` boundary when emitting `RenderOp`s. No
`try_as_u16().unwrap()` calls were found.

**Mandatory manual review:**

- [x] `tui/src/tui/editor/editor_buffer/history/history_cursor.rs`
- [x] `tui/src/tui/editor/editor_buffer/history/editor_history.rs`
- [x] `tui/src/core/common/telemetry.rs`
- [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
- [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
- [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_core.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_delete_ops.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line_metadata.rs`
- [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`

#### [x] 2. Redundant `.as_usize()` & Unwrapping

**Finding:** Violation. There are many `.as_usize()` calls on newtype wrappers used in
arithmetic (e.g., `canvas_caret.col_index.as_usize() + col_amt_usize` in
`scroll_editor_content.rs`).

**Details:** The author was attempting to perform basic arithmetic (addition, subtraction)
by extracting the inner primitive value using `.as_usize()`. This bypasses the type system
because the newtypes likely lack the necessary `Add`/`Sub` trait implementations or
dedicated helper methods.

**Plan:** Replace `.as_usize()` based math with math operators (`Add`, `Sub`, etc.) if
implemented on the newtypes, or use dedicated helper methods (like `inc_caret_row`). Leave
`.as_usize()` calls that are purely for `assert_eq2!` in tests or `Debug` formatting.

#### [x] 3. Multi-Step Conversions & Suspicious Math

**Finding:** Minor Violations. Found manual coordinate manipulation outside of dedicated
panning methods (e.g., bounds checking manually).

**Details:** We found chained conversions in `scroll_editor_content.rs` and
`zcgb_basic_ops.rs` such as `c_row((base_row_index.as_usize() + 1).as_u16_narrowing())`.
The author is converting a Canvas type to `usize`, performing primitive math, and then
forcibly narrowing it back to a Canvas type, completely subverting type safety.

**Plan:** We will comb through `caret_locate.rs`, `scroll_editor_content.rs`, and
`zero_copy_gap_buffer` to replace any manual arithmetic with canonical methods like
`canvas_caret.to_viewport_caret(origin, size)` and `pan_to_include`.

#### [x] 4. Type-Safe Bounds Checks

**Finding:** Violation. Found raw comparators (`<`, `>`, `<=`, `>=`) in `caret_locate.rs`,
`scroll_editor_content.rs`, `validate_buffer_mut.rs`.

**Details:** The author was manually performing coordinate bounds checks by extracting the
primitives (e.g., `if canvas_caret.col_index.as_usize() >= vp_right_edge`). This indicates
they were treating the coordinates as raw integers rather than leveraging the rich,
type-safe bounds-checking traits explicitly designed for these types.

**Plan:** Replace raw comparators with type-safe bound check methods such as
`is_overflowed_by()`, `overflows()`, and `check_range_is_valid_for_length()`.

#### [x] 5. Clean Imports

**Finding:** Violation. Several inline absolute crate paths were found (e.g.,
`crate::CPos`, `crate::CCol`) in:

- `buffer_struct.rs`
- `selection_list.rs`
- `selection_range.rs`
- `validate_buffer_mut.rs`
- `validate_scroll_on_resize.rs`
- `zcgb_line_metadata.rs`

**Plan:** Remove inline absolute paths and add clean `use crate::{...};` statements at the
top of these files.

#### [x] 6. RAII Guard Lifetimes

**Finding:** Violation (Needs Review). Found many mutations running directly off
`buffer_mut.inner` without explicit `{}` scope blocks in files like `content_mut.rs`.

**Plan:** We will carefully review and enclose mutation logic using `get_mut(...)` or
`EditorBufferMutWithDrop` inside explicit `{}` blocks to ensure drop-time validation
assertions execute before subsequent queries.

---

### [x] Phase 7: Verification & Quality Checks

- [x] Run `./check.fish --all` to verify everything builds, tests pass, and documentation
      compiles cleanly.
- [x] Run all the examples in `tui/examples/` to verify the editor behaves correctly in a
      real terminal. There are bound to be bugs / visual artifacts due to this massive
      refactor.
- [x] **Mandatory manual review:** Complete manual review of all modified files.
    - [x] `tui/src/core/coordinates/canvas/canvas_coords.rs`
    - [x] `tui/src/core/coordinates/canvas/canvas_panning.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/caret.rs`
    - [x] `tui/src/core/coordinates/viewport_coords/pos.rs`
    - [x] `tui/src/core/graphemes/mod.rs`
    - [x] `tui/src/tui/editor/editor_buffer/buffer_struct.rs`
    - [x] `tui/src/tui/editor/editor_buffer/caret_locate.rs`
    - [x] `tui/src/tui/editor/editor_buffer/history.rs`
    - [x] `tui/src/tui/editor/editor_buffer/render_cache.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection_list.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection_range.rs`
    - [x] `tui/src/tui/editor/editor_buffer/selection_support.rs`
    - [x] `tui/src/tui/editor/editor_buffer/sizing.rs`
    - [x] `tui/src/tui/editor/editor_component/editor_component_struct.rs`
    - [x] `tui/src/tui/editor/editor_component/editor_event.rs`
    - [x] `tui/src/tui/editor/editor_engine/caret_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/content_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_internal_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/engine_public_api.rs`
    - [x] `tui/src/tui/editor/editor_engine/scroll_editor_content.rs`
    - [x] `tui/src/tui/editor/editor_engine/select_mode.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_buffer_mut.rs`
    - [x] `tui/src/tui/editor/editor_engine/validate_scroll_on_resize.rs`
    - [x] `tui/src/tui/editor/test_fixtures_editor.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/mod.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_access_ops.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_adapters.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_basic_ops.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_core.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_delete_ops.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_insert_ops.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_line_metadata.rs`
    - [x] `tui/src/tui/editor/zero_copy_gap_buffer/zcgb_seg_builder_ops.rs`
    - [x] `task/modernize-editor-using-new-units.md`
