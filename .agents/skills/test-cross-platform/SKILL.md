---
name: test-cross-platform
description: Synchronize repository changes to remote test fleet (Linux, macOS, Windows) and execute the full test suite across all platforms concurrently.
---

# Cross-Platform Fleet Testing

Run the full test suite across the remote cross-platform fleet (Linux on `nazmul-mobile.local`, macOS, and Windows)
to verify compatibility, platform gates, and asynchronous I/O behavior.

## When to Use

- After modifying cross-platform code, platform-specific `#[cfg(...)]` gates, or PTY logic.
- Before merging pull requests or finalizing release milestones.
- When the user runs `/test-cross-platform` or asks to test across platforms/fleet.

## Worktree & Folder Detection

The skill dynamically detects whether you are in the primary repository or a git worktree:

```bash
# Fish syntax:
set -l REPO_ROOT (git rev-parse --show-toplevel)
set -l FOLDER_NAME (basename "$REPO_ROOT")

# Bash syntax:
REPO_ROOT=$(git rev-parse --show-toplevel)
FOLDER_NAME=$(basename "$REPO_ROOT")
```

If you are in `~/github/roc`, `$FOLDER_NAME` is `roc`. If you are in a worktree such as `~/github/roc-fix-shift-home-lockup`, `$FOLDER_NAME` is `roc-fix-shift-home-lockup`. Remote fleet machines mirror this folder under their respective repository parent paths.

### Worktree Portability & Relative Paths (`worktree.useRelativePaths`)

Linked git worktrees store administrative pointer files:
- Inside the worktree: `.git` (file containing `gitdir: <path_to_main_repo>/.git/worktrees/<name>`)
- Inside the main repo: `.git/worktrees/<name>/gitdir` (file containing `<path_to_worktree>/.git`)

By default, Git writes **absolute paths** (`/home/nazmul/...` on Linux vs `/Users/nazmul/...` on macOS). If worktrees or their `.git` files are mirrored across systems with different home directory roots, Git will fail on the remote host with:
```
fatal: not a git repository: (null)
```
which causes any tests or tools that invoke `git` (such as `cargo-rustdoc-fmt` validation tests in `build-infra`) to fail.

To prevent this:
1. **Always enable relative worktree paths globally**:
   ```bash
   git config --global worktree.useRelativePaths true
   ```
   Or explicitly pass `--relative` when creating a worktree:
   ```bash
   git worktree add --relative ../<worktree_folder> <branch>
   ```
2. **To repair existing worktrees with absolute path mismatches**:
   ```bash
   cd ~/github/roc
   git config worktree.useRelativePaths true
   git worktree repair
   ```
   Git will detect the absolute path mismatch and convert all linked worktrees to relative paths in place.

## Fleet Overview

| Platform | Host Address | Shell | Repository Path | Test Runner |
| :--- | :--- | :--- | :--- | :--- |
| **Linux** | `nazmul-mobile.local` | `fish` | `~/github/<folder_name>` | `./check.fish --test` |
| **macOS** | `nazmul-mac.local` | `fish` | `~/github/<folder_name>` | `./check.fish --test` |
| **Windows** | `nazmul-win.local` | `nu` (Nushell) | `github/<folder_name>` | `cargo test` |

## Workflow

### Step 1: Fleet Synchronization

Ensure the target folder exists on each remote fleet machine, then synchronize the current worktree (always excluding `.git` and `target/` so git worktree files and platform binaries are not overwritten):

1. **Linux (`nazmul-mobile.local`)**:
   ```bash
   ssh nazmul-mobile.local "mkdir -p ~/github/$FOLDER_NAME"
   rsync -av --delete --exclude='.git' --exclude='target' "$REPO_ROOT/" nazmul-mobile.local:"github/$FOLDER_NAME/"
   ```

2. **macOS (`nazmul-mac.local`)**:
   ```bash
   ssh nazmul-mac.local "mkdir -p ~/github/$FOLDER_NAME"
   rsync -av --delete --exclude='.git' --exclude='target' "$REPO_ROOT/" nazmul-mac.local:"github/$FOLDER_NAME/"
   ```

3. **Windows (`nazmul-win.local`)**:
   ```bash
   ssh nazmul-win.local "powershell -NoProfile -Command 'New-Item -ItemType Directory -Force -Path github/$FOLDER_NAME'"
   tar -cz -C "$REPO_ROOT" --exclude=.git --exclude=target . | ssh nazmul-win.local "tar -xz -C github/$FOLDER_NAME"
   ```

All sync operations can run in parallel.

### Step 2: Concurrent Test Execution

Run the full test suite across all three platforms concurrently using non-blocking background tasks (`run_command` with async tasks) so the active conversation remains responsive:

1. **Linux (`nazmul-mobile.local`)**:
   ```bash
   ssh nazmul-mobile.local "cd ~/github/$FOLDER_NAME && ./check.fish --test"
   ```
   *(Alternatively: `ssh nazmul-mobile.local "cd ~/github/$FOLDER_NAME && cargo test"`)*

2. **macOS (`nazmul-mac.local`)**:
   ```bash
   ssh nazmul-mac.local "cd ~/github/$FOLDER_NAME && ./check.fish --test"
   ```
   *(Alternatively: `ssh nazmul-mac.local "cd ~/github/$FOLDER_NAME && cargo test"`)*

3. **Windows (`nazmul-win.local`)**:
   ```bash
   ssh nazmul-win.local "cd github/$FOLDER_NAME; cargo test"
   ```

### Step 3: Targeted Subsystem Testing (Optional)

When testing specific subsystems (like `core::pty`) during active refactoring rather than the entire workspace:

- **Linux**: `ssh nazmul-mobile.local "cd ~/github/$FOLDER_NAME && cargo test -p r3bl_tui core::pty"`
- **macOS**: `ssh nazmul-mac.local "cd ~/github/$FOLDER_NAME && cargo test -p r3bl_tui core::pty"`
- **Windows**: `ssh nazmul-win.local "cd github/$FOLDER_NAME; cargo test -p r3bl_tui core::pty"`

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
