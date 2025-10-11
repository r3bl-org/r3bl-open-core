#!/usr/bin/env fish

# Rust Toolchain Sync Script
#
# Purpose: Syncs Rust environment to match the rust-toolchain.toml file
#          This is the opposite of the update script - it respects what's in the TOML
#          rather than updating it.
#
# Use Case: When rust-toolchain.toml changes (git checkout, manual edit, etc.)
#           and you need to install the specified toolchain with all components.
#
# Strategy:
# - Read channel from rust-toolchain.toml
# - Remove all toolchains except stable (safety) and the target nightly
# - Install the target nightly toolchain
# - Install rust-analyzer and rust-src components (required by VSCode, RustRover, cargo, and serena MCP server)
#
# This script is designed to be run manually or when rust-toolchain.toml changes.

# ============================================================================
# Global Variables
# ============================================================================

set -g LOG_FILE /home/nazmul/Downloads/rust-toolchain-sync-to-toml.log
set -g PROJECT_DIR /home/nazmul/github/r3bl-open-core
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

function read_toolchain_from_toml
    log_message "Reading toolchain from rust-toolchain.toml..."

    # Extract the channel value from the TOML file
    # Looks for: channel = "nightly-YYYY-MM-DD"
    set -l channel_line (grep '^channel = ' $TOOLCHAIN_FILE)

    if test -z "$channel_line"
        log_message "ERROR: No channel entry found in rust-toolchain.toml"
        return 1
    end

    # Extract the value between quotes
    set -g target_toolchain (echo $channel_line | sed -n 's/.*channel = "\([^"]*\)".*/\1/p')

    if test -z "$target_toolchain"
        log_message "ERROR: Failed to parse channel value from rust-toolchain.toml"
        return 1
    end

    log_message "Target toolchain from TOML: $target_toolchain"
    return 0
end

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

function cleanup_old_toolchains
    # Get disk usage before cleanup
    log_message "Checking disk usage before cleanup..."
    set -l before_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size before cleanup: $before_size"

    # List all currently installed toolchains
    log_message "Currently installed toolchains:"
    set -l all_toolchains (rustup toolchain list | cut -d' ' -f1)
    for toolchain in $all_toolchains
        log_message "  - $toolchain"
    end

    # Cleanup old toolchains - keep only stable and our target nightly
    log_message "Starting aggressive toolchain cleanup..."
    set -l removed_count 0

    for toolchain in $all_toolchains
        # Keep stable toolchains
        if string match -q "stable-*" $toolchain
            log_message "  KEEPING: $toolchain (stable)"
            continue
        end

        # Keep our target nightly
        if string match -q "$target_toolchain*" $toolchain
            log_message "  KEEPING: $toolchain (target from rust-toolchain.toml)"
            continue
        end

        # Remove everything else (old nightlies, generic nightly, etc.)
        log_message "  REMOVING: $toolchain"
        if rustup toolchain uninstall $toolchain 2>&1 | tee -a $LOG_FILE
            set removed_count (math $removed_count + 1)
            log_message "    ✅ Successfully removed $toolchain"
        else
            log_message "    ❌ Failed to remove $toolchain"
        end
    end

    log_message "Removed $removed_count old toolchain(s)"

    # Get disk usage after cleanup
    log_message "Checking disk usage after cleanup..."
    set -l after_size (du -sh ~/.rustup/toolchains 2>/dev/null | cut -f1)
    log_message "Toolchains directory size after cleanup: $after_size"

    return 0
end

# ============================================================================
# Main Function
# ============================================================================

function main
    # Initialize
    log_message "=== Rust Toolchain Sync Started at "(date)" ==="

    # Execute workflow
    validate_prerequisites
    or return 1

    read_toolchain_from_toml
    or return 1

    show_current_state

    install_target_toolchain
    or return 1

    install_rust_analyzer_component
    or return 1

    install_additional_components

    cleanup_old_toolchains

    verify_final_state

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
