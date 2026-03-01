<!-- cspell:words openpty -->

# Consolidate all PTY creation and spawning through PtyPair

## Status: DONE

## Problem

Production code bypassed `PtyPair` entirely. It used `create_pty_pair()` to get raw
`(Controller, Controlled)` tuples and `spawn_command_in_pty()` to spawn commands on bare
`Controlled` values. This duplicated the spawn-and-close logic that
`PtyPair::spawn_command_and_close_controlled()` already provided, and lost the safety
guarantees that `PtyPair` enforces.

Two parallel APIs doing the same thing:

```rust
// Production (bypassed PtyPair):
let (controller, controlled) = create_pty_pair(size)?;
let child = spawn_command_in_pty(controlled, command)?;

// Test code (used PtyPair):
let mut pty_pair = PtyPair::from(raw_pair);
let child = pty_pair.spawn_command_and_close_controlled(cmd)?;
```

## Goal

One API for everything: `PtyPair`. Delete `create_pty_pair()` and
`spawn_command_in_pty()`. Production and test code use the same path.

After consolidation, `PtyPair`'s public API is:

| Method                                   | Purpose                                                           |
| :--------------------------------------- | :---------------------------------------------------------------- |
| `new_with_size(Size)`                    | Create a PTY pair with explicit dimensions                        |
| `new_default()`                          | Create a PTY pair with standard 80x24 size                        |
| `spawn_command_and_close_controlled(cmd)` | Spawn child + close controlled fd                                |
| `controller()` / `controller_mut()`      | Borrow controller                                                 |
| `into_controller()`                      | Take owned controller (drops controlled if still open)            |

Every codepath either keeps the controlled fd closed or closes it automatically. The
controlled-fd deadlock is **structurally impossible** -- no caller discipline required.

Removed from public API:

| Method                               | Why removed                                                                                                                                                                  |
| :----------------------------------- | :--------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `split()`                            | Returns raw `(Controller, Controlled)` -- reintroduces the deadlock footgun. Only caller (`read_lines_windows`) replaced by `into_controller()`.                             |
| `close_controlled()`                 | Only called internally by `spawn_command_and_close_controlled()`. Inlined as `drop(self.maybe_controlled.take())`.                                                           |
| `controlled()` / `controlled_mut()`  | Only called internally by `spawn_command_and_close_controlled()`. No external callers after consolidation.                                                                   |

Deleted files:

| File                             | Why deleted                                                                                                  |
| :------------------------------- | :----------------------------------------------------------------------------------------------------------- |
| `tui/src/core/pty/pty_common_io.rs` | Both functions (`create_pty_pair`, `spawn_command_in_pty`) replaced by `PtyPair` methods. Tests moved to `pty_types.rs`. |

## Changes

### Phase 1: Replace `portable_pty::PtySize` with r3bl `Size` in public APIs

`portable_pty::PtySize` became a private implementation detail. All public APIs use
`crate::Size` instead. The helper functions `size()`, `width()`, and `height()` provide
ergonomic construction (e.g., `size(width(80) + height(24))`).

**`tui/src/core/pty/pty_core/pty_types.rs`**

Added `From<Size> for portable_pty::PtySize` -- single conversion point for the entire
crate. `ChUnitPrimitiveType` is `u16`, same as `PtySize` fields, so no truncation:

```rust
impl From<Size> for portable_pty::PtySize {
    fn from(size: Size) -> Self {
        Self {
            rows: size.row_height.0.as_u16(),
            cols: size.col_width.0.as_u16(),
            pixel_width: 0,
            pixel_height: 0,
        }
    }
}
```

Added `use crate::{Size, height, size, width};` to imports.

**`tui/src/core/pty/pty_config.rs`**

- `PtyConfigOption::Size(Size)` (was `PtySize`)
- `PtyConfig.pty_size: Size` (was `PtySize`)
- `PtyConfig::get_pty_size() -> Size` (was `PtySize`)
- `From<Size> for PtyConfigOption` (was `From<PtySize>`)
- Removed `use portable_pty::PtySize`
- Added `use crate::{Size, height, size, width};`
- Updated all doc examples to use `size(width(80) + height(24))`

**`tui/src/core/pty/pty_core/pty_input_events.rs`**

- `PtyInputEvent::Resize(Size)` (was `PtySize`)
- Added `Size` to the existing `use crate::{...};` import block
- Removed `use portable_pty::PtySize`

**`tui/src/core/pty/pty_read_write.rs`**

- `spawn_read_write(pty_size: Size)` (was `PtySize`)
- Removed `use portable_pty::PtySize`
- Added `Size` to imports
- Internal `controller.resize()` calls now use `size.into()`
- All `PtySize::default()` in tests replaced with `size(width(80) + height(24))`
- All `PtySize { rows, cols, ... }` struct literals replaced with `size(width(x) + height(y))`

**`tui/src/core/pty_mux/process_manager.rs`**

- Removed `use portable_pty::PtySize`
- Replaced `PtySize { rows, cols, ... }` struct literals with `size(width(x) + height(y))`
- Merged duplicate `buffer_size` and `pty_size` variables (they became identical after
  the type change)

**`tui/src/core/ansi/.../pty_sigwinch_test.rs`**

- Added `use crate::{Size, height, size, width};`
- Replaced `PtySize { rows, cols, ... }` with `size(width(100) + height(30))`
- Added `.into()` for `controller_mut().resize()` call

**Examples** (`pty_simple_example.rs`, `spawn_pty_read_write.rs`, `pty_rw_echo_example.rs`)

- Removed `use portable_pty::PtySize`
- Replaced `PtySize { rows, cols, ... }` with `size(width(x) + height(y))`

**`tui/src/core/pty/mod.rs`**

- Updated doc example to use `size(width(80) + height(24))` instead of
  `PtySize { rows: 24, cols: 80, ... }`

### Phase 2: Shrink PtyPair's public API

**`tui/src/core/pty/pty_core/pty_types.rs`**

**Made struct fields private:**

```rust
// Before:
pub controller: Controller,
pub maybe_controlled: Option<Controlled>,

// After:
controller: Controller,
maybe_controlled: Option<Controlled>,
```

**Added `new_with_size()` and `new_default()` constructors:**

```rust
pub fn new_with_size(pty_size: Size) -> miette::Result<Self> {
    let pty_system = portable_pty::native_pty_system();
    let raw_pair = pty_system
        .openpty(pty_size.into())
        .map_err(|e| miette::miette!("Failed to open PTY: {e}"))?;
    Ok(Self::from(raw_pair))
}

pub fn new_default() -> miette::Result<Self> {
    Self::new_with_size(size(width(80) + height(24)))
}
```

**Updated `into_controller()`** to silently drop controlled instead of asserting:

```rust
pub fn into_controller(self) -> Controller {
    drop(self.maybe_controlled);
    self.controller
}
```

**Updated `spawn_command_and_close_controlled()`** to inline close logic:

```rust
pub fn spawn_command_and_close_controlled(
    &mut self,
    command: CommandBuilder,
) -> miette::Result<ControlledChild> {
    let controlled = self
        .maybe_controlled
        .as_ref()
        .expect("controlled side already consumed");
    let child = controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("{e:#}"))?;
    drop(self.maybe_controlled.take());
    Ok(child)
}
```

**Deleted `split()`** -- only caller (`read_lines_windows`) replaced by
`into_controller()`.

**Deleted `close_controlled()`** -- inlined into `spawn_command_and_close_controlled()`.

**Deleted `controlled()` / `controlled_mut()`** -- inlined into
`spawn_command_and_close_controlled()`.

**Updated struct-level doc example** to use `PtyPair::new_with_size()` instead of raw
`portable_pty`:

```rust
/// use r3bl_tui::{PtyPair, size, width, height};
/// use portable_pty::CommandBuilder;
///
/// let mut pty_pair = PtyPair::new_with_size(size(width(80) + height(24))).unwrap();
/// let _child = pty_pair.spawn_command_and_close_controlled(CommandBuilder::new("cat")).unwrap();
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
```

**Updated doc references:** All references to `PtyPair::close_controlled` changed to
`PtyPair::spawn_command_and_close_controlled`.

### Phase 3: Update pty_read_only.rs

**`tui/src/core/pty/pty_read_only.rs`**

```rust
// Before:
let (controller, controlled) = create_pty_pair(pty_config.get_pty_size())?;
let controlled_child = spawn_command_in_pty(controlled, command)?;
let reader = controller.try_clone_reader()?;
// ... later ...
drop(controller);

// After:
let mut pty_pair = PtyPair::new_with_size(pty_config.get_pty_size())?;
let controlled_child = pty_pair.spawn_command_and_close_controlled(command)?;
let reader = pty_pair.controller().try_clone_reader()?;
// ... later ...
drop(pty_pair);
```

Removed imports: `Controlled`, `pty_common_io::{create_pty_pair, spawn_command_in_pty}`.
Added import: `PtyPair`.

### Phase 4: Update pty_read_write.rs

**`tui/src/core/pty/pty_read_write.rs`**

```rust
// Before:
let (controller, controlled) = create_pty_pair(pty_size)?;
let mut controlled_child = spawn_command_in_pty(controlled, command)?;
// ... controller moved into async block ...

// After:
let mut pty_pair = PtyPair::new_with_size(pty_size)?;
let mut controlled_child = pty_pair.spawn_command_and_close_controlled(command)?;

// Clone the reader before consuming the PtyPair.
let controller_reader = pty_pair
    .controller()
    .try_clone_reader()
    .map_err(|e| miette!("Failed to clone reader: {}", e))?;

// Consume the PtyPair, returning the owned Controller.
let controller = pty_pair.into_controller();
// ... controller_reader and controller moved into async block ...
```

The key difference from pty_read_only is that the reader clone happens **before**
`into_controller()` consumes the pair, because both the reader and the controller need
to be moved into separate tasks.

Removed imports: `Controlled`, `pty_common_io::{create_pty_pair, spawn_command_in_pty}`.
Added imports: `PtyPair`, `Controller`.

**Unit tests** (6 tests for `create_controller_input_writer_task`):

```rust
// Before:
let (controller, _controlled) = create_pty_pair(size(width(80) + height(24))).unwrap();

// After:
let controller = PtyPair::new_default().unwrap().into_controller();
```

### Phase 5: Update test fixtures

**`tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`**

```rust
// Before (inside macro):
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use $crate::PtyPair;
// ...
let pty_system = NativePtySystem::default();
let raw_pty_pair = pty_system
    .openpty(PtySize { rows: 24, cols: 80, pixel_width: 0, pixel_height: 0 })
    .expect("Failed to create PTY pair");
let mut pty_pair = PtyPair::from(raw_pty_pair);

// After (inside macro):
use $crate::{PtyCommand, PtyPair};
// ...
let mut pty_pair = PtyPair::new_default()
    .expect("Failed to create PTY pair");
```

Also replaced `CommandBuilder` with `PtyCommand` (which is a type alias for
`CommandBuilder`).

**`tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`**

```rust
// Before:
use crate::PtyPair;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
// ...
let pty_system = NativePtySystem::default();
let raw_pty_pair = pty_system
    .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
    .expect("Failed to create PTY pair");
let mut pty_pair = PtyPair::from(raw_pty_pair);

// After:
use crate::{PtyPair, height, size, width};
use portable_pty::CommandBuilder;
// ...
let mut pty_pair = PtyPair::new_with_size(size(width(cols) + height(rows)))
    .expect("Failed to create PTY pair");
```

After linter processing, `CommandBuilder` was replaced with `PtyCommand` (crate-level
type alias) and the `portable_pty` import was removed entirely.

**`tui/src/core/test_fixtures/pty_test_fixtures/read_lines_and_drain.rs`**

```rust
// Before:
let (controller, controlled) = pty_pair.split();
drop(controlled);

// After:
let controller = pty_pair.into_controller();
```

**`tui/src/core/test_fixtures/pty_test_fixtures/single_thread_safe_controlled_child.rs`**

Updated doc references: `PtyPair::close_controlled` changed to
`PtyPair::spawn_command_and_close_controlled`.

### Phase 6: Delete pty_common_io.rs

**Deleted `tui/src/core/pty/pty_common_io.rs`** -- both functions (`create_pty_pair`,
`spawn_command_in_pty`) replaced by `PtyPair::new_with_size()` and
`PtyPair::spawn_command_and_close_controlled()`.

**Removed `pub mod pty_common_io;`** from `tui/src/core/pty/mod.rs`.

**Moved tests** to `tui/src/core/pty/pty_core/pty_types.rs`:
- `test_create_pty_pair` became `test_new_default_creates_pty_pair`
- `test_create_pty_pair_with_custom_size` became `test_new_with_custom_size`
- `test_spawn_command_in_pty` became `test_spawn_command_and_close_controlled`

### Phase 7: Update PtyPair doc comments

**`tui/src/core/pty/pty_core/pty_types.rs`**

Added "Deadlock-safe API design" section to the `PtyPair` struct-level rustdoc, after
the "Controlled side lifecycle" section:

```rust
/// ## Deadlock-safe API design
///
/// The API is designed so that every codepath either closes the controlled [`fd`]
/// automatically or never exposes it. There is no way to obtain a raw [`Controlled`]
/// value, so the deadlock described in [Controlled side lifecycle] is structurally
/// impossible.
///
/// | Method                                   | Purpose                                                           |
/// | :--------------------------------------- | :---------------------------------------------------------------- |
/// | [`new_with_size()`]                      | Create a [`PTY`] pair with explicit dimensions                    |
/// | [`new_default()`]                        | Create a [`PTY`] pair with standard 80x24 size                    |
/// | [`spawn_command_and_close_controlled()`] | Spawn child + close controlled [`fd`]                             |
/// | [`controller()`] / [`controller_mut()`]  | Borrow [`Controller`]                                             |
/// | [`into_controller()`]                    | Take owned [`Controller`] (drops controlled [`fd`] if still open) |
///
/// **[`spawn_command_and_close_controlled()`]** is the only way to spawn a child. It
/// closes the parent's controlled [`fd`] immediately after spawning, so the child's
/// copies are the only ones left.
///
/// **[`into_controller()`]** is the only way to extract an owned [`Controller`]. It
/// drops the controlled side automatically if it is still open, as a safety net.
///
/// **[`controller()`]** and **[`controller_mut()`]** return borrows, so the
/// [`PtyPair`] retains ownership and the controlled side stays managed.
```

Added `# Panics` section to `spawn_command_and_close_controlled()` doc:

```rust
/// # Panics
///
/// Panics if the controlled side has already been consumed by a previous call to
/// this method or [`into_controller()`].
```

## Files changed (summary)

- [x] `tui/src/core/pty/pty_core/pty_types.rs` -- Added `From<Size> for PtySize`,
  `new_with_size(Size)`, `new_default()`, updated `into_controller()` (drop instead of
  assert), deleted `split()`, inlined + removed `close_controlled()`, deleted
  `controlled()`/`controlled_mut()`, made fields private, added "Deadlock-safe API
  design" doc section, added `# Panics` doc, moved tests from pty_common_io
- [x] `tui/src/core/pty/pty_config.rs` -- Replaced `PtySize` with `Size` in
  `PtyConfigOption::Size`, `PtyConfig.pty_size`, `get_pty_size()`, `From` impl
- [x] `tui/src/core/pty/pty_core/pty_input_events.rs` -- `Resize(Size)` instead of
  `Resize(PtySize)`
- [x] `tui/src/core/pty/pty_read_write.rs` -- `spawn_read_write(Size)`, replaced
  `create_pty_pair` + `spawn_command_in_pty` with `PtyPair` methods, clone reader before
  `into_controller()`, updated 6 unit tests
- [x] `tui/src/core/pty/pty_read_only.rs` -- Replaced `create_pty_pair` +
  `spawn_command_in_pty` with `PtyPair` methods
- [x] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs` -- Used
  `PtyPair::new_default()`, removed `NativePtySystem`/`PtySize`/`PtySystem` imports,
  replaced `CommandBuilder` with `PtyCommand`
- [x] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs` -- Used
  `PtyPair::new_with_size()`, removed `NativePtySystem`/`PtySize`/`PtySystem` imports
- [x] `tui/src/core/test_fixtures/pty_test_fixtures/read_lines_and_drain.rs` -- Replaced
  `split()` + `drop(controlled)` with `into_controller()`
- [x] `tui/src/core/test_fixtures/pty_test_fixtures/single_thread_safe_controlled_child.rs` --
  Updated doc references to `spawn_command_and_close_controlled`
- [x] `tui/src/core/pty/pty_common_io.rs` -- Deleted entirely
- [x] `tui/src/core/pty/mod.rs` -- Removed `pub mod pty_common_io`, updated doc examples
- [x] `tui/src/core/pty_mux/process_manager.rs` -- Replaced `use portable_pty::PtySize`
  with `Size`, merged duplicate size variables
- [x] `tui/src/core/ansi/.../pty_sigwinch_test.rs` -- Replaced `PtySize` with `Size`,
  added `.into()` for resize call
- [x] `tui/examples/pty_simple_example.rs` -- Replaced `PtySize` with `Size`
- [x] `tui/examples/spawn_pty_read_write.rs` -- Replaced `PtySize` with `Size`
- [x] `tui/examples/pty_rw_echo_example.rs` -- Replaced `PtySize` with `Size`

## Verification

All checks pass (`check.fish --full`):

1. `check.fish --check` -- all call sites compile
2. `check.fish --build` -- full build passes
3. `check.fish --clippy` -- zero warnings
4. `check.fish --doc` -- docs build (intra-doc links resolve)
5. `check.fish --test` -- unit + integration tests pass
6. Windows cross-compilation check passes

## Risk assessment

**Low risk.** This is a pure refactor -- no behavioral changes. The same kernel operations
happen in the same order. The only difference is which struct owns the values.

**Subtlety**: `pty_read_write.rs` moves `controller` into an `async move` block. After
this refactor, `controller` comes from `pty_pair.into_controller()` instead of
`create_pty_pair()`. The reader is cloned **before** `into_controller()` consumes the
pair. The move semantics are identical.

**Safety improvement**: After this refactor, the controlled-fd deadlock is structurally
impossible. Every codepath through `PtyPair` either closes the controlled fd automatically
or never exposes it. No `split()`, no public `close_controlled()`, no way to leak the fd.
