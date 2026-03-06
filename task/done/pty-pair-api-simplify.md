<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
- [Implementation plan](#implementation-plan)
  - [Step 0: Refactor `pty_pair.rs` methods](#step-0-refactor-pty_pairrs-methods)
    - [Step 0.0: Add `open_and_spawn` method](#step-00-add-open_and_spawn-method)
    - [Step 0.1: Add free functions (below `impl PtyPair`)](#step-01-add-free-functions-below-impl-ptypair)
    - [Step 0.2: Remove old methods from `impl PtyPair`](#step-02-remove-old-methods-from-impl-ptypair)
    - [Step 0.3: Inline `From<portable_pty::PtyPair>` impl in `pty_size.rs`](#step-03-inline-fromportable_ptyptypair-impl-in-pty_sizers)
    - [Step 0.4: Fix `pty_size.rs` test assertions](#step-04-fix-pty_sizers-test-assertions)
  - [Step 1: Update rustdoc references](#step-1-update-rustdoc-references)
    - [Step 1.0: Find-and-replace inline references in `pty_pair.rs` doc block](#step-10-find-and-replace-inline-references-in-pty_pairrs-doc-block)
    - [Step 1.1: Update link ref defs at bottom of `pty_pair.rs` doc block](#step-11-update-link-ref-defs-at-bottom-of-pty_pairrs-doc-block)
    - [Step 1.2: Rewrite 3 API contract sections in `pty_pair.rs` doc block](#step-12-rewrite-3-api-contract-sections-in-pty_pairrs-doc-block)
    - [Step 1.3: Update `single_thread_safe_controlled_child.rs` doc references](#step-13-update-single_thread_safe_controlled_childrs-doc-references)
    - [Step 1.4: Update `pty/mod.rs` doc reference](#step-14-update-ptymodrs-doc-reference)
  - [Step 2: Update production wrappers](#step-2-update-production-wrappers)
    - [Step 2.0: Update `pty_read_write.rs` (around line 177)](#step-20-update-pty_read_writers-around-line-177)
    - [Step 2.1: Update `pty_read_only.rs` (around line 124)](#step-21-update-pty_read_onlyrs-around-line-124)
  - [Step 3: Update test fixtures](#step-3-update-test-fixtures)
    - [Step 3.0: Update `spawn_controlled_in_pty.rs` (around line 88)](#step-30-update-spawn_controlled_in_ptyrs-around-line-88)
    - [Step 3.1: Update `generate_pty_test.rs` (around line 275)](#step-31-update-generate_pty_testrs-around-line-275)
  - [Step 4: Update tests](#step-4-update-tests)
    - [Step 4.0: Update `pty_read_write.rs` tests (6 occurrences, lines ~1789-1935)](#step-40-update-pty_read_writers-tests-6-occurrences-lines-1789-1935)
    - [Step 4.1: Update `pty_pair.rs` tests](#step-41-update-pty_pairrs-tests)
  - [Step 5: Verify re-exports and run checks](#step-5-verify-re-exports-and-run-checks)
    - [Step 5.0: Check `mod.rs` re-exports](#step-50-check-modrs-re-exports)
    - [Step 5.1: Check crate-level re-exports](#step-51-check-crate-level-re-exports)
    - [Step 5.2: Run checks](#step-52-run-checks)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

Simplify the `PtyPair` API by introducing `open_and_spawn()` as the primary method that fuses PTY
creation and command spawning into a single deadlock-safe call. Move the old constructors and spawn
logic to free functions for internal/test use.

**Key decisions:**

- `open_and_spawn(size: impl Into<Size>, command: PtyCommand)` is the new primary method on `PtyPair`.
- `controller()`, `controller_mut()`, `into_controller()` remain as methods (public API).
- Old methods (`new_with_size`, `new_with_default_size`, `new`, `spawn_command_and_close_controlled`)
  are removed from `impl PtyPair` and their functionality preserved as free functions and the
  existing `From` impl.
- `DefaultPtySize` (`pub`) provides default 80x24 dimensions for test code.
- No deprecation period - clean break.

**Resulting public API:**

```rust
// Methods on PtyPair (the public interface)
impl PtyPair {
    pub fn open_and_spawn(size: impl Into<Size>, command: PtyCommand) -> Result<(Self, ControlledChild)>;
    pub fn controller(&self) -> &Controller;
    pub fn controller_mut(&mut self) -> &mut Controller;
    pub fn into_controller(self) -> Controller;
}

// Free functions in pty_pair module (internal/test use)
pub fn open_raw_pair(size: impl Into<portable_pty::PtySize>) -> Result<PtyPair>;
pub fn spawn_command_and_close_controlled(pair: &mut PtyPair, cmd: PtyCommand) -> Result<ControlledChild>;

// From impl in pty_size module (replaces PtyPair::new())
impl From<portable_pty::PtyPair> for PtyPair { ... }
```

**Usage sites that need updating (15+ locations):**

| Category                         | Files                                                             | Count |
| :------------------------------- | :---------------------------------------------------------------- | :---- |
| Production (use open_and_spawn)  | `pty_read_write.rs`, `pty_read_only.rs`                           | 2     |
| Test fixtures (use open_and_spawn) | `spawn_controlled_in_pty.rs`, `generate_pty_test.rs`            | 2     |
| Tests needing raw controller     | `pty_read_write.rs` (6 test functions)                            | 6     |
| Tests in pty_pair.rs             | `pty_pair.rs` (3 tests)                                           | 3     |
| Rustdoc references               | `pty_pair.rs`, `single_thread_safe_controlled_child.rs`, `mod.rs` | 3     |
| Resize (no change needed)        | `pty_sigwinch_test.rs`, `pty_read_write.rs`                       | 2     |

# Implementation plan

## Step 0: Refactor `pty_pair.rs` methods

### Step 0.0: Add `open_and_spawn` method

Add the new primary method to `impl PtyPair`:

```rust
/// Opens a [`PTY`] pair with the given size and immediately spawns the command.
///
/// This is the safest way to initialize a [`PTY`] as it automatically drops the
/// parent's copy of the controlled fd after spawning.
pub fn open_and_spawn(
    arg_size: impl Into<Size>,
    command: PtyCommand,
) -> miette::Result<(Self, ControlledChild)> {
    let size = arg_size.into();
    let pty_system = portable_pty::native_pty_system();
    let raw_pair = pty_system
        .openpty(size.into())
        .map_err(|e| miette::miette!("Failed to open PTY: {e}"))?;

    let mut controlled = raw_pair.slave;
    let child = controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("{e:#}"))?;

    // Immediately drop to avoid deadlock.
    drop(controlled);

    Ok((
        Self {
            controller: raw_pair.master,
            maybe_controlled: None,
        },
        child,
    ))
}
```

### Step 0.1: Add free functions (below `impl PtyPair`)

Move old constructor and spawn logic to free functions:

```rust
/// Opens a raw [`PTY`] pair without spawning a child process.
///
/// This is an internal implementation step used by [`PtyPair::open_and_spawn()`].
/// It is exposed as a free function for use-cases that need to separate [`PTY`]
/// creation from spawning (e.g., tests that only need a raw [`Controller`] for
/// mocking). Pass [`DefaultPtySize`] for standard test dimensions.
///
/// The returned [`PtyPair`] still holds the controlled [`fd`]. You must either:
/// - Call [`spawn_command_and_close_controlled()`] to spawn and close the controlled [`fd`], or
/// - Call [`PtyPair::into_controller()`] which drops the controlled side as a safety net.
///
/// See [`PtyPair`]'s [Controlled side lifecycle] section for why the controlled
/// [`fd`] must be closed before reading from the controller.
///
/// # Returns
///
/// A [`PtyPair`] with both controller and controlled sides open.
///
/// # Errors
///
/// Returns an error if the [`PTY`] system fails to open a pair.
///
/// [Controlled side lifecycle]: PtyPair#controlled-side-lifecycle
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn open_raw_pair(arg_pty_size: impl Into<portable_pty::PtySize>) -> miette::Result<PtyPair> {
    let pty_size = arg_pty_size.into();
    let raw_pair = portable_pty::native_pty_system()
        .openpty(pty_size)
        .map_err(|e| miette::miette!("Failed to open PTY: {e}"))?;
    Ok(PtyPair::from(raw_pair))
}

/// Spawns a command on the controlled side and immediately closes the controlled
/// [`fd`].
///
/// This is an internal implementation step used by [`PtyPair::open_and_spawn()`].
/// It is exposed as a free function for use-cases that need to separate [`PTY`]
/// creation from spawning (e.g., tests that configure the pair before spawning).
///
/// See [`PtyPair`]'s [Controlled side lifecycle] section for why the controlled
/// [`fd`] must be closed immediately after spawning.
///
/// # Returns
///
/// The spawned [`ControlledChild`] process handle. The controlled [`fd`] is closed
/// before returning.
///
/// # Errors
///
/// Returns an error if the controlled side has already been consumed (e.g., by a
/// previous call to this function or by [`PtyPair::into_controller()`]), or if
/// the command fails to spawn.
///
/// [Controlled side lifecycle]: PtyPair#controlled-side-lifecycle
/// [`fd`]: https://man7.org/linux/man-pages/man2/open.2.html
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
pub fn spawn_command_and_close_controlled(
    pair: &mut PtyPair,
    command: PtyCommand,
) -> miette::Result<ControlledChild> {
    let controlled = pair
        .maybe_controlled
        .as_ref()
        .ok_or_else(|| miette::miette!(
            "Controlled side already consumed - was this pair already spawned or created via open_and_spawn()?"
        ))?;
    let child = controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("{e:#}"))?;
    drop(pair.maybe_controlled.take());
    Ok(child)
}
```

### Step 0.2: Remove old methods from `impl PtyPair`

Delete these four methods:

- `new(inner: portable_pty::PtyPair)` - replaced by `From` impl in `pty_size.rs`
- `new_with_size(pty_size: Size)` - replaced by `open_raw_pair` free function
- `new_with_default_size()` - replaced by `open_raw_pair(DefaultPtySize)`
- `spawn_command_and_close_controlled(&mut self, command)` - replaced by
  `spawn_command_and_close_controlled` free function

Keep `controller()`, `controller_mut()`, and `into_controller()` as methods.

### Step 0.3: Inline `From<portable_pty::PtyPair>` impl in `pty_size.rs`

The current `From` impl calls `Self::new(it)` which will be removed. Replace with struct literal:

```rust
impl From<portable_pty::PtyPair> for PtyPair {
    fn from(it: portable_pty::PtyPair) -> Self {
        Self {
            controller: it.master,
            maybe_controlled: Some(it.slave),
        }
    }
}
```

### Step 0.4: Fix `pty_size.rs` test assertions

The test `test_default_pty_size_conversion` asserts `rows: 80, cols: 24` but the actual values
produced by `DefaultPtySize` are `rows: 24, cols: 80` (standard 80-column by 24-row terminal). Fix:

```rust
assert_eq!(default_size.rows, 24);
assert_eq!(default_size.cols, 80);
```

## Step 1: Update rustdoc references

The bulk of the ~400-line doc block on `PtyPair` is educational prose about PTY internals, fd
lifecycle, and kernel behavior. This content is still accurate - only the function names and a few
API-facing sections need updating. The approach is:

1. **Find-and-replace display text** in inline references (no stale names).
2. **Update link ref defs** at the bottom to point to the new targets.
3. **Rewrite 3 API contract spots** that describe the public interface.

### Step 1.0: Find-and-replace inline references in `pty_pair.rs` doc block

Replace display text AND link targets together - no stale names left behind:

| Old inline reference                                  | New inline reference                  |
| :---------------------------------------------------- | :------------------------------------ |
| `` [`PtyPair::new_with_size()`] ``                    | `` [`open_raw_pair()`] ``             |
| `` [`new_with_size()`] ``                             | `` [`open_raw_pair()`] ``             |
| `` [`PtyPair::new_with_default_size()`] ``            | `` [`open_raw_pair()`] ``             |
| `` [`new_with_default_size()`] ``                     | `` [`open_raw_pair()`] ``             |
| `` [`PtyPair::spawn_command_and_close_controlled()`] `` | `` [`PtyPair::open_and_spawn()`] `` |
| `` [`spawn_command_and_close_controlled()`] ``        | `` [`spawn_command_and_close_controlled()`] ``|
| `` [`PtyPair::new()`] ``                              | remove (From impl, no display link)   |

### Step 1.1: Update link ref defs at bottom of `pty_pair.rs` doc block

Remove old definitions and add new ones:

```rust
// Remove:
/// [`new_with_default_size()`]: PtyPair::new_with_default_size
/// [`new_with_size()`]: PtyPair::new_with_size
/// [`PtyPair::spawn_command_and_close_controlled()`]:
///     PtyPair::spawn_command_and_close_controlled
/// [`spawn_command_and_close_controlled()`]: PtyPair::spawn_command_and_close_controlled

// Add:
/// [`open_raw_pair()`]: open_raw_pair
/// [`spawn_command_and_close_controlled()`]: spawn_command_and_close_controlled
```

`[`PtyPair::open_and_spawn()`]` resolves automatically (method on the struct being documented).

### Step 1.2: Rewrite 3 API contract sections in `pty_pair.rs` doc block

These sections describe the public API shape and need actual prose changes:

1. **"What this struct does"** (~line 129): Update to reference `open_and_spawn()` instead of
   `spawn_command_and_close_controlled()`.
2. **"Deadlock-safe API design"** (~line 320): Update the two bullet points - first bullet becomes
   `open_and_spawn()`, second bullet (`into_controller()`) stays as-is.
3. **Code example** (~line 336): Rewrite to use `open_and_spawn`:

````rust
/// ```no_run
/// use r3bl_tui::{PtyCommand, PtyPair, size, width, height};
///
/// let (pty_pair, _child) =
///     PtyPair::open_and_spawn(size(width(80) + height(24)), PtyCommand::new("cat")).unwrap();
///
/// // Reads from the controller will return EOF once the child exits.
/// let reader = pty_pair.controller().try_clone_reader().unwrap();
/// ```
````

### Step 1.3: Update `single_thread_safe_controlled_child.rs` doc references

Lines 74, 103, 115, 125 reference `PtyPair::spawn_command_and_close_controlled`. Replace with
`PtyPair::open_and_spawn` or `spawn_command_and_close_controlled` as appropriate.

### Step 1.4: Update `pty/mod.rs` doc reference

Line 169 references `PtyPair::spawn_command_and_close_controlled`. Update.

## Step 2: Update production wrappers

### Step 2.0: Update `pty_read_write.rs` (around line 177)

Replace two-step with single call:

```rust
// Old:
// let mut pty_pair = PtyPair::new_with_size(pty_size)?;
// let mut controlled_child: ControlledChild =
//     pty_pair.spawn_command_and_close_controlled(command)?;

// New:
let (pty_pair, mut controlled_child) = PtyPair::open_and_spawn(pty_size, command)?;
```

Note: `pty_pair` is no longer `mut` since `open_and_spawn` returns with controlled already closed.

### Step 2.1: Update `pty_read_only.rs` (around line 124)

```rust
// Old:
// let mut pty_pair = PtyPair::new_with_size(pty_config.get_pty_size())?;
// let controlled_child: ControlledChild =
//     pty_pair.spawn_command_and_close_controlled(command)?;

// New:
let (pty_pair, controlled_child) = PtyPair::open_and_spawn(pty_config.get_pty_size(), command)?;
```

## Step 3: Update test fixtures

### Step 3.0: Update `spawn_controlled_in_pty.rs` (around line 88)

Replace two-step init with `open_and_spawn`:

```rust
// Old:
// let mut pty_pair = PtyPair::new_with_size(size(width(cols) + height(rows)))?;
// let _child = pty_pair.spawn_command_and_close_controlled(cmd)?;

// New:
let (pty_pair, _child) = PtyPair::open_and_spawn(
    size(width(cols) + height(rows)),
    cmd,
).expect("Failed to open PTY and spawn controlled process");
```

### Step 3.1: Update `generate_pty_test.rs` (around line 275)

Replace two-step init. `DefaultPtySize` can be passed directly (no `.into()` needed):

```rust
// Old:
// let mut pty_pair = PtyPair::new_with_default_size()?;
// let child = pty_pair.spawn_command_and_close_controlled(cmd)?;

// New:
let (pty_pair, child) = PtyPair::open_and_spawn(
    DefaultPtySize,
    cmd,
).expect("Failed to open PTY and spawn controlled process");
```

## Step 4: Update tests

### Step 4.0: Update `pty_read_write.rs` tests (6 occurrences, lines ~1789-1935)

All 6 test functions currently do:

```rust
let controller = PtyPair::new_with_default_size().unwrap().into_controller();
```

Replace with:

```rust
use crate::core::pty::pty_core::pty_pair::open_raw_pair;
use crate::core::pty::pty_core::pty_size::DefaultPtySize;

let controller = open_raw_pair(DefaultPtySize).unwrap().into_controller();
```

### Step 4.1: Update `pty_pair.rs` tests

Rewrite the three existing tests to exercise the new API:

1. `test_new_with_default_size_creates_pty_pair` -> `test_open_raw_pair_default_size` - uses
   `open_raw_pair(DefaultPtySize)`
2. `test_new_with_custom_size` -> `test_open_raw_pair_custom_size` - uses
   `open_raw_pair(size(width(100) + height(30)))`
3. `test_spawn_command_and_close_controlled` -> `test_open_and_spawn_success` - uses
   `PtyPair::open_and_spawn(DefaultPtySize, command)` and asserts
   `pty.maybe_controlled.is_none()`

## Step 5: Verify re-exports and run checks

### Step 5.0: Check `mod.rs` re-exports

`pty_core/mod.rs` has `pub use pty_pair::*;` - verify that the new free functions
(`open_raw_pair`, `spawn_command_and_close_controlled`) are exported and accessible from `crate::`.

### Step 5.1: Check crate-level re-exports

Verify `open_raw_pair` and `spawn_command_and_close_controlled` are accessible via the paths tests use.
If the crate root re-exports `PtyPair` but not the free functions, add the needed re-exports or
adjust test imports.

### Step 5.2: Run checks

Run `./check.fish --full` to verify compilation, clippy, docs, and tests all pass.
