#!/usr/bin/env fish

# Rust Toolchain Update Script
#
# Purpose: Automatically updates the Rust toolchain to use a nightly version
#          from 1 month ago, avoiding instability issues with the latest nightly.
#          Also performs aggressive cleanup to save disk space.
#
# Strategy:
# - Keep only: stable-* + nightly-YYYY-MM-DD (month-old)
# - Remove: all other nightly toolchains, including generic "nightly"
#
# This script is designed to run weekly via systemd timer.

# ============================================================================
# Global Variables
# ============================================================================

set -g LOG_FILE /home/nazmul/Downloads/rust-toolchain-update.log
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

function log_message_no_newline
    set -l message $argv[1]
    echo -n $message | tee -a $LOG_FILE
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

function update_toolchain_config
    log_message "Updating rust-toolchain.toml to use $target_toolchain..."

    # Validate the TOML file before updating
    set -l channel_count (grep -c "^channel = " $TOOLCHAIN_FILE)
    if test $channel_count -eq 0
        log_message "WARNING: No uncommented channel entry found in rust-toolchain.toml"
        log_message "This might indicate a malformed TOML file"
    else if test $channel_count -gt 1
        log_message "WARNING: Multiple uncommented channel entries found ($channel_count)"
        log_message "Only the first one will be updated"
    end

    # Only replace the first uncommented channel line (preserves comments)
    sed -i "0,/^channel = .*/s//channel = \"$target_toolchain\"/" $TOOLCHAIN_FILE

    if test $status -eq 0
        log_message "✅ Successfully updated rust-toolchain.toml"
    else
        log_message "❌ Failed to update rust-toolchain.toml"
        return 1
    end

    # Show the updated content
    log_message "Updated rust-toolchain.toml content:"
    cat $TOOLCHAIN_FILE | tee -a $LOG_FILE

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
        if test "$toolchain" = "$target_toolchain"
            log_message "  KEEPING: $toolchain (target month-old nightly)"
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
    log_message "=== Rust Toolchain Update Started at "(date)" ==="

    # Calculate the date from 1 month ago
    set one_month_ago (date -d "1 month ago" "+%Y-%m-%d")
    set -g target_toolchain "nightly-$one_month_ago"

    log_message "Target toolchain: $target_toolchain"

    # Execute workflow
    validate_prerequisites
    or return 1

    show_current_state

    update_toolchain_config
    or return 1

    install_target_toolchain
    or return 1

    cleanup_old_toolchains

    verify_final_state

    # Cleanup
    log_message "=== Rust Toolchain Update Completed at "(date)" ==="
    log_message ""

    return 0
end

# ============================================================================
# Script Entry Point
# ============================================================================

# Run main function and exit with its status
main
exit $status