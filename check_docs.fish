# Documentation Build, Synchronization & Background Task Orchestration
#
# ============================================================================
# ARCHITECTURE OVERVIEW & EVENTUAL CONSISTENCY MODEL
# ============================================================================
#
# # Two-Tier Build Architecture (--watch-doc):
# 1. Quick build (~5-7s) [BLOCKING]:
#    - Runs `cargo doc --workspace --no-deps` into $CHECK_TARGET_DIR_DOC_STAGING_QUICK.
#    - Fast feedback for authoring/editing, broken cross-crate external links.
#    - Syncs to shared serving directory $CHECK_TARGET_DIR/doc.
# 2. Full build (~5-90s) [FORKED TO BACKGROUND]:
#    - Runs `cargo doc` into $CHECK_TARGET_DIR_DOC_STAGING_FULL with dep-doc caching.
#    - Resolves and fixes all cross-crate links (tokio, crossterm, serde, etc.).
#    - Checks for source changes during execution (blind spot catch-up).
#    - On catch-up: runs quick build → forks another full build (eventual consistency).
#
# ┌─────────────────────────────────────────────────────────────────┐
# │                     DOC BUILD PIPELINE                          │
# └─────────────────────────────────────────────────────────────────┘
#
#   File change detected
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Quick build (~5-7s) [BLOCKING]              │
#   │ • cargo doc --workspace --no-deps           │
#   │ • Staging: staging-quick/doc/               │
#   │ • Syncs:   rsync -a → target/doc/           │
#   └─────────────────────────────────────────────┘
#       │
#       ├──► Catch-up check (if changes during quick build)
#       │         └──► Quick build → forks Full build
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Full build (~5-90s) [BACKGROUND TASK]       │
#   │ • cargo doc (dep-doc caching)               │
#   │ • Staging: staging-full/doc/                │
#   │ • Syncs:   rsync -a → target/doc/           │
#   │ • Single-instance mutex via PID file        │
#   └─────────────────────────────────────────────┘
#       │
#       └──► Catch-up check (if changes during full build)
#                 │
#                 ▼
#            Quick build (~5-7s) → forks Full build (~5-90s)
#
# # Dep-Doc Caching:
# Full builds check a hash of Cargo.lock + rust-toolchain.toml stored at
# staging-full/.dep-docs-hash. If unchanged, dependency docs are skipped (--no-deps,
# ~5-7s instead of ~90s). The hash is stored in the staging directory so it
# survives serving directory wipes (e.g. by check_config_changed).
#
# # Single-Instance Mutex & Mutual Exclusion:
# Coordinates execution between --watch-doc background tasks and foreground
# `--full`/`--doc` commands via a PID lock file. If a full build is active in the
# background, foreground commands await its completion via wait_for_background_doc_build
# instead of spawning competing cargo doc processes.
#
# # Rust Migration Note (cargo-monitor / build-infra):
# - Centralize doc orchestration in a `DocEngine` struct.
# - Replace PID lock files with `tokio::sync::Mutex<()>` or `Semaphore(1)`.
# - Use `tokio::spawn` for background builds with `broadcast` or `watch` channels
#   for notifying awaiting tasks and the UI.
#
# ============================================================================

# Central cargo doc runner with custom CSS support.
#
# Always sets RUSTDOCFLAGS with absolute path for monospace font CSS.
# Accepts optional --timeout=SECS for builds that need time limits.
# All other arguments pass through to `cargo doc`.
function run_cargo_doc
    set -lx RUSTDOCFLAGS "--extend-css $PWD/docs/rustdoc/custom.css"

    set -l timeout_secs 0
    set -l cargo_args
    for arg in $argv
        if string match -q -- '--timeout=*' $arg
            set timeout_secs (string replace -- '--timeout=' '' $arg)
        else
            set -a cargo_args $arg
        end
    end

    if test "$timeout_secs" -gt 0
        ionice_wrapper timeout --foreground $timeout_secs cargo doc $cargo_args
    else
        ionice_wrapper cargo doc $cargo_args
    end
end

# Check if dependency docs are still valid (no dep changes since last full build).
#
# Parameters:
#   $argv[1]: staging_dir (e.g., $CHECK_TARGET_DIR_DOC_STAGING_FULL)
#
# Returns: 0 if dep docs are current, 1 if they need rebuilding.
function dep_docs_are_current
    set -l staging_dir $argv[1]
    set -l hash_file $staging_dir/.dep-docs-hash
    if not test -f $hash_file
        return 1
    end
    set -l current_hash (cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1)
    set -l stored_hash (cat $hash_file)
    test "$current_hash" = "$stored_hash"
end

# Update dep docs hash after successful full build.
#
# Parameters:
#   $argv[1]: staging_dir (e.g., $CHECK_TARGET_DIR_DOC_STAGING_FULL)
function update_dep_docs_hash
    set -l staging_dir $argv[1]
    cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1 > $staging_dir/.dep-docs-hash
end

# Builds quick docs (workspace-wide) and syncs to serving directory.
#
# Parameters:
#   $argv[1]: Staging directory (e.g., $CHECK_TARGET_DIR_DOC_STAGING_QUICK)
#   $argv[2]: Serving directory (e.g., $CHECK_TARGET_DIR)
#
# Returns: 0 on success, non-zero on failure
function build_and_sync_quick_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    set -lx CARGO_TARGET_DIR $staging_dir
    # Fast mode: --workspace --no-deps (~5-7s)
    # No external crate links - full build will fix them soon
    run_cargo_doc --workspace --no-deps > /dev/null 2>&1
    set -l result $status

    if test $result -eq 0
        mkdir -p "$serving_dir/doc"
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
    end

    return $result
end

# Builds full docs (with dep-doc caching) and syncs to serving directory.
# Used by --watch-doc's forked background process for correct cross-crate links.
#
# Parameters:
#   $argv[1]: Staging directory (e.g., $CHECK_TARGET_DIR_DOC_STAGING_FULL)
#   $argv[2]: Serving directory (e.g., $CHECK_TARGET_DIR)
#
# Returns: 0 on success, non-zero on failure
function build_and_sync_full_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    set -lx CARGO_TARGET_DIR $staging_dir
    if dep_docs_are_current $staging_dir
        run_cargo_doc --no-deps > /dev/null 2>&1
    else
        run_cargo_doc > /dev/null 2>&1
    end
    set -l result $status

    if test $result -eq 0
        # Ensure serving doc directory exists
        mkdir -p "$serving_dir/doc"
        # Sync with -a (archive mode preserves permissions, timestamps)
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
        # Update hash only when deps were actually rebuilt
        if not dep_docs_are_current $staging_dir
            update_dep_docs_hash $staging_dir
        end
    end

    return $result
end

# Atomically sync generated docs from staging to serving directory.
#
# Parameters:
#   $argv[1]: "quick" or "full" - which staging directory to sync from
function sync_docs_to_serving
    set -l build_type $argv[1]
    set -l serving_doc_dir $CHECK_TARGET_DIR/doc

    # Select staging directory based on build type
    set -l staging_doc_dir
    if test "$build_type" = "full"
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_FULL/doc
    else
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_QUICK/doc
    end

    # Ensure serving doc directory exists
    mkdir -p $serving_doc_dir

    # Determine if we should use --delete (only for full builds with orphans)
    set -l delete_flag
    if test "$build_type" = "full"
        if has_orphan_files $staging_doc_dir $serving_doc_dir
            set delete_flag "--delete"
            set_color yellow
            echo "    🧹 Cleaning orphaned doc files (serving > staging)"
            set_color normal
        end
    end

    rsync -a $delete_flag $staging_doc_dir/ $serving_doc_dir/
end

# Check if serving directory has orphan files (more files than staging).
#
# Parameters:
#   $argv[1]: staging doc directory (source of truth)
#   $argv[2]: serving doc directory (may have orphans)
#
# Returns: 0 if orphans detected (serving > staging), 1 otherwise
function has_orphan_files
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    # If serving dir doesn't exist yet, no orphans possible
    if not test -d $serving_dir
        return 1
    end

    # If staging dir doesn't exist, something is wrong - don't delete
    if not test -d $staging_dir
        return 1
    end

    set -l staging_count (find $staging_dir -type f 2>/dev/null | wc -l)
    set -l serving_count (find $serving_dir -type f 2>/dev/null | wc -l)

    test $serving_count -gt $staging_count
end

# Check if a background full doc build is active (with stale PID detection).
#
# Parameters:
#   $argv[1]: Optional PID lock file path (defaults to $CHECK_FULL_DOC_PID_FILE)
#
# Returns: 0 if active, 1 if not active.
function is_background_doc_build_running
    set -l pid_file $argv[1]
    if test -z "$pid_file"
        set pid_file $CHECK_FULL_DOC_PID_FILE
    end

    if test -z "$pid_file" -o ! -f "$pid_file"
        return 1
    end
    set -l bg_pid (cat "$pid_file" 2>/dev/null | string trim)
    if test -n "$bg_pid" && is_process_alive "$bg_pid"
        return 0
    else
        # Stale PID file from killed/crashed process — clean it up
        if test -n "$pid_file"
            command rm -f "$pid_file" 2>/dev/null
        end
        return 1
    end
end

# Wait for an in-progress background full doc build to finish.
#
# Parameters:
#   $argv[1]: Optional PID lock file path (defaults to $CHECK_FULL_DOC_PID_FILE)
#   $argv[2]: Optional timeout in seconds (defaults to $CHECK_TIMEOUT_SECS or 300)
#
# Returns: 0 if background build finished successfully, 1 on timeout/error.
function wait_for_background_doc_build
    set -l pid_file $argv[1]
    if test -z "$pid_file"
        set pid_file $CHECK_FULL_DOC_PID_FILE
    end

    set -l timeout_secs $argv[2]
    if test -z "$timeout_secs"
        set timeout_secs $CHECK_TIMEOUT_SECS
    end
    if test -z "$timeout_secs"
        set timeout_secs 300
    end
    set -l elapsed 0

    while is_background_doc_build_running "$pid_file"
        if test $elapsed -ge $timeout_secs
            if functions -q log_message
                log_message "⚠️ Timed out waiting for background doc build ({$timeout_secs}s)"
            end
            if test -n "$pid_file"
                command rm -f "$pid_file" 2>/dev/null
            end
            return 1
        end

        sleep 1
        set elapsed (math $elapsed + 1)

        # Output progress update every 5 seconds
        if test (math "$elapsed % 5") -eq 0
            set_color yellow
            echo "    ⏳ Waiting for background full doc build ({$elapsed}s / {$timeout_secs}s)..."
            set_color normal
        end
    end
    return 0
end

# Runs the full doc build workflow as a background task.
#
# Pure signature: Accepts all configuration as explicit arguments.
# Has zero dependency on uninitialized ambient global variables.
#
# Parameters:
#   $argv[1]: Full build staging directory ($CHECK_TARGET_DIR_DOC_STAGING_FULL)
#   $argv[2]: Quick build staging directory ($CHECK_TARGET_DIR_DOC_STAGING_QUICK)
#   $argv[3]: Serving directory ($CHECK_TARGET_DIR)
#   $argv[4]: Log file path ($CHECK_LOG_FILE)
#   $argv[5]: PID lock file path ($CHECK_FULL_DOC_PID_FILE)
#   $argv[6]: Notification expire time in milliseconds ($NOTIFICATION_EXPIRE_MS)
#   $argv[7]: Workspace name for notifications ($WORKSPACE_NAME)
function run_full_doc_build_task
    set -l staging_full     $argv[1]
    set -l staging_quick    $argv[2]
    set -l serving_dir      $argv[3]
    set -l log_file         $argv[4]
    set -l pid_file         $argv[5]
    set -l notify_expire_ms $argv[6]
    set -l workspace_name   $argv[7]

    # Single-instance guard: if a background full doc build is ALREADY running,
    # do NOT spawn another overlapping background task.
    if is_background_doc_build_running "$pid_file"
        log_and_print $log_file "["(timestamp)"] [bg] ⏩ Background full doc build already in progress, skipping duplicate task."
        return 0
    end

    # Write current PID to lock file
    if test -n "$pid_file"
        echo %self > "$pid_file"
        # Ensure lock file is cleaned up on exit or signal
        trap "command rm -f '$pid_file' 2>/dev/null" EXIT INT TERM
    end

    # Capture build start time for catch-up detection
    set -l full_build_start (date +%s)

    log_and_print $log_file "["(timestamp)"] [bg] 🔨 Full build starting (with deps)..."

    # Build full docs
    if build_and_sync_full_docs $staging_full $serving_dir
        log_and_print $log_file "["(timestamp)"] [bg] ✅ Full build done, synced to serving"

        # Catch-up check: did source files change during our ~90s build?
        if has_source_changes_since $full_build_start
            log_and_print $log_file "["(timestamp)"] [bg] ⚡ Changes during build, running catch-up..."

            # Run quick build for fast feedback (broken links OK - full build will fix)
            if build_and_sync_quick_docs $staging_quick $serving_dir
                log_and_print $log_file "["(timestamp)"] [bg] ✅ Quick catch-up complete!"
                log_and_print $log_file "["(timestamp)"] [bg] 🔀 Forking another full build to fix links..."

                # Remove current PID before forking follow-up full build
                if test -n "$pid_file"
                    command rm -f "$pid_file" 2>/dev/null
                end

                # Fork another full build to eventually fix the broken links
                fish -c "
                    cd $PWD
                    source script_lib.fish
                    source check_constants.fish
                    source check_docs.fish
                    run_full_doc_build_task \
                        '$staging_full' \
                        '$staging_quick' \
                        '$serving_dir' \
                        '$log_file' \
                        '$pid_file' \
                        '$notify_expire_ms' \
                        '$workspace_name'
                " &

                send_system_notification "Watch ($workspace_name): Quick Docs Ready ⚡" "Workspace ready (no ext crate links) - full build starting..." "success" $notify_expire_ms
            else
                log_and_print $log_file "["(timestamp)"] [bg] ⚠️ Quick catch-up failed (full docs still available)"
                send_system_notification "Watch ($workspace_name): Full Docs Ready ✅" "Full build done, but latest edits have errors ❌" "normal" $notify_expire_ms
            end
        else
            # No changes during build - full docs are already up to date
            send_system_notification "Watch ($workspace_name): Full Docs Built ✅" "All documentation including dependencies built" "success" $notify_expire_ms
        end
    else
        log_and_print $log_file "["(timestamp)"] [bg] ❌ Full build failed!"
        send_system_notification "Watch ($workspace_name): Full Doc Build Failed ❌" "cargo doc failed" "critical" $notify_expire_ms
    end

    if test -n "$pid_file"
        command rm -f "$pid_file" 2>/dev/null
    end
end
