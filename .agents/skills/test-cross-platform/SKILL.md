---
name: test-cross-platform
description: Synchronize repository changes to remote test fleet (macOS, Windows) and execute the full test suite across all platforms concurrently.
---

# Cross-Platform Fleet Testing

Run the full test suite across the cross-platform fleet (Linux host, macOS, and Windows)
to verify compatibility, platform gates, and asynchronous I/O behavior.

## When to Use

- After modifying cross-platform code, platform-specific `#[cfg(...)]` gates, or PTY logic.
- Before merging pull requests or finalizing release milestones.
- When the user runs `/test-cross-platform` or asks to test across platforms/fleet.

## Fleet Overview

| Platform | Host Address | Shell | Repository Path | Test Runner |
| :--- | :--- | :--- | :--- | :--- |
| **Linux** | Local host (`nazmul-mobile.local`) | `bash` / `fish` | `~/github/roc` | `./check.fish --test` |
| **macOS** | `nazmul-mac.local` | `fish` | `~/github/roc` | `./check.fish --test` |
| **Windows** | `nazmul-win.local` | `nu` (Nushell) | `github/roc` | `cargo test` |

## Workflow

### Step 1: Fleet Synchronization

Synchronize the local repository to all remote fleet machines using `scp`:

1. **Windows**:
   ```bash
   scp -r ~/github/roc/. nazmul-win.local:github/roc/
   ```

2. **macOS**:
   ```bash
   scp -r ~/github/roc/. nazmul-mac.local:github/roc/
   ```

Both sync operations can run in parallel.

### Step 2: Concurrent Test Execution

Run the full test suite across all three platforms concurrently using non-blocking background tasks (`run_command` with async tasks) so the conversation is not frozen:

1. **Linux (Local)**:
   ```bash
   ./check.fish --test
   ```
   *(Alternatively: `cargo test --workspace`)*

2. **macOS**:
   ```bash
   ssh nazmul-mac.local "cd ~/github/roc && ./check.fish --test"
   ```
   *(Alternatively: `ssh nazmul-mac.local "cd ~/github/roc && cargo test"`)*

3. **Windows**:
   ```bash
   ssh nazmul-win.local "cd github/roc; cargo test"
   ```

### Step 3: Targeted Subsystem Testing (Optional)

When testing specific subsystems (like `core::pty`) during active refactoring rather than the entire workspace:

- **Linux**: `cargo test -p r3bl_tui core::pty`
- **macOS**: `ssh nazmul-mac.local "cd ~/github/roc && cargo test -p r3bl_tui core::pty"`
- **Windows**: `ssh nazmul-win.local "cd github/roc; cargo test -p r3bl_tui core::pty"`

### Step 4: Result Aggregation & Reporting

1. Wait for all three runs to conclude (via reactive task completion messages).
2. Extract summary lines (`test result: ok. <X> passed; <Y> failed; ...`).
3. Present results in a markdown summary table:

| Platform | Host / Machine | Result | Tests Passed | Tests Failed | Duration |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Linux** | `nazmul-mobile.local` | **PASSED** | 53 | 0 | 0.01s |
| **macOS** | `nazmul-mac.local` | **PASSED** | 53 | 0 | 0.02s |
| **Windows** | `nazmul-win.local` | **PASSED** | 55 | 0 | 0.46s |

4. If any test fails, extract the failure diagnostics and failing test names for triage.
5. Emit attention beep signal:
   ```bash
   fish -c "beep"
   ```
