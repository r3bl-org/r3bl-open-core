# Global Configuration Constants
#
# All configuration for check.fish: paths, timeouts, parallelism, file lists.
# Must be sourced first after script_lib.fish (sets globals used by all other modules).
#
# Performance Optimizations:
# - tmpfs: Builds to /tmp/roc/target/check (RAM-based, eliminates disk I/O)
#   Trade-off: Cache lost on reboot, first post-reboot build is cold
# - CARGO_BUILD_JOBS=2/3 of cores: High parallelism without starving interactive processes.
#   Leaves ~1/3 of cores free for terminal input, IDE, and desktop compositor.
# - nice -n 10: Lower CPU priority for cargo/rustdoc so interactive processes win scheduling.
# - ionice -c2 -n0: Highest I/O priority in best-effort class (no sudo needed).
#   Note: Mainly affects SSD reads (source files); tmpfs writes bypass the block I/O layer.

# Lock/PID file for single-instance enforcement.
# Uses PID file with process liveness check - simpler and fish-compatible.
set -g CHECK_LOCK_FILE /tmp/roc/check.fish.pid

# Sliding window debounce for watch mode (in seconds).
# After detecting a file change, waits for this many seconds of "quiet" (no new changes)
# before running checks. Each new change resets the window, coalescing rapid saves.
# This handles IDE auto-save, formatters, and "oops forgot to save that file" moments.
set -g DEBOUNCE_WINDOW_SECS 1

# Use tmpfs for build artifacts - eliminates disk I/O for massive speedup.
# /tmp is already tmpfs on most Linux systems (including this one: 46GB).
# Benefits: ~2-3x faster builds, no SSD wear, isolated from IDE target directories.
# Trade-off: Build cache is lost on reboot (first build after reboot is cold).
#
# CHECK_TARGET_DIR: Primary target for tests and serving docs to browser.
# CHECK_TARGET_DIR_DOC_STAGING_QUICK: Staging for quick doc builds (--no-deps).
# CHECK_TARGET_DIR_DOC_STAGING_FULL: Staging for full doc builds (with deps, runs in background).
# CHECK_LOG_FILE: Log file for all check.fish output (created fresh each run).
#
# Two separate staging dirs prevent race conditions: quick builds can run while
# background full builds are syncing, without files "vanishing" mid-rsync.
set -g CHECK_TARGET_DIR /tmp/roc/target/check
set -g CHECK_TARGET_DIR_DOC_STAGING_QUICK /tmp/roc/target/check-doc-staging-quick
set -g CHECK_TARGET_DIR_DOC_STAGING_FULL /tmp/roc/target/check-doc-staging-full
set -g CHECK_LOG_FILE /tmp/roc/check.log

# Use 2/3 of available cores for cargo operations (ceil to avoid rounding down too far).
# Example: 28 cores → 19 jobs (leaves 9 cores free for interactive processes).
#
# Why not all cores?
#   Full parallelism (nproc) makes terminal input visibly laggy. Each rustdoc/rustc
#   process spawns LLVM codegen threads internally, so N jobs can produce more than N
#   busy threads. Combined with nice -n 10 (see ionice_wrapper in script_lib.fish),
#   the remaining 1/3 of cores stay responsive for the terminal, IDE, and compositor.
#
# Why not fewer?
#   Cargo's compilation parallelism has diminishing returns, but the curve doesn't
#   flatten until well past 2/3. Benchmarks showed ~60% speedup going from cargo's
#   conservative default to explicit nproc; 2/3 of nproc retains most of that gain.
#
# Auto-detect core count: nproc (Linux) or sysctl (macOS).
switch (uname -s)
    case Darwin
        set -gx CARGO_BUILD_JOBS (math "ceil("(sysctl -n hw.ncpu)" * 2 / 3)")
    case '*'
        set -gx CARGO_BUILD_JOBS (math "ceil("(nproc)" * 2 / 3)")
end

# List of config files that affect build artifacts.
# Changes to these files should trigger a clean rebuild to avoid stale artifact issues.
# Used by check_config_changed to detect when target/check needs to be cleaned.
# Dynamically includes: root Cargo.toml, all workspace crate Cargo.toml files,
# rust-toolchain.toml, and .cargo/config.toml.
set -g CONFIG_FILES_TO_WATCH Cargo.toml rust-toolchain.toml .cargo/config.toml
# Dynamically add all workspace crate Cargo.toml files (*/Cargo.toml)
for crate_toml in */Cargo.toml
    if test -f $crate_toml
        set -a CONFIG_FILES_TO_WATCH $crate_toml
    end
end

# Minimum duration (in seconds) before showing desktop notifications for one-off modes.
# If a check completes faster than this, skip the notification since the user is likely
# still looking at the terminal. For longer runs, they've probably switched to their IDE.
set -g NOTIFICATION_THRESHOLD_SECS 1

# Interval (in seconds) for checking if target/check directory exists in watch modes.
# inotifywait will timeout after this interval, allowing us to check for missing target.
# If target/check is missing, a rebuild is triggered automatically.
set -g TARGET_CHECK_INTERVAL_SECS 10

# Auto-dismiss timeout (in milliseconds) for desktop notifications.
# Notifications auto-dismiss to avoid clutter, especially in watch mode.
# 5 seconds = 5000ms. Set to 0 or remove to use system default (persistent).
set -g NOTIFICATION_EXPIRE_MS 5000

# Timeout (in seconds) for all cargo commands (check, build, clippy, test, doctest, doc).
# If any command exceeds this limit, `timeout` kills the process (exit code 124) and
# run_check_with_recovery reports it as a timeout failure. Prevents hanging builds,
# linker stalls, or runaway tests from silently blocking watch mode or interactive sessions.
set -g CHECK_TIMEOUT_SECS 300

# Exit code returned by coreutils `timeout` when the child is killed.
# Used by run_check_with_recovery to distinguish timeouts from other failures.
set -g TIMEOUT_EXIT_CODE 124

# Temp file path for passing duration from run_check_with_recovery to callers.
# Using a well-known path avoids global variable side effects while still
# allowing callers to explicitly opt-in to reading the duration.
set -g CHECK_DURATION_FILE /tmp/check_fish_duration.txt
