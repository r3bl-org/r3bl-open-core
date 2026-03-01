<!-- cspell:words unlockpt errno -->

# Auto-close controlled side after spawn — eliminate deadlock footgun

## Problem

Every PTY spawn site must remember to call `close_controlled()` (or `drop(controlled)`)
immediately after spawning. Forgetting causes permanent deadlocks because the parent's
controlled fd prevents EOF delivery to the controller reader. This is a
"remember-to-call" pattern — the type system does not enforce it.

Current two-step dance (test code):
```rust
let child = pty_pair.controlled().spawn_command(cmd).unwrap();
pty_pair.close_controlled(); // easy to forget
```

Current two-step dance (production code):
```rust
let child = spawn_command_in_pty(&controlled, command)?;
// ... 25 lines later ...
drop(controlled); // easy to forget, far from spawn site
```

## Goal

Make it impossible to hold the parent's controlled fd after spawning. Two strategies:

1. **`PtyPair::spawn_command()`** — combines spawn + close in one method (for test code
   that uses the `PtyPair` wrapper)
2. **`spawn_command_in_pty()` takes ownership** — consumes the `Controlled` value so it
   drops automatically (for production code that uses split tuples)

## Audit of all call sites

### Test code using `PtyPair` wrapper

| Call site | Current pattern | Close controlled? |
| :--- | :--- | :--- |
| `generate_pty_test!` macro (generate_pty_test.rs:300-318) | `pty_pair.controlled().spawn_command()` then `pty_pair.close_controlled()` | Yes |
| `spawn_controlled_in_pty()` (spawn_controlled_in_pty.rs:107) | `pty_pair.controlled().spawn_command()` then returns PtyPair | **No** (callers rely on marker-based reads, not EOF) |
| `read_lines_windows()` (read_lines_and_drain.rs:130) | `pty_pair.split()` then `drop(controlled)` | Yes (via split + drop) |

### Production code using split `(Controller, Controlled)` tuples

| Call site | Current pattern | Close controlled? |
| :--- | :--- | :--- |
| `pty_read_only.rs:124-155` | `spawn_command_in_pty(&controlled, ...)` then `drop(controlled)` 25 lines later | Yes (late) |
| `pty_read_write.rs:183-236` | `spawn_command_in_pty(&controlled, ...)` then `drop(controlled)` 50 lines later | Yes (late) |

### Backend compat tests (callers of `spawn_controlled_in_pty`)

| Call site | How it uses the returned PtyPair |
| :--- | :--- |
| `backend_compat_output_test.rs:186` | `controller::run(spawn_controlled_in_pty(...))` — reads until marker, never closes controlled |
| `backend_compat_output_test.rs:197` | Same |
| `backend_compat_input_test.rs:120` | `controller::run_and_collect(spawn_controlled_in_pty(...))` — reads until marker, never closes controlled |
| `backend_compat_input_test.rs:131` | Same |

The backend compat tests work without closing controlled because they use **marker-based
reads** (read until a specific string, then stop). They never wait for EOF. But this is
fragile — if a test were changed to read until EOF, it would deadlock silently.

## Changes

### Phase 1: Add `PtyPair::spawn_command()` method

**File**: `tui/src/core/pty/pty_core/pty_types.rs`

Add a new method that combines spawn + close:

```rust
/// Spawns a command on the controlled side and immediately closes it.
///
/// This prevents deadlocks by ensuring the parent process never holds the
/// controlled fd after spawning. The child process retains its own copies
/// of the controlled fds (stdin/stdout/stderr), which close when the child
/// exits — delivering EIO/EOF to the controller reader.
pub fn spawn_command(
    &mut self,
    command: CommandBuilder,
) -> Result<ControlledChild, portable_pty::Error> {
    let child = self.controlled().spawn_command(command)?;
    self.close_controlled();
    Ok(child)
}
```

### Phase 2: Update `generate_pty_test!` macro

**File**: `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs`

Replace the two-step dance (lines 300-318):

```rust
// Before:
let child = pty_pair.controlled().spawn_command(cmd).expect("...");
pty_pair.close_controlled();

// After:
let child = pty_pair.spawn_command(cmd).expect("...");
```

Remove the 13-line comment block explaining why `close_controlled()` is needed — the
method's doc comment now carries that responsibility.

### Phase 3: Update `spawn_controlled_in_pty()`

**File**: `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs`

This function currently does NOT close the controlled side. Fix it:

```rust
// Before:
let _child = pty_pair.controlled().spawn_command(cmd).expect("...");

// After:
let _child = pty_pair.spawn_command(cmd).expect("...");
```

Now `spawn_controlled_in_pty()` returns a PtyPair with controlled already closed. The
backend compat tests (`controller::run`, `controller::run_and_collect`) still work because
they only use the controller side.

### Phase 4: Make `spawn_command_in_pty()` consume `Controlled`

**File**: `tui/src/core/pty/pty_common_io.rs`

Change the signature to take ownership:

```rust
// Before:
pub fn spawn_command_in_pty(
    controlled: &Controlled,
    command: PtyCommand,
) -> miette::Result<ControlledChild> {

// After:
pub fn spawn_command_in_pty(
    controlled: Controlled,
    command: PtyCommand,
) -> miette::Result<ControlledChild> {
    let child = controlled
        .spawn_command(command)
        .map_err(|e| miette::miette!("Failed to spawn command: {}", e))?;
    // controlled drops here — parent's fd is released immediately
    Ok(child)
}
```

### Phase 5: Update production callers

**File**: `tui/src/core/pty/pty_read_only.rs` (lines 124-155)

```rust
// Before:
let (controller, controlled) = create_pty_pair(pty_config.get_pty_size())?;
let controlled_child = spawn_command_in_pty(&controlled, command)?;
// ... 25 lines later ...
drop(controlled); // Close the controlled half. CRITICAL for EOF...

// After:
let (controller, controlled) = create_pty_pair(pty_config.get_pty_size())?;
let controlled_child = spawn_command_in_pty(controlled, command)?;
// No drop needed — controlled was consumed by spawn_command_in_pty
```

**File**: `tui/src/core/pty/pty_read_write.rs` (lines 183-236)

Same pattern — remove `&` from `spawn_command_in_pty` call, remove `drop(controlled)` from
the async block. The `controlled` binding must NOT be moved into the async block anymore
(it is consumed before `tokio::spawn`).

### Phase 6: Restrict `controlled()` / `controlled_mut()` visibility

After phases 1-5, no external code needs `controlled()` or `controlled_mut()`. The only
remaining user is `split()` (Windows ConPTY handshake in `read_lines_and_drain.rs`), which
accesses `maybe_controlled` directly.

Consider changing `controlled()` and `controlled_mut()` from `pub` to `pub(crate)` or
removing them entirely. If removed, add a comment on `maybe_controlled` explaining that
`spawn_command()` and `split()` are the only consumers.

**Decision point**: This is optional. If other crate consumers might need direct controlled
access, keep them public. If this is purely internal, restrict visibility.

### Phase 7: Update `PtyPair` struct doc comment

**File**: `tui/src/core/pty/pty_core/pty_types.rs`

The struct doc comment currently explains the two-step pattern. Update it to describe the
new single-step API:

- Update the "Controlled side lifecycle" section to reference `spawn_command()` instead of
  the manual `close_controlled()` dance
- Update the code example (lines 93-110) to use `pty_pair.spawn_command(cmd)`
- Keep the kernel-level explanation (EIO/EOF delivery) — that's still valuable context

### Phase 8: Update `close_controlled()` doc comment

`close_controlled()` remains public (it's still called internally by `spawn_command()` and
may be useful for edge cases). Update its doc comment to note that `spawn_command()` is the
preferred API:

```rust
/// Closes the controlled side of the PTY in the parent process.
///
/// Prefer [`spawn_command()`] which calls this automatically. Use this directly only
/// when you need to close the controlled side without spawning (e.g., after
/// [`split()`]).
```

## Files to change (summary)

- [ ] `tui/src/core/pty/pty_core/pty_types.rs` — Add `PtyPair::spawn_command()`, update
  struct docs, update `close_controlled()` docs
- [ ] `tui/src/core/test_fixtures/pty_test_fixtures/generate_pty_test.rs` — Use
  `pty_pair.spawn_command(cmd)`, remove manual close + comment block
- [ ] `tui/src/core/test_fixtures/pty_test_fixtures/spawn_controlled_in_pty.rs` — Use
  `pty_pair.spawn_command(cmd)` (fixes missing close)
- [ ] `tui/src/core/pty/pty_common_io.rs` — Change `spawn_command_in_pty` to consume
  `Controlled` by value
- [ ] `tui/src/core/pty/pty_read_only.rs` — Remove `&` and `drop(controlled)`
- [ ] `tui/src/core/pty/pty_read_write.rs` — Remove `&`, remove `drop(controlled)` from
  async block, ensure `controlled` is not moved into the closure
- [ ] `tui/src/core/pty/pty_core/pty_types.rs` — Optionally restrict
  `controlled()`/`controlled_mut()` visibility

## Verification

1. `cargo check` — all call sites compile with new signatures
2. `cargo test --lib` — unit tests pass
3. `cargo clippy --all-targets` — no new warnings
4. PTY integration tests (`generate_pty_test!` tests) still pass
5. Backend compat tests still pass (they now get a PtyPair with controlled already closed,
   but they only use the controller side)
6. Windows cross-check:
   `cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata`
   (the `split()` path in `read_lines_and_drain.rs` is Windows-only)

## Risk assessment

**Low risk**: The controlled fd is only needed for `spawn_command()` on the
`portable_pty::SlavePty` trait. Once spawn completes, the child has its own fd copies and
the parent's copy serves no purpose. Dropping it earlier (immediately after spawn vs. 25-50
lines later) is strictly better — the kernel can start EOF delivery sooner.

**Edge case**: `pty_read_write.rs` moves `controlled` into an async block. After this
change, `controlled` is consumed before the async block, so the move is removed. Verify
that no code in the async block references `controlled`.
