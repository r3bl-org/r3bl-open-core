# Watch Mode Loop, Sliding Window Debounce & Check Dispatch
#
# Watch Mode & Sliding Window Debounce:
# Watch mode uses a single-threaded main loop with sliding window debounce.
# The main thread alternates between BLOCKED (waiting for events/timeout) and
# DISPATCHING (forking build processes). Actual builds run in parallel background processes.
#
# Key Insight: The file watcher (inotifywait on Linux, fswatch on macOS) acts as
# both an event listener AND a timer. This lets us implement sliding window
# debounce without threads:
#   - Exit code 0: File changed (event detected)
#   - Exit code 2: Timeout expired (no events for N seconds)
#   - Exit code 1: Error
#
# Sliding Window Debounce Algorithm:
# Instead of threshold-based debounce ("ignore if last run was < N seconds ago"),
# we use sliding window debounce ("wait for N seconds of quiet before running").
#
# Algorithm:
#   1. Wait for FIRST event (file watcher, no timeout - blocks forever)
#   2. Start sliding window: call file watcher with DEBOUNCE_WINDOW_SECS timeout
#   3. If event arrives (code 0): loop back to step 2 (reset window)
#   4. If timeout expires (code 2): quiet period detected, run build
#   5. After build completes, go back to step 1
#
# ┌─────────────────────────────────────────────────────────────────┐
# │                     MAIN THREAD TIMELINE                        │
# └─────────────────────────────────────────────────────────────────┘
#
#      BLOCKED           BLOCKED           BLOCKED        DISPATCHING
#     (waiting)         (waiting)         (waiting)       (fork builds)
#         │                 │                 │                │
#         ▼                 ▼                 ▼                ▼
#    ┌─────────┐       ┌─────────┐       ┌─────────┐      ┌────────┐
#    │inotify  │       │inotify  │       │inotify  │      │ fork   │──▶ quick build (background)
#    │wait 10s │──────▶│wait 10s │──────▶│wait 10s │─────▶│ builds │──▶ full build (background)
#    └─────────┘       └─────────┘       └─────────┘      └────────┘
#         │                 │                 │                │
#         │                 │                 │                │
#    File changed!    File changed!      TIMEOUT!         Returns
#    (exit code 0)    (exit code 0)     (exit code 2)    immediately
#    "Reset window"   "Reset window"    "Quiet period,
#                                        dispatch builds!"
#
# Example Timeline (Rapid Saves with 1s debounce):
#   0:00  (idle)              Thread BLOCKED, waiting forever for first event
#   0:05  User saves file A   Thread UNBLOCKS, starts 1s debounce window
#   0:05.3 User saves file B  Window RESET (now 1s again)
#   0:05.8 User saves file C  Window RESET again
#   0:06.8 (1s of quiet)      TIMEOUT! Thread forks builds, returns immediately
#   0:06.8 (same instant)     Thread BLOCKED again, waiting for next event
#   0:14  Quick build done    Notification: "Quick docs ready!" (background)
#   1:37  Full build done     Notification: "Full docs built!" (background)
#
# Why This Works:
# - File watcher does the heavy lifting (efficient kernel-level blocking)
# - No CPU burn while waiting (uses kernel's inotify/FSEvents subsystem)
# - Single mechanism handles both "coalesce rapid saves" AND "wait for quiet"
# - No separate event draining or timestamp tracking needed
#
# Events During Build:
# Since builds are forked to background, the main thread returns to the file
# watcher immediately. File events are processed normally while builds run in
# parallel. If a new change arrives, a new build cycle starts (previous builds
# continue in background until completion).
#
# Doc Build Architecture (--watch-doc):
# Uses a two-tier build system with eventual consistency for cross-crate links.
# See script_lib.fish for detailed documentation of the algorithm.
#
# ARCHITECTURE OVERVIEW:
#
#   File change detected
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Quick build (~5-7s) [BLOCKING]              │
#   │ • cargo doc -p r3bl_tui --no-deps           │
#   │ • Fast feedback, broken cross-crate links   │
#   └─────────────────────────────────────────────┘
#       │
#       ├──► Catch-up check (if changes during quick build)
#       │         └──► Quick build → forks Full build
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Full build (~90s) [FORKED TO BACKGROUND]    │
#   │ • cargo doc (all deps)                      │
#   │ • Fixes all cross-crate links               │
#   └─────────────────────────────────────────────┘
#       │
#       └──► Catch-up check (if changes during full build)
#                 │
#                 ▼
#            Quick build (~5-7s) → forks Full build (~90s)
#                 │                      │
#                 ▼                      ▼
#            [fast feedback]      [fixes links eventually]
#
# EVENTUAL CONSISTENCY MODEL:
# The system forms a cycle: quick → full → (if changes) → quick → full → ...
# Termination: When no files change during a build, docs are current and correct.
#
# WHY BROKEN LINKS IN QUICK BUILD?
# The --no-deps flag makes builds fast but rustdoc can't resolve cross-crate
# links (e.g., to crossterm, tokio) because it doesn't know they exist.
# Full build documents everything, so links become correct.
#
# BLIND SPOTS:
# While a build runs, inotifywait isn't watching. Changes during this window
# are detected via `find -newermt "@$epoch"` after each build completes.
#
# STAGING DIRECTORIES:
# - staging-quick/doc/ → Quick builds write here
# - staging-full/doc/  → Full builds write here
# - serving/doc/       → Browser loads from here (both sync to this)
# Separate staging prevents race conditions when builds run concurrently.
#
# Why Quick Blocks but Full Forks?
# - Cargo uses a global package cache lock (~/.cargo/.package-cache)
# - Running both simultaneously causes "Blocking waiting for file lock"
# - Quick build is fast (~5-7s), so blocking is acceptable
# - Full build is slow (~90s), so forking lets user continue editing
#
# Target Directory Auto-Recovery:
# In addition to debounce, watch mode periodically checks if target/ exists.
# If deleted externally (cargo clean, manual rm), triggers immediate rebuild.

# Watch source directories and run checks on file changes
# Parameters: check_type - "full", "test", or "doc"
function watch_mode
    set -l check_type $argv[1]

    # Acquire exclusive lock, killing any existing watch instance
    # One-off commands can run concurrently, but watch modes conflict
    if not acquire_watch_lock
        return 1
    end

    # Check for file watcher (inotifywait on Linux, fswatch on macOS)
    if not command -v inotifywait >/dev/null 2>&1
        and not command -v fswatch >/dev/null 2>&1
        echo "❌ Error: No file watcher found" >&2
        echo "Install with: ./bootstrap.sh" >&2
        echo "Or manually:" >&2
        switch (uname -s)
            case Darwin
                echo "  macOS: brew install fswatch" >&2
            case '*'
                echo "  Ubuntu/Debian: sudo apt install inotify-tools" >&2
                echo "  Fedora/RHEL:   sudo dnf install inotify-tools" >&2
                echo "  Arch:          sudo pacman -S inotify-tools" >&2
        end
        return 1
    end

    # Use shared SRC_DIRS constant from script_lib.fish
    # Make a local copy so we can modify it (add config files)
    set -l watch_dirs $SRC_DIRS

    # Verify directories exist
    for dir in $watch_dirs
        if not test -d $dir
            echo "⚠️  Warning: Directory $dir not found, skipping" >&2
            set -e watch_dirs[(contains -i $dir $watch_dirs)]
        end
    end

    # Add config files to watch list (for detecting config changes mid-session)
    # These are files, not directories, but inotifywait handles both
    for config_file in $CONFIG_FILES_TO_WATCH
        if test -f $config_file
            set watch_dirs $watch_dirs $config_file
        end
    end

    if test (count $watch_dirs) -eq 0
        echo "❌ Error: No valid directories to watch" >&2
        return 1
    end

    # Initialize log file (fresh for each watch session)
    mkdir -p (dirname $CHECK_LOG_FILE)
    echo "["(timestamp)"] Watch mode started" > $CHECK_LOG_FILE

    echo ""
    set_color cyan --bold
    echo "👀 Watch mode activated"
    set_color normal
    echo "Monitoring: "(string join ", " $watch_dirs)
    echo "Log file:   $CHECK_LOG_FILE"
    echo "Press Ctrl+C to stop"
    echo ""

    # Check if config files changed (cleans target if needed)
    check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

    # Validate toolchain BEFORE entering watch loop
    echo "🔧 Validating toolchain..."
    ensure_toolchain_installed
    set -l toolchain_status $status
    if test $toolchain_status -eq 1
        echo ""
        echo "❌ Toolchain validation failed"
        return 1
    end

    # Run initial check
    echo ""
    echo "🚀 Running initial checks..."
    echo ""
    run_checks_for_type $check_type
    set -l initial_result $status

    echo ""
    set_color cyan
    log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
    set_color normal
    echo ""

    # ══════════════════════════════════════════════════════════════════════════
    # Main Watch Loop with Sliding Window Debounce
    # ══════════════════════════════════════════════════════════════════════════
    # See header comments for detailed algorithm explanation and ASCII diagram.
    #
    # Summary:
    #   1. Wait for FIRST event (no timeout - blocks forever)
    #   2. Start sliding window: wait DEBOUNCE_WINDOW_SECS for quiet
    #   3. If new event arrives: reset window (loop back to step 2)
    #   4. If timeout expires: quiet period detected, run checks
    #   5. Go back to step 1
    # ══════════════════════════════════════════════════════════════════════════

    while true
        # ──────────────────────────────────────────────────────────────────────
        # PHASE 1: Wait for first event (blocks forever until file changes)
        # ──────────────────────────────────────────────────────────────────────
        # Use TARGET_CHECK_INTERVAL_SECS timeout to periodically check if
        # target/ directory was deleted externally (cargo clean, rm -rf, etc.)

        wait_for_file_changes $TARGET_CHECK_INTERVAL_SECS $watch_dirs
        set -l wait_status $status

        # Check if target/ directory is missing (regardless of event or timeout)
        # This handles external deletions (cargo clean, manual rm -rf target/, etc.)
        if not test -d "$CHECK_TARGET_DIR"
            echo ""
            set_color yellow
            echo "["(timestamp)"] 📁 target/ missing, triggering rebuild..."
            set_color normal
            echo ""

            run_checks_for_type $check_type
            set -l result $status

            # Handle recoverable error (status 2)
            if test $result -eq 2
                cleanup_for_recovery (dirs_for_check_type $check_type)
            end

            echo ""
            set_color cyan
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
            set_color normal
            echo ""
            continue
        end

        # If timeout (status 2) with target/ present, just loop back to keep watching
        if test $wait_status -eq 2
            continue
        end

        # ──────────────────────────────────────────────────────────────────────
        # PHASE 2: Sliding window - wait for "quiet period" before running
        # ──────────────────────────────────────────────────────────────────────
        # Each new event resets the window. Only when DEBOUNCE_WINDOW_SECS pass
        # with NO new events do we proceed to run checks.

        echo ""
        set_color brblack
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📝 Change detected, waiting for quiet ("$DEBOUNCE_WINDOW_SECS"s window)..."
        set_color normal

        while true
            # Record window start time for remaining time calculation
            set -l window_start (date +%s.%N)

            wait_for_file_changes $DEBOUNCE_WINDOW_SECS $watch_dirs
            set -l debounce_status $status

            if test $debounce_status -eq 2
                # Timeout! No events for DEBOUNCE_WINDOW_SECS seconds = quiet period
                break
            else if test $debounce_status -eq 0
                # Another event arrived - calculate how much time was remaining
                set -l now (date +%s.%N)
                set -l elapsed (math "$now - $window_start")
                set -l remaining (math "$DEBOUNCE_WINDOW_SECS - $elapsed")
                # Format remaining time (round to 1 decimal)
                set -l remaining_str (math --scale=1 "$remaining")

                set_color brblack
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📝 Another change, resetting window... (was "$remaining_str"s remaining)"
                set_color normal
                continue
            else
                # Error (status 1) - break out and proceed
                break
            end
        end

        # ──────────────────────────────────────────────────────────────────────
        # PHASE 3: Run checks (quiet period detected)
        # ──────────────────────────────────────────────────────────────────────

        # Check if config files changed (cleans target if needed)
        check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH

        echo ""
        set_color yellow
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔄 Quiet period reached, running checks..."
        set_color normal
        echo ""

        run_checks_for_type $check_type
        set -l result $status

        # Handle recoverable error (status 2)
        if test $result -eq 2
            cleanup_for_recovery (dirs_for_check_type $check_type)
            # Note: cleanup_for_recovery will trigger another check, continuing naturally
        end

        echo ""
        set_color cyan
        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 👀 Watching for changes..."
        set_color normal
        echo ""
    end
end

# Helper function to run checks based on type
# Parameters: check_type - "full", "test", or "doc"
# Returns: 0 = success, 1 = failure, 2 = recoverable error (triggers cleanup in watch loop)
function run_checks_for_type
    set -l check_type $argv[1]

    switch $check_type
        case "full"
            # Full checks: all three
            run_watch_checks
            set -l result $status

            if test $result -eq 2
                # Recoverable error - let caller (watch loop) handle cleanup
                return 2
            end

            if test $result -eq 0
                sync_docs_to_serving full
                echo ""
                set_color green --bold
                echo "["(timestamp)"] ✅ All checks passed!"
                set_color normal
                send_system_notification "Watch: All Passed ✅" "Tests, doctests, and docs passed" "success" $NOTIFICATION_EXPIRE_MS
            else
                # Notify on failure in watch mode
                send_system_notification "Watch: Checks Failed ❌" "One or more checks failed" "critical" $NOTIFICATION_EXPIRE_MS
            end
            return $result

        case "test"
            # Test checks: cargo test + doctests only
            run_check_with_recovery check_cargo_test "tests"
            if test $status -eq 2
                return 2  # Recoverable error
            end
            if test $status -ne 0
                send_system_notification "Watch: Tests Failed ❌" "cargo test failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            run_check_with_recovery check_doctests "doctests"
            if test $status -eq 2
                return 2  # Recoverable error
            end
            if test $status -ne 0
                send_system_notification "Watch: Doctests Failed ❌" "doctests failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            echo ""
            set_color green --bold
            echo "["(timestamp)"] ✅ All test checks passed!"
            set_color normal
            send_system_notification "Watch: Tests Passed ✅" "All tests and doctests passed" "success" $NOTIFICATION_EXPIRE_MS
            return 0

        case "doc"
            # Doc checks: quick build BLOCKING, full build FORKED to background
            # Architecture: Each build has its own staging directory, both sync to shared serving dir.
            #
            # Why quick blocks but full forks?
            # - Cargo uses a global package cache lock (~/.cargo/.package-cache)
            # - Running both simultaneously causes "Blocking waiting for file lock" messages
            # - Quick build targets only r3bl_tui (~3-5s), so blocking is acceptable
            # - Full build is slow (~90s), so forking lets user continue editing
            #
            # Build flow:
            # 1. Run quick build (blocking) → staging-quick → sync to serving → notify
            # 2. Check for changes during build (catch-up if needed)
            # 3. Fork full build → staging-full → sync to serving → patch if needed → notify
            #
            # Catch-up mechanism (quick build):
            # While the quick build runs (~3-5s), inotifywait isn't watching for changes.
            # If the user saves a file during this "blind spot", the change would be lost.
            # After the quick build, we use has_source_changes_since to detect changes.
            #
            # Catch-up mechanism (full build):
            # The full build (~90s) also has a blind spot. After syncing dep docs,
            # run_full_doc_build_task checks for changes and patches with quick docs if needed.

            # Step 0: Format rustdoc comments on changed files before building docs.
            # Runs before epoch capture so fmt changes don't trigger catch-up rebuilds.
            run_rustdoc_fmt >/dev/null 2>&1

            # Step 1: Quick build (BLOCKING - targets only r3bl_tui for fast feedback)
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔨 Quick build starting (r3bl_tui only)..."

            # Capture build start time for catch-up detection
            set -l build_start_epoch (date +%s)

            # Use extracted function for quick build + sync
            if build_and_sync_quick_docs $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📄 Quick build done!"
                log_and_print $CHECK_LOG_FILE "    📖 Read the docs at: file://$CHECK_TARGET_DIR/doc/r3bl_tui/index.html"
                log_and_print $CHECK_LOG_FILE ""
                send_system_notification "Watch: Quick Docs Ready ⚡" "r3bl_tui done w/ broken dep links - full build starting" "success" $NOTIFICATION_EXPIRE_MS

                # Step 1.5: Catch-up check - did any source files change during the build?
                # Uses has_source_changes_since from script_lib.fish (checks SRC_DIRS)
                if has_source_changes_since $build_start_epoch
                    log_and_print $CHECK_LOG_FILE "["(timestamp)"] ⚡ Files changed during build, catching up..."
                    if build_and_sync_quick_docs $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR
                        log_and_print $CHECK_LOG_FILE "["(timestamp)"] 📄 Catch-up build done!"
                    else
                        log_and_print $CHECK_LOG_FILE "["(timestamp)"] ⚠️ Catch-up build failed (non-fatal)"
                    end
                end
            else
                log_and_print $CHECK_LOG_FILE "["(timestamp)"] ❌ Quick build failed!"
                send_system_notification "Watch: Quick Doc Build Failed ❌" "cargo doc -p r3bl_tui failed" "critical" $NOTIFICATION_EXPIRE_MS
                return 1
            end

            # Step 2: Full build (FORKED - runs in background while user continues editing)
            # Uses run_full_doc_build_task from script_lib.fish which handles:
            # - Building full docs
            # - Syncing to serving (deps are always valid)
            # - Catch-up check for changes during full build
            # - Patching with quick docs if changes detected
            # - Desktop notifications
            log_and_print $CHECK_LOG_FILE "["(timestamp)"] 🔀 Forking full build to background..."

            fish -c "
                cd $PWD
                source script_lib.fish
                run_full_doc_build_task \
                    $CHECK_TARGET_DIR_DOC_STAGING_FULL \
                    $CHECK_TARGET_DIR_DOC_STAGING_QUICK \
                    $CHECK_TARGET_DIR \
                    $CHECK_LOG_FILE \
                    $NOTIFICATION_EXPIRE_MS
            " &

            # Return immediately - quick build done, full build running in background
            return 0

        case '*'
            echo "❌ Unknown check type: $check_type" >&2
            return 1
    end
end
