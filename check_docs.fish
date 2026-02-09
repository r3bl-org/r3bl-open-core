# Doc Sync Functions
#
# Handles syncing generated documentation from staging directories to the
# serving directory, and detecting orphaned doc files from renamed/deleted sources.

# Atomically sync generated docs from staging to serving directory.
#
# Delete behavior (--delete flag):
# - Quick builds: NEVER use --delete (would wipe dependency docs from full builds)
# - Full builds: Use --delete ONLY when orphan files detected (serving > staging count)
#
# Why orphan detection matters for long-running watch sessions:
# - Renamed/deleted source files leave behind stale .html files in serving dir
# - Without cleanup, these accumulate over days of development
# - File count comparison detects this: if serving has MORE files than staging,
#   those extra files are orphans that should be removed
#
# Parameters:
#   $argv[1]: "quick" or "full" - which staging directory to sync from
function sync_docs_to_serving
    set -l build_type $argv[1]
    set -l serving_doc_dir $CHECK_TARGET_DIR/doc

    # Select staging directory based on build type
    # NOTE: Must declare staging_doc_dir OUTSIDE the if block, otherwise
    # "set -l" creates a variable scoped to the if block that vanishes.
    set -l staging_doc_dir
    if test "$build_type" = "full"
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_FULL/doc
    else
        set staging_doc_dir $CHECK_TARGET_DIR_DOC_STAGING_QUICK/doc
    end

    # Ensure serving doc directory exists
    mkdir -p $serving_doc_dir

    # Determine if we should use --delete (only for full builds with orphans)
    # NOTE: Must NOT initialize to "" - Fish treats that as a 1-element list containing
    # an empty string, which rsync receives as an empty argument causing errors.
    # Leaving uninitialized creates a 0-element list that expands to nothing.
    set -l delete_flag
    if test "$build_type" = "full"
        if has_orphan_files $staging_doc_dir $serving_doc_dir
            set delete_flag "--delete"
            set_color yellow
            echo "    🧹 Cleaning orphaned doc files (serving > staging)"
            set_color normal
        end
    end

    # -a = archive mode (preserves permissions, timestamps)
    # --delete (conditional): removes orphaned files when serving has more than staging
    rsync -a $delete_flag $staging_doc_dir/ $serving_doc_dir/
end

# Check if serving directory has orphan files (more files than staging).
# This indicates stale docs from renamed/deleted source files.
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

    # Count files in each directory (fast - just readdir operations)
    set -l staging_count (find $staging_dir -type f 2>/dev/null | wc -l)
    set -l serving_count (find $serving_dir -type f 2>/dev/null | wc -l)

    # Orphans exist if serving has MORE files than staging
    test $serving_count -gt $staging_count
end
