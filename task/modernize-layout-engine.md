# Task: Modernize Layout Engine with Canvas & Viewport Coordinates

## Overview

The TUI layout engine (`Surface`, `FlexBox`, `PartialFlexBox`) currently operates strictly
within 16-bit physical screen coordinates (`Pos`, `Size`). It lacks native support for
scrolling, scrollable flex containers, or virtual canvas dimensions.

To integrate with the refactored coordinate system (`CPos` / `CanvasPos`, `CSize` /
`CanvasSize`, `VPPos` / `ViewportPos`, `VPSize` / `ViewportSize`, and `Viewport`), this
task modernizes `Surface` and `FlexBox` to perform layout staging in virtual `Canvas`
coordinate space (`usize`). This enables native scrollable sub-surfaces and overflow-safe
layout math while maintaining automatic `Viewport` clipping when generating
`RenderPipeline` instructions for physical screen rendering.

## Implementation Plan

### [ ] Phase 1: Foundation & Type Upgrades for `FlexBox` & `Surface`

- [ ] Refactor `FlexBox` in `tui/src/tui/layout/flex_box.rs`: Upgrade `origin_pos` to
      `CPos` (`CanvasPos`), `bounds_size` to `CSize` (`CanvasSize`), and
      `insertion_pos_for_next_box` to `Option<CPos>`.
- [ ] Add optional `maybe_viewport: Option<Viewport>` field to `FlexBox` to support
      scrollable container windows.
- [ ] Refactor `Surface` in `tui/src/tui/layout/surface.rs`: Store `canvas_size: CSize`
      alongside physical `box_size: Size` (`SurfaceBounds`).
- [ ] Update `SurfaceProps` and `FlexBoxProps` in `tui/src/tui/layout/props.rs` to accept
      `CSize` allocations and optional scrolling configurations.
- [ ] Update `tui/src/lib.rs` (Canvas vs Viewport Architecture section) to document how
      the layout engine utilizes Canvas and Viewport for scrollable areas.
- [ ] **Mandatory manual review:** Verify foundational layout types and struct definitions
      compile cleanly.
    - [ ] `tui/src/tui/layout/flex_box.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/layout/props.rs`

### [ ] Phase 2: Layout Engine Algorithms & Overflow-Safe Positioning

- [ ] Update `PerformPositioningAndSizing` and `LayoutManagement` in
      `tui/src/tui/layout/surface.rs`: Perform all child box insertion and flex allocation
      calculations in `CPos` space (`usize`).
- [ ] Update `update_insertion_pos_for_next_box()` to handle `CSize` additions without
      16-bit truncation risk.
- [ ] Add scrolling helper methods to `FlexBox` (`pan_viewport_by`, `set_viewport_origin`)
      to adjust scroll state on container viewports.
- [ ] Update `PartialFlexBox` in `tui/src/tui/layout/partial_flex_box.rs` to support
      `CPos` staging.
- [ ] **Mandatory manual review:** Verify positioning algorithms and layout math.
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/layout/layout_and_positioning_traits.rs`
    - [ ] `tui/src/tui/layout/partial_flex_box.rs`

### [ ] Phase 3: RenderPipeline Integration & Automatic Viewport Clipping

- [ ] Update render operation generation in `surface.rs`: Use
      `Viewport::to_viewport_pos()` to map child `CanvasPos` coordinates to screen `Pos`
      before appending to `RenderPipeline`.
- [ ] Implement automatic viewport clipping in `surface.rs` so operations falling outside
      a scrollable `FlexBox` viewport are omitted or clipped.
- [ ] Update layout macros (`surface!`, `flex_box!`) to support updated type signatures
      and optional viewport settings.
- [ ] Update layout unit tests in `surface.rs` and `flex_box.rs` to verify canvas
      positioning and scrolling behavior.
- [ ] **Mandatory manual review:** Verify render pipeline generation and clipping
      behavior.
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/terminal_lib_backends/render_pipeline.rs`

### [ ] Phase 4: Mouse Support & Focus Management Integration

- [ ] Update layout engine and component focusing logic to natively support
      `InputEvent::Mouse`.
- [ ] Implement hit-testing for `FlexBox` / `Surface` to map global terminal mouse
      coordinates to layout boundaries.
- [ ] Ensure that clicking within a specific layout boundary automatically updates the
      active focus context.
- [ ] Provide hooks for components (like the editor or lists) to receive translated local
      mouse events.
- [ ] **Mandatory manual review:** Verify mouse-focus integration.
    - [ ] `tui/src/tui/layout/flex_box.rs`
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] Focus management modules

### [ ] Phase 5: Tests, Documentation & Integration

- [ ] Write unit tests for `CSize` and `CPos` additions to ensure overflow safety.
- [ ] Write integration tests for `pan_viewport_by` to ensure layout components can be
      scrolled correctly.
- [ ] Add rustdoc comments to all modified traits and functions explaining the interplay
      between `CPos`, `CSize`, and `Viewport`.
- [ ] **Mandatory manual review:** Verify tests and documentation quality.
    - [ ] `tui/src/tui/layout/surface.rs`
    - [ ] `tui/src/tui/layout/flex_box.rs`

### [ ] Phase 6: Verification & Quality Checks

- [ ] Run `./check.fish --full` to verify builds, tests, clippy, and rustdoc generation.
- [ ] Update the docs here `@tui/src/lib.rs#L1044-1051` to reflect the layout code
      modernization using Canvas and Viewport architecture.
- [ ] **Mandatory manual review:** Verify full test suite and quality checks pass.
    - [ ] `task/modernize-layout-engine.md`
