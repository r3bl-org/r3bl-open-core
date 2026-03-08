# Task Overview: Add Zombie Process Purging to check.fish and run.fish

## Description
This task implements a robust, surgical system to identify and purge "zombie" processes (STAT `Z`) and their project-related parent processes (matching `r3bl*`) that can leak during test runs. This ensures that hung test binaries don't block subsequent builds (ETXTBSY) or exhaust system PID limits.

## The "Why"
Interactive TUI tests (especially those using PTYs) can sometimes hang or crash without reaping their children. These zombies remain in the process table until their parent is killed, at which point `init` (PID 1) adopts and reaps them. Manually finding and killing these parents is tedious; automating it makes the development loop "self-healing."

## Architecture
- **Shared Utility**: `purge_zombie_processes` in `script_lib.fish` provides a unified implementation for all workspace scripts.
- **Surgical Killing**: The logic only kills a parent if it is the parent of an actual zombie AND either the parent or the zombie is project-related (`r3bl*`).
- **Suicide Prevention**: The function explicitly checks `$fish_pid` to ensure it never kills its own parent shell.
- **Strategic Triggers**:
  - **Startup**: Every run of `check.fish` or `run.fish` cleans up legacy state first.
  - **Recovery**: Automatic retries (on ICE or ETXTBSY) trigger a purge to clear file locks.
  - **Manual**: Integrated into `check.fish --kill` for explicit cleanup.

# Implementation Plan

## Step 0: Research & Core Logic [COMPLETE]
- [x] Analyze `ps` output to identify zombie patterns (`r3bl_tui-` test binaries) [COMPLETE]
- [x] Design the parent-killing logic to allow `init` to reap zombies [COMPLETE]

## Step 1: Implementation of Shared Utility [COMPLETE]
- [x] Create `purge_zombie_processes` in `script_lib.fish` [COMPLETE]
- [x] Implement "suicide prevention" check for `$fish_pid` [COMPLETE]
- [x] Add dual-output logging (support both `log_message` and `echo`) [COMPLETE]

## Step 2: Integration into check.fish Framework [COMPLETE]
- [x] Call purge logic in `main` startup of `check.fish` [COMPLETE]
- [x] Add to `case kill` in `check.fish` [COMPLETE]
- [x] Add to `cleanup_for_recovery` in `check_recovery.fish` [COMPLETE]
- [x] Add to ETXTBSY detection in `check_orchestrators.fish` [COMPLETE]
- [x] Broaden `pkill` pattern for ETXTBSY recovery [COMPLETE]

## Step 3: Integration into Workspace Scripts [COMPLETE]
- [x] Call purge logic in `main` of `run.fish` [COMPLETE]
- [x] Add to `rust-toolchain-update.fish` cleanup routine [COMPLETE]
- [x] Add to `rust-toolchain-validate.fish` comprehensive mode [COMPLETE]

## Step 4: Documentation & Cleanup [COMPLETE]
- [x] Update `--help` in `check_cli.fish` with new feature [COMPLETE]
- [x] Synchronize `README.md` with current script reality (Tmux layout, session names) [COMPLETE]
- [x] Delete outdated `setup-dev-tools.sh` [COMPLETE]
- [x] Add detailed comments explaining the "suicide-proof" nature of the logic [COMPLETE]

# Verification [COMPLETE]
- [x] Verify `fish -n` syntax check on all modified scripts [COMPLETE]
- [x] Confirm `purge_zombie_processes` is visible to both `run.fish` and `check.fish` [COMPLETE]
- [x] Manually verify help text output [COMPLETE]
