#!/usr/bin/env fish

# Shared Test Helper Functions
#
# Usage: source /scripts/tests/lib/test-helpers.fish
#
# Provides assertion functions and test state management for all E2E tests.

# ============================================================================
# Test State
# ============================================================================

set -g tests_passed 0
set -g tests_failed 0
set -g failed_tests

# ============================================================================
# Network Isolation Helpers
# ============================================================================
# Some commands iterate over $ALL_HOSTNAMES and SSH to other hosts.
# These helpers isolate such commands so they only operate locally in tests.
#
# Commands that use ALL_HOSTNAMES (as of Jan 2026):
#   - env-save.fish (SSHs to sync env to other hosts)
#   - github-save.fish (SSHs to sync GitHub config)
#   - ssh-copy-folders-to-others.fish (SSHs to copy folders)
#   - vscode/sync.fish (SSHs to sync VSCode settings)
#   - nfs-show-shares-on-machines.fish (SSHs to show NFS shares)
#
# Usage:
#   isolate_from_network
#   <run network-dependent command>
#   restore_network_access

function isolate_from_network -d "Prevent commands from SSHing to LAN hosts"
    # Save original values
    set -g _SAVED_ALL_HOSTNAMES $ALL_HOSTNAMES
    set -g _SAVED_MY_HOSTNAME $MY_HOSTNAME

    # Set hostname to machine's hostname, ALL_HOSTNAMES to only that
    # Commands that iterate over ALL_HOSTNAMES will skip themselves
    set -g MY_HOSTNAME (hostname).local
    set -g ALL_HOSTNAMES $MY_HOSTNAME
end

function restore_network_access -d "Restore ALL_HOSTNAMES after network isolation"
    set -g ALL_HOSTNAMES $_SAVED_ALL_HOSTNAMES
    set -g MY_HOSTNAME $_SAVED_MY_HOSTNAME
end

# ============================================================================
# Assertion Functions
# ============================================================================

function assert_equals
    set expected $argv[1]
    set actual $argv[2]
    set test_name $argv[3]

    if test "$expected" = "$actual"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Expected: '$expected'"
        echo "     Actual:   '$actual'"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_not_equals
    set unexpected $argv[1]
    set actual $argv[2]
    set test_name $argv[3]

    if test "$unexpected" != "$actual"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Should not equal: '$unexpected'"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_not_empty
    set value $argv[1]
    set test_name $argv[2]

    if test -n "$value"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Value is empty"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_true
    set condition $argv[1]
    set test_name $argv[2]

    if eval $condition
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Condition failed: $condition"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_false
    set condition $argv[1]
    set test_name $argv[2]

    if not eval $condition
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Condition should have failed: $condition"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_file_exists
    set file_path $argv[1]
    set test_name $argv[2]

    if test -f "$file_path"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     File not found: $file_path"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_file_not_exists
    set file_path $argv[1]
    set test_name $argv[2]

    if not test -f "$file_path"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     File should not exist: $file_path"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_dir_exists
    set dir_path $argv[1]
    set test_name $argv[2]

    if test -d "$dir_path"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Directory not found: $dir_path"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_file_contains
    set file_path $argv[1]
    set pattern $argv[2]
    set test_name $argv[3]

    if test -f "$file_path"; and grep -q "$pattern" "$file_path"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        if not test -f "$file_path"
            echo "     File not found: $file_path"
        else
            echo "     Pattern not found: '$pattern' in $file_path"
        end
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_output_contains
    set output $argv[1]
    set pattern $argv[2]
    set test_name $argv[3]

    if echo "$output" | grep -q "$pattern"
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Pattern '$pattern' not found in output"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_command_succeeds
    set test_name $argv[1]
    # Remaining args are the command
    set cmd $argv[2..-1]

    if eval $cmd >/dev/null 2>&1
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Command failed: $cmd"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_command_fails
    set test_name $argv[1]
    # Remaining args are the command
    set cmd $argv[2..-1]

    if not eval $cmd >/dev/null 2>&1
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Command should have failed: $cmd"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_function_exists
    set func_name $argv[1]
    set test_name $argv[2]

    if functions -q $func_name
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Function '$func_name' not found"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_var_set
    set var_name $argv[1]
    set test_name $argv[2]

    if set -q $var_name
        set var_value (eval echo \$$var_name)
        echo "  ✅ $test_name = $var_value"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Variable '\$$var_name' not set"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_package_installed
    set pkg_name $argv[1]
    set test_name $argv[2]

    if pkg_is_installed $pkg_name
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Package '$pkg_name' not installed"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

function assert_command_exists
    # Check if a command exists (more portable than package checks)
    # Use this for utilities where package names differ across distros
    # (e.g., wget is 'wget2-wget' on Fedora 43)
    set cmd_name $argv[1]
    set test_name $argv[2]

    if command -q $cmd_name
        echo "  ✅ $test_name"
        set tests_passed (math $tests_passed + 1)
    else
        echo "  ❌ $test_name"
        echo "     Command '$cmd_name' not found"
        set tests_failed (math $tests_failed + 1)
        set -a failed_tests $test_name
    end
end

# ============================================================================
# Test Utilities
# ============================================================================

function print_test_header
    set header $argv[1]
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "$header"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
end

function print_test_section
    set section $argv[1]
    echo ""
    echo "--- $section ---"
end

function print_test_summary
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Test Results: $tests_passed passed, $tests_failed failed"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    if test $tests_failed -gt 0
        echo ""
        echo "Failed tests:"
        for failed in $failed_tests
            echo "  - $failed"
        end
        return 1
    else
        return 0
    end
end

function reset_test_state
    set -g tests_passed 0
    set -g tests_failed 0
    set -g failed_tests
end

# ============================================================================
# Cleanup Utilities
# ============================================================================

function cleanup_test_dirs
    # Clean up common test directories
    for dir in /tmp/backup /tmp/restored-home /tmp/test-repo /tmp/synced-data /tmp/test-home
        if test -d "$dir"
            command rm -rf "$dir"
        end
    end
end

function setup_test_env
    # Set up environment variables for testing
    # This overrides paths to use /tmp/ inside the machine

    # For backup/restore tests
    set -gx LOCAL_BACKUP_PATH /tmp/backup/archives
    set -gx RESTORE_TARGET /tmp/restored-home

    # For individual backup tests
    set -gx SYNCED_DATA_ROOT /tmp/synced-data

    # Create directories
    mkdir -p $LOCAL_BACKUP_PATH
    mkdir -p $RESTORE_TARGET
    mkdir -p $SYNCED_DATA_ROOT
end
