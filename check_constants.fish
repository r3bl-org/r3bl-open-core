# Global Configuration Constants
#
# All configuration for check.fish: paths, timeouts, parallelism, file lists.
# Must be sourced first after script_lib.fish (sets globals used by all other modules).
#
# Performance Optimizations:
# - tmpfs: Builds to $CHECK_TARGET_DIR (RAM-based, eliminates disk I/O)
#   Trade-off: Cache lost on reboot, first post-reboot build is cold
# - CARGO_BUILD_JOBS=2/3 of cores: High parallelism without starving interactive processes.
#   Leaves ~1/3 of cores free for terminal input, IDE, and desktop compositor.
# - nice -n 10: Lower CPU priority for cargo/rustdoc so interactive processes win scheduling.
# - ionice -c2 -n0: Highest I/O priority in best-effort class (no sudo needed).
#   Note: Mainly affects SSD reads (source files); tmpfs writes bypass the block I/O layer.

# Lock/PID file for single-instance enforcement.
# Uses PID file with process liveness check - simpler and fish-compatible.
# Project name for isolation.
set -l project_name (basename $PWD)
set -g CHECK_LOCK_FILE /tmp/check-fish-$project_name.pid

# Project name (folder name) for notifications.
set -g WORKSPACE_NAME (prompt_pwd)

# Sliding window debounce for watch mode (in seconds).
# After detecting a file change, waits for this many seconds of "quiet" (no new changes)
# before running checks. Each new change resets the window, coalescing rapid saves.
# This handles IDE auto-save, formatters, and "oops forgot to save that file" moments.
set -g DEBOUNCE_WINDOW_SECS 1

# Two-tree architecture for build artifacts and metadata:
# 1. SHARED TREE (cargo + IDE + check.fish): Build artifacts shared with rust-analyzer.
# 2. PRIVATE TREE (check.fish only): Metadata and doc staging (isolated from IDE).
#
# Benefits: ~2-3x faster builds (tmpfs), no SSD wear, shared cache with IDE.
#
# PRIVATE TREE: check.fish-owned metadata and doc staging.
# Always under /tmp for tmpfs performance. Independent of CARGO_TARGET_DIR.
set -g CHECK_PROJECT_ROOT /tmp/check-fish-$project_name

# SHARED TREE: cargo build artifacts (shared between check.fish and IDE).
# Respect user's CARGO_TARGET_DIR if set, otherwise default to isolated tmpfs path.
# All doc modes build to staging dirs (private tree), then rsync to serving dir (shared tree).
# This prevents browser tabs from seeing empty doc folders during builds.
if set -q CARGO_TARGET_DIR; and test -n "$CARGO_TARGET_DIR"
    set -g CHECK_TARGET_DIR $CARGO_TARGET_DIR
else
    set -g CHECK_TARGET_DIR $CHECK_PROJECT_ROOT/target
    # Export so cargo picks it up (scoped to this process)
    set -gx CARGO_TARGET_DIR $CHECK_TARGET_DIR
end

# Derived paths for staging and metadata.
set -g CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_PROJECT_ROOT/staging-quick
set -g CHECK_TARGET_DIR_DOC_STAGING_FULL  $CHECK_PROJECT_ROOT/staging-full
set -g CHECK_LOG_FILE               $CHECK_PROJECT_ROOT/check.log
set -g CHECK_BUILD_CONFIG_HASH_FILE $CHECK_PROJECT_ROOT/.build_config_toml_hash
set -g CHECK_DURATION_FILE          $CHECK_PROJECT_ROOT/check_duration.txt

# Single-instance mutex PID file for background full doc builds.
# Coordinates execution between --watch-doc background tasks and --full/--doc runs.
set -g CHECK_FULL_DOC_PID_FILE      $CHECK_PROJECT_ROOT/full_doc_build.pid

# # Design & Architecture: Parallelism & Core Allocation
# Targets 75% of P-core (Performance Core) threads for cargo operations (`cargo check`, `cargo build`, `cargo doc`).
#
# Hybrid CPU Architecture Optimization (e.g. Intel i7-14700 with 16 P-threads + 12 E-threads):
#   - Queries `detect_p_core_threads` and `detect_e_core_threads` (defined in script_lib.fish).
#   - Caps jobs to 75% of P-threads (e.g. 16 P-threads * 0.75 = 12 jobs).
#   - Prevents heavy cargo/rustdoc jobs from spilling onto slower E-cores, eliminating lock
#     contention on /tmp/check-fish-roc/staging-full/doc/.lock.
#   - Leaves all E-cores (12 threads) and remaining P-threads (4 threads) completely free for terminal,
#     IDE, and UI responsiveness.
#
# Non-Hybrid CPUs / Fallback:
#   - Uses 75% of total logical cores (`nproc` / `hw.ncpu`).
#
# # Rust Migration Requirement (cargo-monitor / build-infra):
# In Rust implementation, map `CARGO_BUILD_JOBS` using `CpuTopology::detect()`:
#   `pub fn optimal_build_jobs(&self) -> usize { (self.p_core_threads as f64 * 0.75).ceil() as usize }`
if not set -q CARGO_BUILD_JOBS; or test -z "$CARGO_BUILD_JOBS"
    set -l p_threads (detect_p_core_threads)
    set -gx CARGO_BUILD_JOBS (math "ceil($p_threads * 0.75)")
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

# Maximum size (in GiB) for managed directories before triggering automatic cleanup.
# 16 GiB gives headroom for incremental artifacts without thrashing.
set -g MAX_TARGET_SIZE_GB 16
