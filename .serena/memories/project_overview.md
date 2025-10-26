# r3bl-open-core Project Overview

## Project Purpose
A comprehensive Rust TUI (Terminal User Interface) framework and tools, featuring:
- Advanced terminal rendering pipeline with semantic type separation
- Async readline and interactive selection components
- VT-100 ANSI parser and terminal abstraction layer

## Tech Stack
- **Language**: Rust (MSRV: Current nightly/stable)
- **Terminal Backend**: crossterm (with Termion support planned)
- **Async Runtime**: tokio
- **Build System**: Cargo with workspace (tui + cmdr crates)
- **Key Libraries**: miette (diagnostics), smallvec (InlineVec), r3bl_rs_utils_core

## Key Architecture Components

### Rendering Pipeline (NEW - Task Complete)
1. **RenderOpCommon** - 27 shared operations used in both contexts
2. **RenderOpIR** - App/Component-level operations (Intermediate Representation)
3. **RenderOpOutput** - Terminal/Backend-level operations (low-level optimized)
4. **OffscreenBuffer** - 2D pixel grid for composition
5. **Terminal Executor** - Executes output operations via crossterm

### Critical Components
- **OffscreenBuffer** (ofs_buf): 2D pixel character grid with terminal state
- **RenderPipeline**: Collects render operations by z-order (Background, Normal, Glass)
- **Compositor**: Converts RenderOpIR → OffscreenBuffer
- **Backend Converter**: Scans OffscreenBuffer → produces RenderOpsOutput
- **Terminal Executor**: Executes RenderOpsOutput via crossterm

### Async Subsystems
- **Readline**: Line editor with async interface
- **Choose**: Interactive list selector with single/multi-select
- **SelectComponent**: Direct terminal renderer (bypasses RenderOps architecture)
- **SharedWriter**: Thread-safe output coordination

## Code Style & Conventions

### Type Safety
- Use index types (0-based): RowIndex, ColIndex
- Use length types (1-based): RowHeight, ColWidth
- Traits for bounds checking: ArrayBoundsCheck, CursorBoundsCheck

### Module Organization
- **Pattern**: Private modules + public re-exports
- **Benefits**: Clean API, refactoring freedom, no name conflicts
- **Example**: csi_codes mod with private submodules, public re-exports

### Documentation
- **Inverted pyramid**: High-level at trait/module, implementation details at method level
- **Examples**: Conceptual at trait level, syntax at method level
- **ASCII diagrams**: Use for visual explanation of concepts

## Suggested Commands

### Code Quality & Testing
- `cargo check` - Fast typecheck
- `cargo build` - Compile
- `cargo test --no-run` - Compile test code
- `cargo clippy --all-targets` - Find lints
- `cargo clippy --fix --allow-dirty` - Auto-fix lints
- `cargo doc --no-deps` - Generate docs
- `cargo test --all-targets` - Run tests (excludes doctests)
- `cargo test --doc` - Run doctests

### Performance Analysis
- `cargo bench` - Run benchmarks
- `cargo flamegraph` - Profile code

### Build Optimizations
- sccache: Shared compilation cache
- wild-linker: Fast alternative linker (Linux)
- Auto-activated via bootstrap.sh

## Important Files & Locations

### Render System
- `tui/src/tui/terminal_lib_backends/render_op.rs` - RenderOp types (new 3-enum architecture)
- `tui/src/tui/terminal_lib_backends/render_pipeline.rs` - RenderPipeline with RenderOpsIR
- `tui/src/tui/terminal_lib_backends/compositor_render_ops_to_ofs_buf.rs` - IR → OffscreenBuffer
- `tui/src/tui/terminal_lib_backends/crossterm_backend/offscreen_buffer_paint_impl.rs` - OffscreenBuffer → RenderOpsOutput
- `tui/src/tui/terminal_lib_backends/crossterm_backend/paint_render_op_impl.rs` - Executes RenderOpsOutput

### Async Components
- `tui/src/readline_async/` - Main readline/choose implementation
- `tui/src/readline_async/choose_impl/select_component.rs` - Direct terminal renderer
- `tui/src/readline_async/readline_async_impl/` - Readline implementation

## Project Guidelines from CLAUDE.md
- Always ask for clarification on important decisions
- Use bounds checking traits (ArrayBoundsCheck, etc.) from units/bounds_check/
- Run full build + tests after completing tasks
- Private modules + public re-exports for clean API
- Inverted pyramid documentation structure
