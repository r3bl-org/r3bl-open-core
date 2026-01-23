#!/usr/bin/env fish

# Import shared toolchain utilities
source script_lib.fish

# Rust Toolchain Sync Script
#
# Purpose: Syncs Rust environment to match the rust-toolchain.toml file
#          This is the opposite of the update script - it respects what's in the TOML
#          rather than updating it.
#
# Use Case: When rust-toolchain.toml changes (git checkout, manual edit, etc.)
#           and you need to install the specified toolchain with all components.
#           Also useful for troubleshooting toolchain corruption - provides a clean slate.
#
# Strategy (Nuclear approach - complete reinstall):
# - Read channel from rust-toolchain.toml
# - Purge ALL existing toolchains (stable and nightly)
# - Install fresh stable toolchain
# - Install the target nightly toolchain from TOML
# - Install rust-analyzer and rust-src components (required by VSCode, RustRover, Claude Code, and cargo)
# - Install x86_64-pc-windows-gnu target for cross-platform verification
# - Update cargo development tools (wild-linker, bacon, flamegraph, etc.)
#
# Concurrency Safety:
# - Uses mkdir (atomic directory creation) for mutual exclusion
# - Atomic lock: check-and-create happens in ONE kernel operation (only one process succeeds)
# - Safe to run alongside other toolchain scripts - they'll queue or abort gracefully
# - Stale lock detection: Automatically removes locks older than 10 minutes (crashed processes)
#
# This script is designed to be run manually or when rust-toolchain.toml changes.

# ============================================================================
# Global Variables
# ============================================================================

set -g LOG_FILE $HOME/Downloads/rust-toolchain-sync-to-toml.log
set -g PROJECT_DIR (pwd)
set -g TOOLCHAIN_FILE $PROJECT_DIR/rust-toolchain.toml
set -g target_toolchain ""

# ============================================================================
# Script-Specific Functions
# ============================================================================
# Note: Common helper functions (logging, validation, state management, component
# installation) are now in script_lib.fish with toolchain_* prefix.
# ============================================================================

function purge_all_toolchains
    # Clear rustup caches to prevent stale download/temp file issues
    toolchain_log "Clearing rustup download and temp caches..."
    command rm -rf ~/.rustup/downloads/
    command rm -rf ~/.rustup/tmp/
    toolchain_log "✅ Rustup caches cleared"
    toolchain_log ""

    # Get disk usage before purge
    toolchain_log "Checking disk usage before purge..."
    set -l before_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    toolchain_log "Toolchains directory size before purge: $before_size"

    # List all currently installed toolchains
    toolchain_log "Currently installed toolchains:"
    set -l all_toolchains (rustup toolchain list | cut -d' ' -f1)
    for toolchain in $all_toolchains
        toolchain_log "  - $toolchain"
    end

    # Nuclear cleanup - remove ALL toolchains (stable and nightly)
    toolchain_log "Starting nuclear toolchain purge (removing everything)..."
    set -l removed_count 0
    set -l failed_toolchains

    for toolchain in $all_toolchains
        toolchain_log "  REMOVING: $toolchain"
        if rustup toolchain uninstall $toolchain 2>&1 | tee -a $LOG_FILE
            set removed_count (math $removed_count + 1)
            toolchain_log "    ✅ Successfully removed $toolchain"
        else
            toolchain_log "    ⚠️  rustup uninstall failed for $toolchain"
            set -a failed_toolchains $toolchain
        end
    end

    # Fallback: directly delete folders for any toolchains that failed to uninstall
    # This handles corrupted toolchains with "Missing manifest" errors
    if test (count $failed_toolchains) -gt 0
        toolchain_log ""
        toolchain_log "🔧 Attempting direct folder cleanup for stubborn toolchains..."
        set -l toolchains_dir "$HOME/.rustup/toolchains"

        for toolchain in $failed_toolchains
            # Try both with and without platform suffix
            for suffix in "" "-x86_64-unknown-linux-gnu" "-aarch64-unknown-linux-gnu"
                set -l folder_path "$toolchains_dir/$toolchain$suffix"
                if test -d "$folder_path"
                    toolchain_log "    Deleting folder: $folder_path"
                    if command rm -rf "$folder_path"
                        set removed_count (math $removed_count + 1)
                        toolchain_log "    ✅ Removed via direct deletion"
                    else
                        toolchain_log "    ❌ Failed to delete folder"
                    end
                end
            end
        end
    end

    # Final safety check: remove any remaining toolchain folders
    # (handles edge cases where toolchains exist on disk but not in rustup list)
    set -l remaining_folders (find ~/.rustup/toolchains -mindepth 1 -maxdepth 1 -type d 2>/dev/null)
    if test (count $remaining_folders) -gt 0
        toolchain_log ""
        toolchain_log "🧹 Cleaning orphaned toolchain folders..."
        for folder in $remaining_folders
            toolchain_log "    Removing orphaned: $folder"
            command rm -rf "$folder" 2>/dev/null
        end
        toolchain_log "    ✅ Orphaned folders cleaned"
    end

    toolchain_log "Removed $removed_count toolchain(s)"

    # Get disk usage after purge
    toolchain_log "Checking disk usage after purge..."
    set -l after_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    toolchain_log "Toolchains directory size after purge: $after_size"

    return 0
end

function install_stable_toolchain
    toolchain_log "Installing fresh stable toolchain..."
    if not toolchain_log_command "Installing stable toolchain..." rustup toolchain install stable
        toolchain_log "❌ Failed to install stable toolchain"
        return 1
    end

    toolchain_log "✅ Successfully installed stable toolchain"
    return 0
end

# ============================================================================
# Main Function
# ============================================================================

function main
    # Truncate log file at start of script so each run gets a fresh log
    echo -n "" > $LOG_FILE

    # Acquire lock before proceeding
    if not acquire_toolchain_lock
        return 1
    end

    # Initialize
    toolchain_log "=== Rust Toolchain Sync Started at "(date)" ==="

    # Output log file location to stdout for user visibility
    echo ""
    echo "📋 Detailed log: $LOG_FILE"
    echo ""

    # Execute workflow
    toolchain_validate_prerequisites
    or begin
        release_toolchain_lock
        return 1
    end

    # Read toolchain from TOML using script_lib function
    toolchain_log "Reading toolchain from rust-toolchain.toml..."
    set -g target_toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        toolchain_log "ERROR: Failed to read toolchain from rust-toolchain.toml"
        release_toolchain_lock
        return 1
    end
    toolchain_log "Target toolchain from TOML: $target_toolchain"

    toolchain_show_current_state

    # Phase 1: Nuclear purge - remove all toolchains
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Phase 1: Purge All Toolchains"
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    purge_all_toolchains

    # Phase 2: Install fresh stable toolchain
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Phase 2: Install Fresh Stable Toolchain"
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    install_stable_toolchain
    or begin
        release_toolchain_lock
        return 1
    end

    # Phase 3: Install target nightly toolchain
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Phase 3: Install Target Nightly Toolchain"
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    toolchain_install_target
    or begin
        release_toolchain_lock
        return 1
    end

    # Phase 4: Install components
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Phase 4: Install Components"
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    toolchain_install_rust_analyzer
    or begin
        release_toolchain_lock
        return 1
    end

    toolchain_install_additional_components

    # Install Windows cross-compilation target for verifying platform-specific code
    install_windows_target

    # Phase 5: Update cargo development tools
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Phase 5: Update Cargo Development Tools"
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    toolchain_log "Updating cargo development tools to latest versions..."
    if fish run.fish update-cargo-tools 2>&1 | tee -a $LOG_FILE
        toolchain_log "✅ Cargo tools updated successfully"
    else
        toolchain_log "⚠️  Cargo tools update had issues (non-critical, continuing)"
    end
    toolchain_log ""

    toolchain_verify_final_state

    # Release lock after successful completion
    release_toolchain_lock

    # Validate installation using quick validation
    toolchain_log ""
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log "Validating final installation (quick check)..."
    toolchain_log "═══════════════════════════════════════════════════════"
    toolchain_log ""

    if fish ./rust-toolchain-validate.fish quick 2>&1 | tee -a $LOG_FILE
        toolchain_log ""
        toolchain_log "✅ Validation passed - toolchain fully operational"
    else
        set -l validation_code $status
        toolchain_log ""
        toolchain_log "⚠️  Validation returned code $validation_code - check details above"
    end
    toolchain_log ""

    # Cleanup
    toolchain_log "=== Rust Toolchain Sync Completed at "(date)" ==="
    toolchain_log ""
    toolchain_log "✨ Your Rust environment is now synced to rust-toolchain.toml"
    toolchain_log "   Toolchain: $target_toolchain"
    toolchain_log "   Components: rust-analyzer, rust-src"
    toolchain_log ""
    toolchain_log "💡 Next steps:"
    toolchain_log "   - Restart your IDE/editor to pick up the new toolchain"
    toolchain_log "   - Run 'cargo check' to verify everything works"
    toolchain_log ""

    return 0
end

# ============================================================================
# Script Entry Point
# ============================================================================

# Run main function and exit with its status
main
exit $status
