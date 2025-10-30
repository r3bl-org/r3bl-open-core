# Code Style & Conventions (r3bl-open-core)

## Key Guidelines from CLAUDE.md

### Module Organization
- **Prefer private modules with public re-exports** as the default pattern
- Use `#![cfg_attr(rustfmt, rustfmt_skip)]` to prevent reformatting in `mod.rs` when needed
- Conditional visibility for docs/tests: `#[cfg(any(test, doc))]` for public access in those contexts only

### Documentation Comments (Inverted Pyramid)
- **Trait/Module level**: Conceptual examples, workflows, visual diagrams, antipatterns
- **Method level**: Minimal syntax examples showing how to call the method
- Use ASCII diagrams to illustrate concepts
- Avoid duplication between levels

### Type Safety
- Use index types (0-based): `RowIndex`, `ColIndex`, `Index`
- Use length types (1-based): `RowHeight`, `ColWidth`, `Length`
- Use `.is_zero()` instead of `== 0`
- From `r3bl_tui`: ArrayBoundsCheck, CursorBoundsCheck, ViewportBoundsCheck, RangeBoundsExt, etc.

### Code Quality Standards
- Strict clippy lints enabled workspace-wide (pedantic + all)
- Production library code must use `#![cfg_attr(not(test), deny(clippy::unwrap_in_result))]`
- Comprehensive error handling - no unwrap in library code
- Debug implementations required via lint

### Platform-Specific Code
- Use `#[cfg(unix)]` and `#[cfg(windows)]` for platform-specific modules
- Use rustix for safe termios API (preferred over libc)
