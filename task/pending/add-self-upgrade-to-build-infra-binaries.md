# Add Self-Upgrade Capability to r3bl-build-infra Binaries

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Overview](#overview)
  - [Task Description](#task-description)
  - [Current State](#current-state)
    - [r3bl-cmdr Self-Upgrade (Reference Implementation)](#r3bl-cmdr-self-upgrade-reference-implementation)
    - [r3bl-build-infra Current State](#r3bl-build-infra-current-state)
  - [Goals](#goals)
  - [Architecture Overview](#architecture-overview)
    - [Option 1: Keep Current Pattern (Minimal Refactoring)](#option-1-keep-current-pattern-minimal-refactoring)
    - [Option 2: Create Shared Upgrade Module in r3bl_tui](#option-2-create-shared-upgrade-module-in-r3bl_tui)
    - [Recommendation: Option 2](#recommendation-option-2)
- [Implementation Plan](#implementation-plan)
  - [Phase 1: Extract Core Upgrade Logic to r3bl_tui](#phase-1-extract-core-upgrade-logic-to-r3bl_tui)
    - [Step 1.0: Create upgrade module structure in r3bl_tui [PENDING]](#step-10-create-upgrade-module-structure-in-r3bl_tui-pending)
    - [Step 1.1: Extract installation orchestration [PENDING]](#step-11-extract-installation-orchestration-pending)
    - [Step 1.2: Create reusable abstractions [PENDING]](#step-12-create-reusable-abstractions-pending)
    - [Step 1.3: Add tests for upgrade module [PENDING]](#step-13-add-tests-for-upgrade-module-pending)
  - [Phase 2: Refactor r3bl-cmdr to Use Shared Module](#phase-2-refactor-r3bl-cmdr-to-use-shared-module)
    - [Step 2.0: Update cmdr to use r3bl_tui::upgrade [PENDING]](#step-20-update-cmdr-to-use-r3bl_tuiupgrade-pending)
    - [Step 2.1: Verify cmdr upgrade functionality [PENDING]](#step-21-verify-cmdr-upgrade-functionality-pending)
  - [Phase 3: Add Self-Upgrade to r3bl-build-infra](#phase-3-add-self-upgrade-to-r3bl-build-infra)
    - [Step 3.0: Add upgrade check to cargo-rustdoc-fmt [PENDING]](#step-30-add-upgrade-check-to-cargo-rustdoc-fmt-pending)
    - [Step 3.1: Implement upgrade UI for build-infra binaries [PENDING]](#step-31-implement-upgrade-ui-for-build-infra-binaries-pending)
    - [Step 3.2: Add CLI flag for upgrade behavior [PENDING]](#step-32-add-cli-flag-for-upgrade-behavior-pending)
  - [Phase 4: Documentation and Testing](#phase-4-documentation-and-testing)
    - [Step 4.0: Add documentation for upgrade module [PENDING]](#step-40-add-documentation-for-upgrade-module-pending)
    - [Step 4.1: Integration testing [PENDING]](#step-41-integration-testing-pending)
- [Technical Details](#technical-details)
  - [Existing Infrastructure in r3bl_tui](#existing-infrastructure-in-r3bl_tui)
  - [What Needs to be Extracted from cmdr](#what-needs-to-be-extracted-from-cmdr)
  - [Proposed r3bl_tui::upgrade API](#proposed-r3bl_tuiupgrade-api)
  - [Usage Example for build-infra](#usage-example-for-build-infra)
- [Success Metrics](#success-metrics)
- [Risks and Mitigations](#risks-and-mitigations)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

## Task Description

Add self-upgrade capability to all binaries in `r3bl-build-infra` crate (currently `cargo-rustdoc-fmt`,
with more binaries planned). The upgrade functionality should mirror what `r3bl-cmdr` already provides
for `edi` and `giti` binaries.

To achieve this without code duplication, we need to **lift the upgrade infrastructure out of
`r3bl-cmdr` into `r3bl_tui`**, making it a reusable facility that any R3BL binary can leverage.

## Current State

### r3bl-cmdr Self-Upgrade (Reference Implementation)

The `r3bl-cmdr` crate has a sophisticated self-upgrade system in
`cmdr/src/analytics_client/upgrade_check.rs` (415 lines) that provides:

1. **Background Version Check**: Spawns async task at startup to check crates.io for newer version
2. **Version Comparison**: Compares compile-time version against latest crates.io version
3. **User Prompt on Exit**: Shows upgrade availability and prompts user (yes/no)
4. **Two-Phase Installation**:
   - Phase 1: `rustup toolchain install nightly --force`
   - Phase 2: `cargo +nightly install r3bl-cmdr`
5. **Progress UI**: Spinner with real-time progress from PTY output and OSC events
6. **Ctrl+C Handling**: Graceful cancellation support

**Key Dependencies Used:**
- `r3bl_tui::try_get_latest_release_version_from_crates_io()` - already in tui
- `r3bl_tui::PtyCommandBuilder` - PTY-based command execution
- `r3bl_tui::Spinner` - progress display
- `r3bl_tui::choose()` - yes/no prompts

### r3bl-build-infra Current State

- Contains `cargo-rustdoc-fmt` binary (with more planned via `[[bin]]` sections)
- Already depends on `r3bl_tui` (version 0.7.7, local path)
- Uses `r3bl_tui::core::script::{try_get_changed_files_by_ext, try_is_git_repo}`
- **No self-upgrade capability** - users must manually run `cargo install --force`
- Uses tokio async runtime

## Goals

1. **Enable self-upgrade for `cargo-rustdoc-fmt`** and future build-infra binaries
2. **Extract reusable upgrade logic** from cmdr into r3bl_tui
3. **Maintain cmdr functionality** - refactor cmdr to use the shared module
4. **Provide consistent UX** across all R3BL binaries
5. **Support customization** - different crate names, toolchains, messages

## Architecture Overview

### Option 1: Keep Current Pattern (Minimal Refactoring)

Each crate implements its own `upgrade_check` module, duplicating the 415 lines from cmdr.

**Pros:**
- No changes to r3bl_tui
- Each binary can customize freely

**Cons:**
- Code duplication (~400 lines per binary)
- Bug fixes need to be applied multiple places
- Inconsistent behavior risk

### Option 2: Create Shared Upgrade Module in r3bl_tui

Extract common upgrade infrastructure into `r3bl_tui::core::upgrade`, leaving only crate-specific
UI/UX in each binary.

**Pros:**
- Single source of truth
- Bug fixes apply everywhere
- Consistent behavior across all R3BL tools
- Easier to add upgrade to new binaries

**Cons:**
- More upfront work
- r3bl_tui grows in scope

### Recommendation: Option 2

Given the plan to add more binaries to build-infra and the desire for consistent UX across R3BL
tools, Option 2 is strongly recommended.

# Implementation Plan

## Phase 1: Extract Core Upgrade Logic to r3bl_tui

### Step 1.0: Create upgrade module structure in r3bl_tui [PENDING]

Create the new module structure in `tui/src/core/upgrade/`:

```
tui/src/core/upgrade/
├── mod.rs              # Module exports
├── version_check.rs    # Background version checking (reuse crates_api.rs)
├── installer.rs        # PTY-based installation orchestration
├── progress.rs         # Progress tracking and reporting
└── types.rs            # UpgradeConfig, UpgradeProgress, UpgradeError
```

**Tasks:**
- [ ] Create `tui/src/core/upgrade/mod.rs` with public exports
- [ ] Create `types.rs` with `UpgradeConfig`, `UpgradeProgress`, `UpgradeError`
- [ ] Update `tui/src/core/mod.rs` to export the upgrade module
- [ ] Update `tui/src/lib.rs` to re-export upgrade types at crate root

### Step 1.1: Extract installation orchestration [PENDING]

Move the core installation logic from `cmdr/src/analytics_client/upgrade_check.rs` to
`tui/src/core/upgrade/installer.rs`.

**Functions to extract:**
- `run_rustup_update()` - PTY-based rustup execution
- `run_cargo_install_with_progress()` - PTY-based cargo install with OSC progress
- `install_upgrade_with_progress()` - Main orchestration function

**Key design decisions:**
- Use callback-based progress reporting instead of direct spinner updates
- Accept `UpgradeConfig` for customization (crate name, toolchain, messages)
- Return `Result<(), UpgradeError>` for error handling

### Step 1.2: Create reusable abstractions [PENDING]

Create the abstraction types in `tui/src/core/upgrade/types.rs`:

```rust
/// Configuration for the upgrade process
pub struct UpgradeConfig {
    /// Name of the crate to upgrade (e.g., "r3bl-cmdr", "r3bl-build-infra")
    pub crate_name: String,
    /// Current version (typically from env!("CARGO_PKG_VERSION"))
    pub current_version: String,
    /// Rust toolchain to use (e.g., "nightly", "stable")
    pub toolchain: String,
    /// Whether to update the toolchain before installing
    pub update_toolchain: bool,
    /// Optional callback for progress updates
    pub on_progress: Option<Box<dyn Fn(UpgradeProgress) + Send + Sync>>,
}

/// Progress events during upgrade
#[derive(Debug, Clone)]
pub enum UpgradeProgress {
    VersionCheckStarted,
    VersionCheckComplete { latest: String, update_available: bool },
    ToolchainUpdateStarted,
    ToolchainUpdateProgress(String),
    ToolchainUpdateComplete,
    CargoInstallStarted,
    CargoInstallProgress(u8), // 0-100 percentage
    CargoInstallComplete,
}

/// Errors that can occur during upgrade
#[derive(Debug, thiserror::Error)]
pub enum UpgradeError {
    #[error("Version check failed: {0}")]
    VersionCheckFailed(String),
    #[error("Toolchain update failed: {0}")]
    ToolchainUpdateFailed(String),
    #[error("Cargo install failed: {0}")]
    CargoInstallFailed(String),
    #[error("User cancelled the upgrade")]
    UserCancelled,
    #[error("No update available")]
    NoUpdateAvailable,
}

/// Result of checking for updates
pub struct VersionCheckResult {
    pub current: String,
    pub latest: String,
    pub update_available: bool,
}
```

### Step 1.3: Add tests for upgrade module [PENDING]

Create tests in `tui/src/core/upgrade/` or `tui/tests/`:

**Tasks:**
- [ ] Unit tests for version comparison logic
- [ ] Integration tests for PTY-based installation (may need to be optional/feature-gated)
- [ ] Mock tests for progress callback system

## Phase 2: Refactor r3bl-cmdr to Use Shared Module

### Step 2.0: Update cmdr to use r3bl_tui::upgrade [PENDING]

Refactor `cmdr/src/analytics_client/upgrade_check.rs` to:

1. Import types from `r3bl_tui::upgrade`
2. Use `r3bl_tui::upgrade::install_upgrade_with_progress()` for installation
3. Keep cmdr-specific code:
   - UI strings and messages
   - Analytics integration
   - Exit flow handling
   - Spinner display (using progress callbacks)

**Expected reduction:** ~415 lines → ~150-200 lines (cmdr-specific UI/UX only)

### Step 2.1: Verify cmdr upgrade functionality [PENDING]

**Tasks:**
- [ ] Test `edi` binary upgrade flow end-to-end
- [ ] Test `giti` binary upgrade flow end-to-end
- [ ] Verify analytics events are still reported
- [ ] Test Ctrl+C cancellation
- [ ] Test with no update available
- [ ] Test with network failure

## Phase 3: Add Self-Upgrade to r3bl-build-infra

### Step 3.0: Add upgrade check to cargo-rustdoc-fmt [PENDING]

Create `build-infra/src/upgrade_check.rs` (thin wrapper around r3bl_tui::upgrade):

```rust
use r3bl_tui::upgrade::{
    UpgradeConfig, UpgradeProgress, VersionCheckResult,
    check_for_update, install_upgrade_with_progress,
};

/// Check if an upgrade is available (called at startup, async background task)
pub fn start_upgrade_check_task() {
    tokio::spawn(async {
        // Similar pattern to cmdr - set a flag if update available
    });
}

/// Show upgrade prompt and handle installation (called on exit if update available)
pub async fn handle_upgrade_on_exit() -> miette::Result<()> {
    // Use r3bl_tui::choose() for prompt
    // Use r3bl_tui::Spinner for progress display
    // Call r3bl_tui::upgrade::install_upgrade_with_progress()
}
```

### Step 3.1: Implement upgrade UI for build-infra binaries [PENDING]

Create build-infra specific UI in `build-infra/src/cargo_rustdoc_fmt/upgrade_ui.rs`:

**Tasks:**
- [ ] Design exit messages appropriate for cargo subcommand context
- [ ] Implement spinner with progress display
- [ ] Handle the yes/no prompt
- [ ] Integrate with binary entry point

### Step 3.2: Add CLI flag for upgrade behavior [PENDING]

Add to `build-infra/src/cargo_rustdoc_fmt/cli_arg.rs`:

```rust
#[derive(Parser)]
pub struct Args {
    // ... existing args ...

    /// Skip upgrade check on exit
    #[arg(long, default_value = "false")]
    pub no_upgrade_check: bool,

    /// Check for updates and exit
    #[arg(long)]
    pub check_update: bool,

    /// Upgrade to latest version and exit
    #[arg(long)]
    pub upgrade: bool,
}
```

## Phase 4: Documentation and Testing

### Step 4.0: Add documentation for upgrade module [PENDING]

**Tasks:**
- [ ] Rustdoc for all public types in `r3bl_tui::upgrade`
- [ ] Usage examples in rustdoc
- [ ] Update r3bl_tui README with upgrade capability
- [ ] Add example in `tui/examples/` showing upgrade integration

### Step 4.1: Integration testing [PENDING]

**Tasks:**
- [ ] Test upgrade flow on Linux
- [ ] Test upgrade flow on macOS (if possible)
- [ ] Test with various network conditions
- [ ] Test cancellation at each stage
- [ ] Document manual test procedures

# Technical Details

## Existing Infrastructure in r3bl_tui

Already available and public:

| Function | Location | Purpose |
|----------|----------|---------|
| `try_get_latest_release_version_from_crates_io()` | `tui/src/core/script/crates_api.rs` | Fetch latest version |
| `try_get_latest_release_tag_from_github()` | `tui/src/core/script/github_api.rs` | Fetch GitHub releases |
| `create_client_with_user_agent()` | `tui/src/core/script/http_client.rs` | Create HTTP client |
| `PtyCommandBuilder` | `tui/src/core/pty/pty_command_builder.rs` | Spawn PTY commands |
| `PtyReadOnlyOutputEvent` | `tui/src/core/pty/pty_read_only.rs` | PTY output events |
| `Spinner`, `SpinnerStyle` | `tui/src/tui/spinner/` | Progress display |
| `choose()`, `HowToChoose` | `tui/src/tui/dialog/choose/` | User prompts |

## What Needs to be Extracted from cmdr

From `cmdr/src/analytics_client/upgrade_check.rs`:

| Function | Lines | Extract? | Notes |
|----------|-------|----------|-------|
| `start_task_to_check_if_upgrade_is_needed()` | ~30 | Pattern only | Uses static ATOMIC flag |
| `show_exit_message()` | ~80 | No | cmdr-specific UI |
| `install_upgrade_command_with_spinner_and_ctrl_c()` | ~100 | Yes | Core orchestration |
| `run_rustup_update()` | ~60 | Yes | PTY-based rustup |
| `run_cargo_install_with_progress()` | ~80 | Yes | PTY with OSC parsing |
| `handle_osc_event()` | ~30 | Yes | OSC progress parsing |

## Proposed r3bl_tui::upgrade API

```rust
// tui/src/core/upgrade/mod.rs

pub use types::{UpgradeConfig, UpgradeProgress, UpgradeError, VersionCheckResult};
pub use version_check::check_for_update;
pub use installer::{install_upgrade, InstallOptions};

/// Check if a newer version is available on crates.io
pub async fn check_for_update(crate_name: &str, current_version: &str)
    -> Result<VersionCheckResult, UpgradeError>;

/// Install the latest version of a crate
pub async fn install_upgrade(
    config: UpgradeConfig,
    options: InstallOptions,
) -> Result<(), UpgradeError>;

/// Install options
pub struct InstallOptions {
    /// Toolchain to use (default: "nightly")
    pub toolchain: String,
    /// Update toolchain before install (default: true)
    pub update_toolchain: bool,
    /// Progress callback
    pub on_progress: Option<Box<dyn Fn(UpgradeProgress) + Send + Sync>>,
    /// Cancellation token
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
}
```

## Usage Example for build-infra

```rust
// build-infra/src/bin/cargo-rustdoc-fmt.rs

use r3bl_tui::upgrade::{check_for_update, install_upgrade, InstallOptions, UpgradeProgress};
use r3bl_tui::{Spinner, SpinnerStyle, choose, HowToChoose};

async fn check_and_prompt_upgrade() -> miette::Result<()> {
    let result = check_for_update(
        "r3bl-build-infra",
        env!("CARGO_PKG_VERSION"),
    ).await?;

    if !result.update_available {
        return Ok(());
    }

    println!("New version available: {} -> {}", result.current, result.latest);

    let choice = choose(
        "Would you like to upgrade now?",
        &["Yes", "No"],
        HowToChoose::Single,
    )?;

    if choice == "Yes" {
        let spinner = Spinner::new(SpinnerStyle::default());

        install_upgrade(
            UpgradeConfig {
                crate_name: "r3bl-build-infra".into(),
                current_version: env!("CARGO_PKG_VERSION").into(),
                ..Default::default()
            },
            InstallOptions {
                on_progress: Some(Box::new(move |progress| {
                    match progress {
                        UpgradeProgress::ToolchainUpdateProgress(msg) => {
                            spinner.set_message(&msg);
                        }
                        UpgradeProgress::CargoInstallProgress(pct) => {
                            spinner.set_message(&format!("Installing... {}%", pct));
                        }
                        _ => {}
                    }
                })),
                ..Default::default()
            },
        ).await?;

        println!("Upgrade complete!");
    }

    Ok(())
}
```

# Success Metrics

- [ ] `cargo-rustdoc-fmt` can check for and install updates
- [ ] cmdr upgrade functionality unchanged after refactor
- [ ] Shared upgrade code in r3bl_tui is well-documented
- [ ] New binaries can add upgrade support with <100 lines of crate-specific code
- [ ] All upgrade flows work on Linux (primary target)
- [ ] Ctrl+C cancellation works at any point

# Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking cmdr upgrade during refactor | High | Comprehensive testing before/after |
| PTY differences across platforms | Medium | Feature-gate platform-specific code |
| Network failures during upgrade | Low | Graceful error messages, retry option |
| User confusion about nightly toolchain | Low | Clear messaging about why nightly is needed |
| Upgrade during active file processing | Medium | Only prompt on clean exit |
