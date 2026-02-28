# Plan: Add Self-Upgrade Support to `cargo-rustdoc-fmt`

## Context

The `build-infra` crate (published as `r3bl-build-infra` on crates.io) provides the
`cargo-rustdoc-fmt` binary. We make frequent changes to this tool and need a way to notify users
when updates are available - the same "r3bl experience" that `r3bl-cmdr` binaries (giti, edi, rc)
already provide.

The upgrade machinery currently lives entirely in `r3bl-cmdr`
(`cmdr/src/analytics_client/upgrade_check.rs` + `ui_str.rs`), but is mostly crate-agnostic. We
extract the generic parts into `r3bl_tui` so both cmdr and build-infra can share them.

**Behavior**:
- Normal run: background version check, notification printed at end if update available
- `--upgrade` flag: skip formatting, run full TUI upgrade (spinner + progress), no confirmation
  prompt

## Implementation Plan

### Step 0: Create shared upgrade module in r3bl_tui

New directory: `tui/src/core/script/upgrade/`

```
upgrade/
  mod.rs              -- barrel exports
  version_check.rs    -- background version check against crates.io
  run_upgrade.rs      -- rustup + cargo install with spinner/PTY
  ui_strings.rs       -- parameterized install progress/success/failure messages
```

#### Step 0.0: Create `version_check.rs`

Extract and parameterize the version-checking logic from
`cmdr/src/analytics_client/upgrade_check.rs` (lines 65-149).

**Public API:**

```rust
use std::sync::atomic::AtomicU8;
use crate::AtomicU8Ext; // from tui/src/core/common/common_atomic.rs

/// Holds the result of a background version check.
/// Each binary crate declares its own static instance.
/// Uses AtomicU8 with AtomicU8Ext (0 = false, 1 = true) for consistency
/// with the codebase's atomic patterns.
pub struct UpgradeCheckResult {
    upgrade_required: AtomicU8,
}

impl UpgradeCheckResult {
    pub const fn new() -> Self;
    pub fn is_upgrade_required(&self) -> bool;       // reads via AtomicU8Ext::get(), == 1
    pub fn set_upgrade_required(&self, value: bool);  // writes via AtomicU8Ext::set()
}

/// Spawns a background tokio task to check crates.io for a newer version.
/// Returns immediately without blocking.
///
/// - `crate_name`: e.g., "r3bl-cmdr" or "r3bl-build-infra"
/// - `current_version`: from `env!("CARGO_PKG_VERSION")` in the calling crate
/// - `result`: caller-owned static where the result is stored
pub fn start_background_version_check(
    crate_name: &'static str,
    current_version: &'static str,
    result: &'static UpgradeCheckResult,
);

/// Gets the filename of the currently running executable (at runtime).
pub fn get_bin_name_from_current_exe() -> InlineString;
```

Key design: `env!("CARGO_PKG_VERSION")` and `env!("CARGO_PKG_NAME")` are compile-time macros that
resolve to the compiling crate's values. They **must** stay in the binary crates and be passed as
`&'static str` parameters to the library.

#### Step 0.1: Create `run_upgrade.rs`

Extract the upgrade execution logic from `cmdr/src/analytics_client/upgrade_check.rs` (lines
208-423). These functions are already crate-agnostic - they just need `crate_name` as a parameter
instead of calling `get_self_crate_name()`.

**Public API:**

```rust
/// Runs the full upgrade: rustup toolchain update + cargo install.
/// Shows spinner with progress. Handles Ctrl+C cancellation.
pub async fn run_upgrade_with_spinner(crate_name: &str);
```

**Private functions that move here (unchanged except parameterization):**
- `extract_rustup_progress(output: &str) -> String`
- `run_rustup_update(spinner: Option<&Spinner>) -> Result<ExitStatus, Error>`
- `run_cargo_install_with_progress(crate_name, spinner) -> Result<ExitStatus, Error>`
- `handle_osc_event(event, crate_name, spinner)`
- `report_upgrade_install_result(crate_name, result)`

All imports are crate-internal (`crate::` paths to PTY, Spinner, OscEvent, etc.) since this code
now lives inside r3bl_tui.

#### Step 0.2: Create `ui_strings.rs`

Extract **install-related** UI strings from `cmdr/src/analytics_client/ui_str.rs` (`upgrade_install`
module, lines 9-125). Parameterize with `crate_name: &str`.

Also include the generic "upgrade available" notification (currently in `upgrade_check_msgs`,
line 147), parameterized with `bin_name` and `crate_name`.

**Public API:**

```rust
pub fn install_success_msg(crate_name: &str) -> InlineString;
pub fn install_not_success_msg(crate_name: &str, status: ExitStatus) -> InlineString;
pub fn install_failed_to_run_command_msg(crate_name: &str, err: Error) -> InlineString;
pub fn stop_msg() -> InlineString;
pub fn upgrade_available_notification(bin_name: &str, crate_name: &str) -> InlineString;
```

**Formatting helpers**: The `cmdr/src/common/fmt.rs` helpers (`normal()`, `emphasis()`, `error()`,
`dim()`, `period()`, `colon()`) are thin wrappers around `r3bl_tui` color functions
(`fg_silver_metallic`, `fg_lizard_green`, etc.). Include them as private helpers in `ui_strings.rs`.

#### Step 0.3: Create `mod.rs` and wire into `script/mod.rs`

`upgrade/mod.rs` - barrel exports:
```rust
mod run_upgrade;
mod ui_strings;
mod version_check;

pub use run_upgrade::*;
pub use ui_strings::*;
pub use version_check::*;
```

`tui/src/core/script/mod.rs` - add:
```rust
mod upgrade;
// ...
pub use upgrade::*;
```

This makes everything available at `r3bl_tui::*` through the existing barrel export chain.

### Step 1: Refactor r3bl-cmdr to use shared module

#### Step 1.0: Refactor `upgrade_check.rs`

File: `cmdr/src/analytics_client/upgrade_check.rs`

**Delete** (moved to r3bl_tui - no deprecation, direct removal):
- `UPGRADE_REQUIRED: AtomicBool` static (line 65)
- `extract_rustup_progress` (lines 218-238)
- `run_rustup_update` (lines 248-285)
- `run_cargo_install_with_progress` (lines 287-322)
- `handle_osc_event` (lines 327-351)
- `install_upgrade_command_with_spinner_and_ctrl_c` (lines 353-401)
- `report_upgrade_install_result` (lines 404-423)

**Rewrite** to use shared code:
- Add `static UPGRADE_CHECK: UpgradeCheckResult = UpgradeCheckResult::new();`
- `start_task_to_check_if_upgrade_is_needed()` becomes thin wrapper calling
  `r3bl_tui::start_background_version_check(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"), &UPGRADE_CHECK)`
- `is_upgrade_required()` reads from `UPGRADE_CHECK.is_upgrade_required()`
- `show_exit_message()` calls `r3bl_tui::run_upgrade_with_spinner(get_self_crate_name()).await`
  instead of the local orchestrator
- `get_self_bin_name()` delegates to `r3bl_tui::get_bin_name_from_current_exe()`

**Keep** (cmdr-specific):
- `ExitContext` enum + `show_exit_message()` (interactive yes/no confirmation + goodbye)
- `get_self_bin_emoji()` (cmdr-specific emoji mapping)
- `get_self_version()`, `get_self_crate_name()` (compile-time `env!()` wrappers)

#### Step 1.1: Refactor `ui_str.rs`

File: `cmdr/src/analytics_client/ui_str.rs`

**Delete**: `upgrade_install` module (lines 9-125) - moved to r3bl_tui. No deprecation shim.

**Update**: `upgrade_check_msgs::upgrade_is_required_msg()` to call the shared
`r3bl_tui::upgrade_available_notification(bin_name, crate_name)`.

**Keep**: `goodbye_greetings` module (cmdr-specific lolcat farewell), `yes_msg_raw()`,
`no_msg_raw()`, `ask_user_msg_raw()` (cmdr-specific interactive prompt labels).

### Step 2: Integrate self-upgrade into build-infra

#### Step 2.0: Add `--upgrade` flag to CLIArg

File: `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`

```rust
/// Check for and install the latest version from crates.io.
/// Skips all formatting and runs the upgrade process directly.
#[arg(long)]
pub upgrade: bool,
```

#### Step 2.1: Integrate into binary entry point

File: `build-infra/src/bin/cargo-rustdoc-fmt.rs`

**At module level** - declare the static:
```rust
static UPGRADE_CHECK: r3bl_tui::UpgradeCheckResult = r3bl_tui::UpgradeCheckResult::new();
```

**Early in `run()`** after parsing CLIArg - handle `--upgrade` early exit:
```rust
if cli_arg.upgrade {
    r3bl_tui::run_upgrade_with_spinner(env!("CARGO_PKG_NAME")).await;
    return Ok(());
}
```

**After parsing args, before main work** - spawn background check:
```rust
r3bl_tui::start_background_version_check(
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_VERSION"),
    &UPGRADE_CHECK,
);
```

**At end of `run()`** after the summary line - print notification if available:
```rust
if UPGRADE_CHECK.is_upgrade_required() {
    println!(
        "{}",
        r3bl_tui::upgrade_available_notification(
            &r3bl_tui::get_bin_name_from_current_exe(),
            env!("CARGO_PKG_NAME"),
        )
    );
}
```

#### Step 2.2: Update `Cargo.toml`

File: `build-infra/Cargo.toml`

Bump r3bl_tui version to match the new release that includes the upgrade module. Currently
`version = "0.7.7"` - update to whatever the new version is after Step 0.

### Step 3: Version bumps

All three crates get a version bump as part of this change:

| Crate | Current | New |
|:------|:--------|:----|
| `r3bl_tui` | 0.7.8 | 0.7.9 |
| `r3bl-cmdr` | 0.0.26 | 0.0.27 |
| `r3bl-build-infra` | 0.0.5 | 0.0.6 |

**Files to update:**
- `tui/Cargo.toml` - bump `version` to `"0.7.9"`
- `cmdr/Cargo.toml` - bump `version` to `"0.0.27"`, update `r3bl_tui` dependency to `"0.7.9"`
- `build-infra/Cargo.toml` - bump `version` to `"0.0.6"`, update `r3bl_tui` dependency to `"0.7.9"`

### Step 4: Audit - verify migrated code is fully deleted

After all code changes are complete, perform a systematic audit to ensure no orphaned code remains:

#### Step 4.0: Audit `cmdr/src/analytics_client/upgrade_check.rs`

Verify these items are **completely removed** (not commented out, not behind `#[deprecated]`, not
behind a feature flag - just gone):
- `UPGRADE_REQUIRED: AtomicBool` static
- `extract_rustup_progress()` function
- `run_rustup_update()` function
- `run_cargo_install_with_progress()` function
- `handle_osc_event()` function
- `install_upgrade_command_with_spinner_and_ctrl_c()` function
- `report_upgrade_install_result()` function

Verify no dead imports remain (e.g., `PtyCommandBuilder`, `PtyConfigOption`,
`PtyReadOnlyOutputEvent`, `pty_to_std_exit_status`, `OscEvent`, `SpinnerStyle`, `OutputDevice`,
`Spinner` - these were only needed by the extracted functions).

#### Step 4.1: Audit `cmdr/src/analytics_client/ui_str.rs`

Verify the entire `upgrade_install` module (lines 9-125) is **deleted** - not the individual
functions, but the whole `pub mod upgrade_install { ... }` block.

Verify no imports from `super::upgrade_check` reference removed functions.

#### Step 4.2: Audit for orphaned cross-references

Search the entire workspace for references to the old function names to catch any callers that
were missed:
```bash
rg "install_upgrade_command_with_spinner_and_ctrl_c\|report_upgrade_install_result\|UPGRADE_REQUIRED" --type rust
```

This should return zero results (other than in the new r3bl_tui module where the logic now lives
under new names).

#### Step 4.3: Verify compilation proves completeness

`./check.fish --check` across the whole workspace. If any caller still references a deleted
function, the compiler will catch it. This is the ultimate audit.

### Step 5: Testing

#### Step 5.0: Unit tests in r3bl_tui upgrade module

**`version_check.rs` tests:**
- `UpgradeCheckResult::new()` starts as not required (inner AtomicU8 == 0)
- `set_upgrade_required(true)` / `is_upgrade_required()` round-trip
- `set_upgrade_required(false)` resets back
- `get_bin_name_from_current_exe()` returns non-empty string

**`run_upgrade.rs` tests:**
- `extract_rustup_progress()`: empty input, single line, multi-line (returns last meaningful
  line), long line truncation at 50 chars, `"info: "` prefix stripping

**`ui_strings.rs` tests:**
- Message functions contain the parameterized crate name
- `upgrade_available_notification()` contains both bin name and crate name

#### Step 5.1: CLI arg tests in build-infra

- `CLIArg::parse_from(["cargo-rustdoc-fmt", "--upgrade"])` sets `upgrade: true`
- Other flags remain unaffected

#### Step 5.2: Verify cmdr still works

Run full workspace tests to ensure the refactoring didn't break existing cmdr upgrade flow.

### Step 6: Verification

1. `./check.fish --check` - typecheck passes for all workspace crates
2. `./check.fish --build` - compile succeeds
3. `./check.fish --clippy` - no new warnings
4. `./check.fish --test` - all tests pass (including cmdr tests)
5. `cargo install --path build-infra --force` - binary installs
6. `cargo rustdoc-fmt --help` - shows `--upgrade` flag
7. `cargo rustdoc-fmt --upgrade` - runs the TUI upgrade experience
8. `cargo rustdoc-fmt` - normal run, notification appears at end if update available
9. Run Step 4 audit checks to confirm no orphaned code

## Followup Task (not part of this plan)

After this implementation is complete, a separate task is needed to:
- Update `CHANGELOG.md` for all three crates (r3bl_tui, r3bl-cmdr, r3bl-build-infra)
- Follow `docs/release-guide.md` to publish all three crates together to crates.io
- All three must be published in dependency order: r3bl_tui first, then cmdr and build-infra

## Critical Files

| File | Action |
|:-----|:-------|
| `tui/src/core/script/upgrade/mod.rs` | **Create** - barrel exports |
| `tui/src/core/script/upgrade/version_check.rs` | **Create** - background check w/ AtomicU8 |
| `tui/src/core/script/upgrade/run_upgrade.rs` | **Create** - upgrade execution |
| `tui/src/core/script/upgrade/ui_strings.rs` | **Create** - parameterized messages |
| `tui/src/core/script/mod.rs` | **Modify** - add `upgrade` submodule |
| `tui/Cargo.toml` | **Modify** - bump version to 0.7.9 |
| `cmdr/src/analytics_client/upgrade_check.rs` | **Modify** - delete extracted code, use shared |
| `cmdr/src/analytics_client/ui_str.rs` | **Modify** - delete `upgrade_install` module |
| `cmdr/Cargo.toml` | **Modify** - bump version to 0.0.27, r3bl_tui to 0.7.9 |
| `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs` | **Modify** - add `--upgrade` flag |
| `build-infra/src/bin/cargo-rustdoc-fmt.rs` | **Modify** - integrate version check + upgrade |
| `build-infra/Cargo.toml` | **Modify** - bump version to 0.0.6, r3bl_tui to 0.7.9 |

## Existing Code to Reuse

| What | Location |
|:-----|:---------|
| AtomicU8Ext trait | `tui/src/core/common/common_atomic.rs` |
| crates.io version fetch | `tui/src/core/script/crates_api.rs` - `try_get_latest_release_version_from_crates_io()` |
| HTTP client | `tui/src/core/script/http_client.rs` - `create_client_with_user_agent()` |
| PTY command builder | `tui/src/core/pty/pty_command_builder.rs` - `PtyCommandBuilder` |
| Spinner | `tui/src/readline_async/spinner.rs` - `Spinner::try_start()` |
| OSC events | `tui/src/core/osc/osc_event.rs` - `OscEvent` |
| Color functions | `r3bl_tui::{fg_silver_metallic, fg_lizard_green, fg_soft_pink, fg_slate_gray}` |
| ColorWheel gradient | `r3bl_tui::ColorWheel` for the notification message |
