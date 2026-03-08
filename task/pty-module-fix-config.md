# Plan: Type-safe `PtySessionConfig` with explicit defaults

## Context

`PtySessionConfig` currently uses `bool` fields and an `impl Default` that silently injects
default values whenever a caller passes a single `PtySessionConfigOption` or combines two
options with `+`. This is at odds with the codebase's "make illegal states unrepresentable"
philosophy — a single option like `CaptureOutput` is not a complete config, yet the type system
treats it as one by silently filling in defaults via `From<PtySessionConfigOption>`.

The codebase already has a clean pattern for explicit defaults: `DefaultPtySize` (a zero-sized
marker struct in `pty_size.rs`). We'll apply the same pattern to `PtySessionConfig`.

## Changes

### 1. Define flag enums (`pty_session_builder.rs`)

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFlag { Capture, NoCapture }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectFlag { Detect, NoDetect }
```

No helper methods — callers compare directly against variants (e.g.,
`config.capture_output == CaptureFlag::Capture`).

### 2. Update `PtySessionConfig` struct fields (`pty_session_builder.rs`)

```rust
pub struct PtySessionConfig {
    pub capture_osc: CaptureFlag,
    pub capture_output: CaptureFlag,
    pub detect_cursor_mode: DetectFlag,
    pub pty_size: Size,
}
```

### 3. Replace `impl Default` with `DefaultPtySessionConfig` marker (`pty_session_builder.rs`)

```rust
#[derive(Debug, Clone, Copy)]
pub struct DefaultPtySessionConfig;

impl From<DefaultPtySessionConfig> for PtySessionConfig {
    fn from(_: DefaultPtySessionConfig) -> Self {
        Self {
            capture_osc: CaptureFlag::NoCapture,
            capture_output: CaptureFlag::Capture,
            detect_cursor_mode: DetectFlag::Detect,
            pty_size: DefaultPtySize.into(),
        }
    }
}
```

Delete `impl Default for PtySessionConfig`.

### 4. Update `apply()` match arms (`pty_session_builder.rs`)

Use `CaptureFlag::Capture` / `CaptureFlag::NoCapture` / `DetectFlag::Detect` /
`DetectFlag::NoDetect` instead of `true` / `false`.

### 5. Rework trait impls (`pty_session_builder.rs`)

**Remove** (these all silently inject defaults):
- `From<PtySessionConfigOption> for PtySessionConfig`
- `Add for PtySessionConfigOption` (Option + Option)
- `Add<PtySessionConfig> for PtySessionConfigOption` (reverse-order add)
- `From<Size> for PtySessionConfigOption`
- `From<Size> for PtySessionConfig`

**Add:**
- `Add<PtySessionConfigOption> for DefaultPtySessionConfig` — enables
  `DefaultPtySessionConfig + PtySessionConfigOption::CaptureOsc`

**Keep as-is:**
- `Add<PtySessionConfigOption> for PtySessionConfig` — enables chaining after first `+`
- `AddAssign<PtySessionConfigOption> for PtySessionConfig` — enables `+=`

### 6. Update reader task conditionals (`pty_session_impl_read_only.rs`)

Three sites, lines 131, 138, 147 — compare directly against enum variants:
- `if config.capture_output` → `if config.capture_output == CaptureFlag::Capture`
- `if config.capture_osc` → `if config.capture_osc == CaptureFlag::Capture`
- `if config.detect_cursor_mode` → `if config.detect_cursor_mode == DetectFlag::Detect`

### 7. Update all 13 call sites

| File | Before | After |
|------|--------|-------|
| `read_only_session_test.rs:10` | `CaptureOutput` | `DefaultPtySessionConfig` |
| `read_write_session_test.rs:10` | `CaptureOutput` | `DefaultPtySessionConfig` |
| `error_handling_test.rs:10` | `NoCaptureOutput` | `DefaultPtySessionConfig + NoCaptureOutput` |
| `osc_capture_test.rs:12` | `CaptureOsc + NoCaptureOutput` | `DefaultPtySessionConfig + CaptureOsc + NoCaptureOutput` |
| `resize_test.rs:11` | `CaptureOutput + Size(...)` | `DefaultPtySessionConfig + Size(...)` |
| `spawn_pty_read_only.rs:50` | `NoCaptureOutput` | `DefaultPtySessionConfig + NoCaptureOutput` |
| `spawn_pty_read_only.rs:85` | `CaptureOsc` | `DefaultPtySessionConfig + CaptureOsc` |
| `spawn_pty_read_write.rs:43` | `size(...)` | `DefaultPtySessionConfig + Size(size(...))` |
| `spawn_pty_read_write.rs:203` | `size(...)` | `DefaultPtySessionConfig + Size(size(...))` |
| `pty_simple_example.rs:57` | `terminal_size` | `DefaultPtySessionConfig + Size(terminal_size)` |
| `pty_rw_echo_example.rs:50` | `terminal_size` | `DefaultPtySessionConfig + Size(terminal_size)` |
| `upgrade_check.rs:251` | `CaptureOutput` | `DefaultPtySessionConfig` |
| `upgrade_check.rs:294` | `CaptureOsc` | `DefaultPtySessionConfig + CaptureOsc` |

(All `PtySessionConfigOption::` prefixes omitted for brevity in the table.)

### 8. Rewrite unit tests (`pty_session_builder.rs`)

- `test_default_config` → use `PtySessionConfig::from(DefaultPtySessionConfig)`, assert enum values
- `test_option_combination` → all chains start from `DefaultPtySessionConfig +`
- `test_add_assign_and_chaining` → start from `DefaultPtySessionConfig.into()`
- `test_from_size` → replace with `DefaultPtySessionConfig + Size(sz)` test

## Files to modify

1. `tui/src/core/pty/pty_session/pty_session_builder.rs` — core changes (steps 1-5, 8)
2. `tui/src/core/pty/pty_session/pty_session_impl_read_only.rs` — conditionals (step 6)
3. `tui/src/core/pty/e2e_tests/read_only_session_test.rs` — call site
4. `tui/src/core/pty/e2e_tests/read_write_session_test.rs` — call site
5. `tui/src/core/pty/e2e_tests/error_handling_test.rs` — call site
6. `tui/src/core/pty/e2e_tests/osc_capture_test.rs` — call site
7. `tui/src/core/pty/e2e_tests/resize_test.rs` — call site
8. `tui/examples/spawn_pty_read_only.rs` — call site
9. `tui/examples/spawn_pty_read_write.rs` — call site
10. `tui/examples/pty_simple_example.rs` — call site
11. `tui/examples/pty_rw_echo_example.rs` — call site
12. `cmdr/src/analytics_client/upgrade_check.rs` — call site + import update

## Verification

1. `./check.fish --check` — typecheck passes
2. `cargo test -p r3bl_tui --lib pty_session_builder` — unit tests pass
3. `./check.fish --clippy` — no warnings
