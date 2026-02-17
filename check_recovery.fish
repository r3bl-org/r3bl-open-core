# Cleanup, Recovery Utilities & Logging
#
# Provides target directory cleanup, recovery from ICE/stale artifacts,
# check-type-to-directory mapping, and dual-output logging (terminal + file).

# Maps check type to the target directories it uses.
# Used by watch loop to pass targeted dirs to cleanup_for_recovery.
# Parameters:
#   $argv[1]: "full", "test", "doc", or anything else (defaults to all dirs)
# Output: One directory path per line (Fish command substitution splits on newlines)
function dirs_for_check_type
    switch $argv[1]
        case "full"
            printf '%s\n' $CHECK_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_FULL
        case "test"
            printf '%s\n' $CHECK_TARGET_DIR
        case "doc"
            printf '%s\n' $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR_DOC_STAGING_FULL
        case '*'
            printf '%s\n' $CHECK_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR_DOC_STAGING_FULL
    end
end

# Helper function to clean target folder
# Removes build artifacts and caches to ensure a clean rebuild.
# This is important because various parts of the cache (incremental, metadata, etc.)
# can become corrupted and cause compiler panics or other mysterious failures.
#
# Parameters (optional):
#   $argv: Specific directories to clean. If none provided, cleans all 3 target dirs.
function cleanup_target_folder
    echo "🧹 Cleaning target folders..."
    set -l dirs_to_clean
    if test (count $argv) -gt 0
        set dirs_to_clean $argv
    else
        set dirs_to_clean $CHECK_TARGET_DIR $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR_DOC_STAGING_FULL
    end
    for dir in $dirs_to_clean
        if test -d "$dir"
            command rm -rf "$dir"
        end
    end
end

# Helper function to run cleanup for recoverable errors (ICE, stale artifacts, etc.)
# Removes target folders and any ICE dump files, logs events for debugging.
function cleanup_for_recovery
    log_message "🧹 Running cache cleanup..."

    # Remove ICE dump files and log their names for debugging
    set -l ice_files (find . -name "rustc-ice-*.txt" 2>/dev/null)
    if test (count $ice_files) -gt 0
        log_message "🗑️  Removing "(count $ice_files)" ICE dump file(s):"
        for ice_file in $ice_files
            log_message "    - $ice_file"
        end
        command rm -f rustc-ice-*.txt
    end

    # Remove target folders (pass through optional dir arguments for targeted cleanup)
    cleanup_target_folder $argv

    log_message "✨ Cleanup complete. Retrying checks..."
    echo ""
end

# Helper function to log messages to both terminal and log file.
# Ensures log file directory exists and appends to CHECK_LOG_FILE.
# Falls back to echo-only if CHECK_LOG_FILE is not set.
function log_message
    set -l message $argv

    # Always echo to terminal
    echo $message

    # Also append to log file if CHECK_LOG_FILE is set
    if set -q CHECK_LOG_FILE; and test -n "$CHECK_LOG_FILE"
        mkdir -p (dirname $CHECK_LOG_FILE)
        echo "["(timestamp)"] $message" >> $CHECK_LOG_FILE
    end
end

# Prints a hint suggesting rust-toolchain-update.fish when a check fails even after retry.
# This helps diagnose persistent failures caused by toolchain ecosystem issues
# (e.g., linker incompatibilities) that cache cleanup alone cannot fix.
#
# Parameters:
#   $argv[1]: The exit status from the retried check
function hint_toolchain_update_on_persistent_failure
    set -l check_status $argv[1]
    if test $check_status -ne 0
        echo ""
        set_color yellow
        echo "💡 Persistent failure after cache cleanup."
        echo "   This may be a toolchain or linker incompatibility."
        echo "   Try running: ./rust-toolchain-update.fish"
        set_color normal
    end
end
