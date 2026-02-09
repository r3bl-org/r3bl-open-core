# Argument Parsing & Help Display
#
# Command-line interface for check.fish: parses arguments into mode strings,
# and displays colorful help with usage, features, and workflow documentation.

# Parse command line arguments and return the mode
# Returns: "help", "check", "build", "clippy", "test", "doc", "full",
#          "watch", "watch-test", "watch-doc", or "normal"
function parse_arguments
    if test (count $argv) -eq 0
        echo "normal"
        return 0
    end

    switch $argv[1]
        case --help -h
            echo "help"
            return 0
        case --check
            echo "check"
            return 0
        case --build
            echo "build"
            return 0
        case --clippy
            echo "clippy"
            return 0
        case --full
            echo "full"
            return 0
        case --watch -w
            echo "watch"
            return 0
        case --watch-test
            echo "watch-test"
            return 0
        case --watch-doc
            echo "watch-doc"
            return 0
        case --kill
            echo "kill"
            return 0
        case --doc
            echo "doc"
            return 0
        case --test
            echo "test"
            return 0
        case '*'
            echo "❌ Unknown argument: $argv[1]" >&2
            echo "Use --help for usage information" >&2
            return 1
    end
end

# Display colorful help information
function show_help
    set_color green --bold
    echo "check.fish"
    set_color normal
    echo ""

    set_color yellow
    echo "PURPOSE:"
    set_color normal
    echo "  Comprehensive build and test verification for r3bl-open-core"
    echo "  Validates toolchain, runs tests, doctests, and builds documentation"
    echo ""

    set_color yellow
    echo "USAGE:"
    set_color normal
    echo "  ./check.fish              Run default checks (tests, doctests, docs)"
    echo "  ./check.fish --check      Run typecheck only (cargo check)"
    echo "  ./check.fish --build      Run build only (cargo build)"
    echo "  ./check.fish --clippy     Run clippy only (cargo clippy --all-targets)"
    echo "  ./check.fish --test       Run tests only (cargo test + doctests)"
    echo "  ./check.fish --doc        Build documentation only (quick, no deps)"
    echo "  ./check.fish --full       Run ALL checks (check + build + clippy + tests + doctests + docs)"
    echo "  ./check.fish --watch      Watch mode: run default checks on changes"
    echo "  ./check.fish --watch-test Watch mode: run tests/doctests only"
    echo "  ./check.fish --watch-doc  Watch mode: run doc build (full with deps)"
    echo "  ./check.fish --help       Show this help message"
    echo "  ./check.fish --kill       Kill any running watch instances and cleanup"
    echo ""

    set_color yellow
    echo "FEATURES:"
    set_color normal
    echo "  ✓ Single instance enforcement in watch modes (kills previous watch instances)"
    echo "  ✓ Config change detection (auto-cleans stale artifacts)"
    echo "  ✓ Automatic toolchain validation and repair"
    echo "  ✓ Corrupted toolchain detection and recovery (Missing manifest, etc.)"
    echo "  ✓ ICE escalation to rust-toolchain-update.fish (finds stable nightly)"
    echo "  ✓ Fast tests using cargo test"
    echo "  ✓ Documentation tests (doctests)"
    echo "  ✓ Documentation building"
    echo "  ✓ Blind spot recovery (catch-up build for changes during doc build)"
    echo "  ✓ Auto-recovery from ICE and stale build artifacts (cleans cache, retries)"
    echo "  ✓ Desktop notifications on toolchain changes"
    echo "  ✓ Target directory auto-recovery in watch modes"
    echo "  ✓ Orphan doc file cleanup (full builds detect and remove stale files)"
    echo "  ✓ Performance optimizations (tmpfs, ionice, parallel jobs)"
    echo "  ✓ Comprehensive logging (all modes log to /tmp/roc/check.log)"
    echo "  ✓ One-off + watch-doc can run simultaneously (no lock contention)"
    echo ""

    set_color yellow
    echo "ONE-OFF MODES:"
    set_color normal
    echo "  (default)     Runs default checks: tests, doctests, docs"
    echo "  --check       Runs typecheck only: cargo check (fast compile check)"
    echo "  --build       Runs build only: cargo build (compile production code)"
    echo "  --clippy      Runs clippy only: cargo clippy --all-targets (lint warnings)"
    echo "  --test        Runs tests only: cargo test + doctests"
    echo "  --doc         Builds documentation only (--no-deps, quick check)"
    echo "  --full        Runs ALL checks: check + build + clippy + tests + doctests + docs"
    echo "                Auto-recovers from ICE (escalates to rust-toolchain-update.fish)"
    echo "                and stale build artifacts (cleans cache, retries)"
    echo ""

    set_color yellow
    echo "WATCH MODES:"
    set_color normal
    echo "  --watch       Runs all checks: tests, doctests, docs (full with deps)"
    echo "  --watch-test  Runs tests only: cargo test + doctests (faster iteration)"
    echo "  --watch-doc   Runs quick docs first, then forks full docs to background"
    echo ""
    echo "  Watch mode options:"
    echo "  Monitors: cmdr/src/, analytics_schema/src/, tui/src/, plus all config files"
    echo "  Toolchain: Validated once at startup, before watch loop begins"
    echo "  Behavior: Continues watching even if checks fail"
    echo "  Requirements: inotifywait (Linux) or fswatch (macOS) - installed via bootstrap.sh"
    echo ""
    echo "  Doc Builds (--watch-doc):"
    echo "  • Quick build (r3bl_tui only) runs first, blocking (~3-5s)"
    echo "  • Catch-up: detects files changed during build, rebuilds if needed"
    echo "  • Full build (all crates + deps) then forks to background (~90s)"
    echo "  • Quick docs available immediately, full docs notify when done"
    echo "  • Desktop notification when each build completes"
    echo "  • Separate staging directories prevent race conditions"
    echo "  • Output logged to /tmp/roc/check.log for debugging"
    echo ""
    echo "  Orphan File Cleanup (full builds only):"
    echo "  • Long-running sessions accumulate stale docs from renamed/deleted files"
    echo "  • Detection: compares file counts between staging and serving directories"
    echo "  • If serving has MORE files than staging, orphans exist"
    echo "  • Full builds use rsync --delete to clean orphaned files"
    echo "  • Quick builds never delete (would wipe dependency docs)"
    echo ""
    echo "  Sliding Window Debounce:"
    echo "  • Waits for $DEBOUNCE_WINDOW_SECS seconds of 'quiet' (no new changes) before dispatching"
    echo "  • Each new file change resets the window, coalescing rapid saves"
    echo "  • Handles: IDE auto-save, formatters, 'forgot to save that file' moments"
    echo "  • Adjust DEBOUNCE_WINDOW_SECS in script if needed"
    echo ""
    echo "  Target Directory Auto-Recovery:"
    echo "  • Monitors for missing target/ directory (every "$TARGET_CHECK_INTERVAL_SECS"s)"
    echo "  • Auto-triggers rebuild if target/ is deleted externally"
    echo "  • Recovers from: cargo clean, manual rm -rf target/, IDE cache clearing"
    echo ""

    set_color yellow
    echo "CONFIG CHANGE DETECTION (all modes):"
    set_color normal
    echo "  Automatically detects config file changes and cleans stale build artifacts."
    echo "  Works in ALL modes: one-off (--test, --doc, default) and watch modes."
    echo ""
    echo "  Monitored files:"
    echo "  • Cargo.toml (root + all workspace crates, dynamically detected)"
    echo "  • rust-toolchain.toml"
    echo "  • .cargo/config.toml"
    echo ""
    echo "  Algorithm:"
    echo "  1. Concatenate all config file contents"
    echo "  2. Compute SHA256 hash of concatenated content"
    echo "  3. Compare with stored hash in target/check/.config_hash"
    echo "  4. If different: clean target/check, store new hash, rebuild"
    echo "  5. If same: proceed without cleaning (artifacts are valid)"
    echo ""
    echo "  Handles these scenarios:"
    echo "  • Toggling incremental compilation on/off"
    echo "  • Changing optimization levels or profiles"
    echo "  • Updating Rust toolchain version"
    echo "  • Adding/removing dependencies in any crate"
    echo ""

    set_color yellow
    echo "TOOLCHAIN CORRUPTION RECOVERY:"
    set_color normal
    echo "  Detects and recovers from corrupted toolchain installations."
    echo ""
    echo "  Symptoms of corruption:"
    echo "  • 'Missing manifest in toolchain' errors"
    echo "  • Repeated 'syncing channel updates' loops that never complete"
    echo "  • Toolchain appears in 'rustup toolchain list' but doesn't work"
    echo ""
    echo "  Common causes:"
    echo "  • Interrupted installation (Ctrl+C, network failure, power loss)"
    echo "  • Corrupted download cache"
    echo "  • Manifest file loss or corruption"
    echo ""
    echo "  Recovery process:"
    echo "  1. Detects corruption BEFORE normal validation (prevents loops)"
    echo "  2. Tries 'rustup toolchain uninstall' first"
    echo "  3. Falls back to direct folder deletion (~/.rustup/toolchains/)"
    echo "  4. Clears rustup caches (~/.rustup/downloads/, ~/.rustup/tmp/)"
    echo "  5. Reinstalls via rust-toolchain-sync-to-toml.fish"
    echo ""
    echo "  Visibility improvements:"
    echo "  • On sync failure: shows last 30 lines of output (not silent)"
    echo "  • Reports specific failure reason (e.g., 'rust-analyzer missing')"
    echo "  • Points to full log: ~/Downloads/rust-toolchain-sync-to-toml.log"
    echo ""

    set_color yellow
    echo "WORKFLOW:"
    set_color normal
    echo "  1. Checks for config file changes (cleans target if needed)"
    echo "  2. Checks for corrupted toolchain (force-removes if detected)"
    echo "  3. Validates Rust toolchain (nightly + components)"
    echo "  4. Auto-installs/repairs toolchain if needed"
    echo "  5. Runs cargo test (all unit and integration tests)"
    echo "  6. Runs doctests"
    echo "  7. Builds documentation:"
    echo "     • One-off --doc:  cargo doc --no-deps (quick, your crates only)"
    echo "     • --watch-doc:    Forks both quick + full builds to background"
    echo "     • Other watch:    cargo doc (full, includes dependencies)"
    echo "  8. On ICE: removes target/, retries once"
    echo ""

    set_color yellow
    echo "NOTIFICATIONS:"
    set_color normal
    echo "  Desktop notifications alert you when checks complete."
    echo ""
    echo "  Platform support:"
    echo "  • Linux: gdbus (GNOME) with notify-send fallback"
    echo "  • macOS: osascript (native AppleScript)"
    echo ""
    echo "  When notifications are sent:"
    echo "  • One-off modes (--test, --doc): On success only if duration > $NOTIFICATION_THRESHOLD_SECS""s"
    echo "  • One-off modes: Always on failure (you need to know!)"
    echo "  • Default mode (all checks): Always on completion"
    echo "  • Watch modes: On success and failure"
    echo "  • Toolchain installation: On install success/failure"
    echo ""
    echo "  Auto-dismiss behavior:"
    echo "  • All notifications auto-dismiss after "(math $NOTIFICATION_EXPIRE_MS / 1000)" seconds"
    echo "  • Linux/GNOME: Uses gdbus + CloseNotification (GNOME ignores --expire-time)"
    echo "  • macOS: System handles auto-dismiss automatically"
    echo ""
    echo "  Rationale: Quick one-off operations (<$NOTIFICATION_THRESHOLD_SECS""s) don't need notifications"
    echo "  since you're likely still watching the terminal. Longer operations trigger"
    echo "  notifications because you've probably switched to your IDE."
    echo ""

    set_color yellow
    echo "PERFORMANCE OPTIMIZATIONS:"
    set_color normal
    echo "  This script uses several techniques to maximize build speed:"
    echo ""
    echo "  1. tmpfs Build Directory (/tmp/roc/target/check):"
    echo "     • Builds happen in RAM instead of disk - eliminates I/O bottleneck"
    echo "     • /tmp is typically a tmpfs mount (RAM-based filesystem)"
    echo "     • ⚠️  Trade-off: Build cache is lost on reboot"
    echo "     • First build after reboot will be a cold start (slower)"
    echo "     • Subsequent builds use cached artifacts (fast)"
    echo ""
    echo "  2. Parallel Jobs (CARGO_BUILD_JOBS=$CARGO_BUILD_JOBS):"
    echo "     • Auto-detected core count: nproc (Linux) or sysctl (macOS)"
    echo "     • Benchmarked: 4 min vs 10 min for cargo doc (60% speedup)"
    echo "     • Despite cargo docs, this doesn't always default to nproc"
    echo ""
    echo "  3. I/O Priority (ionice -c2 -n0):"
    echo "     • Gives cargo highest I/O priority in best-effort class"
    echo "     • Helps when other processes compete for disk/tmpfs access"
    echo "     • No sudo required (unlike realtime I/O class)"
    echo ""
    echo "  4. Two-Stage Doc Build:"
    echo "     • Docs are built to staging directory first"
    echo "     • Only synced to serving directory on success"
    echo "     • Browser never sees incomplete/missing docs during rebuilds"
    echo ""
    echo "  5. Incremental Compilation:"
    echo "     • Rust only recompiles what changed"
    echo "     • Pre-compiled .rlib files stay cached between runs"
    echo "     • Test executables are cached - re-running just executes the binary"
    echo ""

    set_color yellow
    echo "WHY IS IT SO FAST?"
    set_color normal
    echo "  Example output (2,700+ tests in ~9 seconds with warm cache):"
    echo ""
    echo "    ./check.fish"
    echo ""
    echo "    🚀 Running checks..."
    echo ""
    echo "    [03:41:57 PM] ▶️  Running tests..."
    echo "    [03:42:01 PM] ✅ tests passed (3s)"
    echo ""
    echo "    [03:42:01 PM] ▶️  Running doctests..."
    echo "    [03:42:06 PM] ✅ doctests passed (5s)"
    echo ""
    echo "    [03:42:06 PM] ▶️  Running docs..."
    echo "    [03:42:08 PM] ✅ docs passed (1s)"
    echo ""
    echo "    [03:42:08 PM] ✅ All checks passed!"
    echo ""
    echo "  The speed comes from a combination of techniques:"
    echo "  • tmpfs (RAM disk): All I/O happens in RAM, no SSD/HDD seeks"
    echo "  • Incremental compilation: Only changed modules rebuild"
    echo "  • Cached test binaries: Re-running tests just executes the binary"
    echo "  • 28 parallel jobs: Maximizes CPU utilization during compilation"
    echo ""
    echo "  Trade-off: First build after reboot is cold (~2-4 min),"
    echo "  but subsequent runs stay blazing fast."
    echo ""

    set_color yellow
    echo "EXIT CODES:"
    set_color normal
    echo "  0  All checks passed ✅"
    echo "  1  Checks failed or toolchain installation failed ❌"
    echo ""

    set_color yellow
    echo "EXAMPLES:"
    set_color normal
    echo "  # Run checks once"
    echo "  ./check.fish"
    echo ""
    echo "  # Watch for changes and auto-run all checks"
    echo "  ./check.fish --watch"
    echo ""
    echo "  # Watch for changes and auto-run tests/doctests only (faster iteration)"
    echo "  ./check.fish --watch-test"
    echo ""
    echo "  # Watch for changes and auto-run doc build only"
    echo "  ./check.fish --watch-doc"
    echo ""
    echo "  # Show this help"
    echo "  ./check.fish --help"
    echo ""
end
