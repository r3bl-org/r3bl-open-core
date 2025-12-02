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
# - Install rust-analyzer and rust-src components (required by VSCode, RustRover, cargo, and serena MCP server)
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
# Helper Functions
# ============================================================================

function log_message
    set -l message $argv[1]
    echo $message | tee -a $LOG_FILE
end

function log_command_output
    set -l description $argv[1]
    log_message $description
    $argv[2..] 2>&1 | tee -a $LOG_FILE
    return $pipestatus[1]
end

function validate_prerequisites
    log_message "Validating prerequisites..."

    # Check if project directory exists
    if not test -d $PROJECT_DIR
        log_message "ERROR: Project directory not found: $PROJECT_DIR"
        return 1
    end

    # Check if rust-toolchain.toml exists
    if not test -f $TOOLCHAIN_FILE
        log_message "ERROR: rust-toolchain.toml not found: $TOOLCHAIN_FILE"
        return 1
    end

    log_message "✅ Prerequisites validated successfully"
    return 0
end

# ============================================================================
# State Management Functions
# ============================================================================

function show_current_state
    log_message "Changing to project directory: $PROJECT_DIR"
    cd $PROJECT_DIR

    if not log_command_output "Current toolchain information:" rustup show
        log_message "WARNING: Failed to get current toolchain information"
    end
end

function verify_final_state
    if not log_command_output "Final installed toolchains:" rustup toolchain list
        log_message "WARNING: Failed to list final toolchains"
    end

    if not log_command_output "Verifying project toolchain:" rustup show
        log_message "WARNING: Failed to verify project toolchain"
    end
end

# ============================================================================
# Core Logic Functions
# ============================================================================

function install_target_toolchain
    if not log_command_output "Installing toolchain $target_toolchain (if not already installed)..." rustup toolchain install $target_toolchain
        log_message "❌ Failed to install $target_toolchain"
        return 1
    end

    log_message "✅ Successfully installed/verified $target_toolchain"
    return 0
end

function install_rust_analyzer_component
    log_message "Installing rust-analyzer component for $target_toolchain..."
    if not log_command_output "Adding rust-analyzer component..." rustup component add rust-analyzer --toolchain $target_toolchain
        log_message "❌ Failed to install rust-analyzer component"
        return 1
    end

    log_message "✅ Successfully installed rust-analyzer component"
    return 0
end

function install_additional_components
    log_message "Installing additional components for $target_toolchain..."

    # Install rust-src for better IDE support
    if log_command_output "Adding rust-src component..." rustup component add rust-src --toolchain $target_toolchain
        log_message "✅ Successfully installed rust-src component"
    else
        log_message "⚠️  Failed to install rust-src component (continuing anyway)"
    end

    return 0
end

function purge_all_toolchains
    # Clear rustup caches to prevent stale download/temp file issues
    log_message "Clearing rustup download and temp caches..."
    rm -rf ~/.rustup/downloads/
    rm -rf ~/.rustup/tmp/
    log_message "✅ Rustup caches cleared"
    log_message ""

    # Get disk usage before purge
    log_message "Checking disk usage before purge..."
    set -l before_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size before purge: $before_size"

    # List all currently installed toolchains
    log_message "Currently installed toolchains:"
    set -l all_toolchains (rustup toolchain list | cut -d' ' -f1)
    for toolchain in $all_toolchains
        log_message "  - $toolchain"
    end

    # Nuclear cleanup - remove ALL toolchains (stable and nightly)
    log_message "Starting nuclear toolchain purge (removing everything)..."
    set -l removed_count 0

    for toolchain in $all_toolchains
        log_message "  REMOVING: $toolchain"
        if rustup toolchain uninstall $toolchain 2>&1 | tee -a $LOG_FILE
            set removed_count (math $removed_count + 1)
            log_message "    ✅ Successfully removed $toolchain"
        else
            log_message "    ❌ Failed to remove $toolchain"
        end
    end

    log_message "Removed $removed_count toolchain(s)"

    # Get disk usage after purge
    log_message "Checking disk usage after purge..."
    set -l after_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size after purge: $after_size"

    return 0
end

function install_stable_toolchain
    log_message "Installing fresh stable toolchain..."
    if not log_command_output "Installing stable toolchain..." rustup toolchain install stable
        log_message "❌ Failed to install stable toolchain"
        return 1
    end

    log_message "✅ Successfully installed stable toolchain"
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
    log_message "=== Rust Toolchain Sync Started at "(date)" ==="

    # Output log file location to stdout for user visibility
    echo ""
    echo "📋 Detailed log: $LOG_FILE"
    echo ""

    # Execute workflow
    validate_prerequisites
    or begin
        release_toolchain_lock
        return 1
    end

    # Read toolchain from TOML using script_lib function
    log_message "Reading toolchain from rust-toolchain.toml..."
    set -g target_toolchain (read_toolchain_from_toml)
    if test $status -ne 0
        log_message "ERROR: Failed to read toolchain from rust-toolchain.toml"
        release_toolchain_lock
        return 1
    end
    log_message "Target toolchain from TOML: $target_toolchain"

    show_current_state

    # Phase 1: Nuclear purge - remove all toolchains
    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 1: Purge All Toolchains"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    purge_all_toolchains

    # Phase 2: Install fresh stable toolchain
    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 2: Install Fresh Stable Toolchain"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    install_stable_toolchain
    or begin
        release_toolchain_lock
        return 1
    end

    # Phase 3: Install target nightly toolchain
    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 3: Install Target Nightly Toolchain"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    install_target_toolchain
    or begin
        release_toolchain_lock
        return 1
    end

    # Phase 4: Install components
    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Phase 4: Install Components"
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    install_rust_analyzer_component
    or begin
        release_toolchain_lock
        return 1
    end

    install_additional_components

    verify_final_state

    # Release lock after successful completion
    release_toolchain_lock

    # Validate installation using quick validation
    log_message ""
    log_message "═══════════════════════════════════════════════════════"
    log_message "Validating final installation (quick check)..."
    log_message "═══════════════════════════════════════════════════════"
    log_message ""

    if fish ./rust-toolchain-validate.fish quick 2>&1 | tee -a $LOG_FILE
        log_message ""
        log_message "✅ Validation passed - toolchain fully operational"
    else
        set -l validation_code $status
        log_message ""
        log_message "⚠️  Validation returned code $validation_code - check details above"
    end
    log_message ""

    # Cleanup
    log_message "=== Rust Toolchain Sync Completed at "(date)" ==="
    log_message ""
    log_message "✨ Your Rust environment is now synced to rust-toolchain.toml"
    log_message "   Toolchain: $target_toolchain"
    log_message "   Components: rust-analyzer, rust-src"
    log_message ""
    log_message "💡 Next steps:"
    log_message "   - Restart your IDE/editor to pick up the new toolchain"
    log_message "   - Run 'cargo check' to verify everything works"
    log_message ""

    return 0
end

# ============================================================================
# Script Entry Point
# ============================================================================

# Run main function and exit with its status
main
exit $status
