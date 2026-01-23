#!/usr/bin/env fish

# Test runner for systemd-nspawn machines
#
# Usage:
#   ./run.fish                              # All tests, all distros (parallel)
#   ./run.fish unit                         # Unit tests only (parallel)
#   ./run.fish e2e                          # E2E tests only (parallel)
#   ./run.fish <distro>                     # All tests, one distro
#   ./run.fish unit <distro>                # Unit tests, one distro
#   ./run.fish <distro> unit/test-foo.fish  # Single test file
#   ./run.fish start                        # Start all machines (no tests)
#   ./run.fish start <distro>                # Start one machine (no tests)
#   ./run.fish reboot                       # Reboot all machines (abort tests)
#   ./run.fish reboot <distro>              # Reboot one machine (abort tests)
#   ./run.fish stop                         # Stop all machines
#   ./run.fish stop <distro>                # Stop one machine
#   ./run.fish nuke <distro>                # Force reset: delete + recreate from zygote
#   ./run.fish shell <distro>               # Interactive shell (boots if needed)
#   ./run.fish ps                           # Show processes in all machines
#   ./run.fish ps <distro>                  # Show processes in one machine
#   ./run.fish logs                         # Show last 20 lines from all logs
#   ./run.fish logs <distro>                # Show last 20 lines from one log
#   ./run.fish list                         # List all available tests
#   ./run.fish list unit                    # List unit tests only
#   ./run.fish list e2e                     # List e2e tests only
#
# Setup: Run ./setup.fish first to create machines and zygotes.
#
# Shell: Boots machine if not running, keeps running after exit.
#        Use ./run.fish stop to stop machines (or ./teardown.fish to delete).
#
# Zygotes: E2E tests restore from zygote before each test (clean state).
#          Unit tests run without restoration (pure functions).

set tests_dir (dirname (status filename))
source $tests_dir/lib/ensure-prereqs.fish
source $tests_dir/lib/nspawn.fish

# ============================================================================
# Usage
# ============================================================================

function show_usage
    echo "Usage: ./run.fish [category] [distro] [test-path]"
    echo ""
    echo "Run tests:"
    echo "  ./run.fish                              # All tests, all distros (parallel)"
    echo "  ./run.fish unit                         # Unit tests only (parallel)"
    echo "  ./run.fish e2e                          # E2E tests only (parallel)"
    echo "  ./run.fish <distro>                     # All tests, one distro"
    echo "  ./run.fish unit <distro>                # Unit tests, one distro"
    echo "  ./run.fish <distro> unit/test-foo.fish  # Single test file"
    echo ""
    echo "Machine control:"
    echo "  ./run.fish start                        # Start all machines (no tests)"
    echo "  ./run.fish start <distro>                # Start one machine (no tests)"
    echo "  ./run.fish reboot                       # Reboot all machines (abort tests)"
    echo "  ./run.fish reboot <distro>              # Reboot one machine (abort tests)"
    echo "  ./run.fish stop                         # Stop all machines"
    echo "  ./run.fish stop <distro>                # Stop one machine"
    echo "  ./run.fish nuke <distro>                # Force reset: delete + recreate from zygote"
    echo "  ./run.fish shell <distro>               # Interactive shell in machine"
    echo ""
    echo "Debug:"
    echo "  ./run.fish ps                           # Show processes in all machines"
    echo "  ./run.fish ps <distro>                  # Show processes in one machine"
    echo "  ./run.fish logs                         # Show last 20 lines from all logs"
    echo "  ./run.fish logs <distro>                # Show last 20 lines from one log"
    echo ""
    echo "List tests:"
    echo "  ./run.fish list                         # List all available tests"
    echo "  ./run.fish list unit                    # List unit tests only"
    echo "  ./run.fish list e2e                     # List e2e tests only"
    echo ""
    echo "Distros: ubuntu, fedora, arch"
    echo "Categories: unit, e2e"
    echo ""
    echo "Setup: Run ./setup.fish first to create machines and zygotes."
    echo ""
    echo "Shell behavior:"
    echo "  - Boots machine if not already running"
    echo "  - Machine keeps running after shell exit"
    echo "  - Use ./run.fish stop to stop (or ./teardown.fish to delete)"
    echo ""
    echo "E2E tests restore from zygote before each test (clean state)."
    echo "Unit tests run without restoration (pure functions)."
    echo ""
    echo "Log files: $LOG_DIR/test-<distro>.log"
end

# ============================================================================
# Helper Functions
# ============================================================================

# Print invalid distro error and exit
function print_invalid_distro_error -a arg
    echo "Unknown distro: $arg"
    echo "Valid: ubuntu, fedora, arch"
    exit 1
end

# Check machine exists, print error if not
function require_machine_exists -a distro
    if not machine_exists $distro
        echo "Machine not found: $distro-test"
        echo "Run ./setup.fish first"
        return 1
    end
    return 0
end

# Check zygote exists, print error if not
function require_zygote_exists -a distro
    if not zygote_exists $distro
        echo "No zygote found for $distro"
        echo "Run ./setup.fish to create machines with zygotes"
        return 1
    end
    return 0
end

# Print log file contents
function print_log -a distro
    echo ""
    cat (get_log_path $distro)
end

# ============================================================================
# Test Setup
# ============================================================================

# Copy /etc/ssh into machine (makes it writable for package installers)
# Required for e2e tests that install openssh-server, libvirt, etc.
function copy_etc_ssh_to_machine -a distro
    set machine_path (get_machine_path $distro)
    if sudo test -d $machine_path
        echo "Copying /etc/ssh into container (writable)..."
        sudo cp -a /etc/ssh/* $machine_path/etc/ssh/ 2>/dev/null || true
    end
end

# Copy synced-data into machine (KDE config, terminal settings, ssh keys, etc.)
# Required for e2e tests that run restore scripts like linux-kde-customize-restore
function copy_synced_data_to_machine -a distro
    set machine_path (get_machine_path $distro)
    set host_synced_data ~/github/notes/files/synced-data
    if sudo test -d $machine_path; and test -d $host_synced_data
        echo "Copying synced-data into container..."
        sudo mkdir -p $machine_path/home/tester/synced-data
        sudo cp -a $host_synced_data/* $machine_path/home/tester/synced-data/ 2>/dev/null || true
        sudo chroot $machine_path chown -R tester:tester /home/tester/synced-data
    end
end

# Prepare machine for e2e test (restore zygote + copy required data)
function prepare_machine_for_e2e -a distro
    echo "Restoring from zygote..."
    restore_from_zygote $distro
    # Copy /etc/ssh so package installers can write to it
    copy_etc_ssh_to_machine $distro
    # Copy synced-data for restore scripts (KDE, terminal, ssh configs)
    copy_synced_data_to_machine $distro
end

# ============================================================================
# Test Discovery
# ============================================================================

function find_tests -a category
    set scripts_dir (realpath $tests_dir/..)

    switch $category
        case unit
            find $tests_dir/unit -name "*.fish" 2>/dev/null | sort
        case e2e
            find $tests_dir/e2e -name "*.fish" 2>/dev/null | sort
        case all
            find $tests_dir/unit $tests_dir/e2e -name "*.fish" 2>/dev/null | sort
    end
end

function get_relative_path -a full_path
    string replace "$tests_dir/" "" $full_path
end

# List available tests
function list_tests -a category
    set test_files (find_tests $category)

    if test (count $test_files) -eq 0
        echo "No tests found for category: $category"
        return 1
    end

    echo "Available tests ($category):"
    echo ""
    for test_file in $test_files
        set rel_path (get_relative_path $test_file)
        echo "  $rel_path"
    end
    echo ""
    echo "Total: "(count $test_files)" tests"
end

# ============================================================================
# Single Distro Test Runner
# ============================================================================

function run_tests_on_distro -a distro category
    set log_path (get_log_path $distro)

    # Clear log file
    echo "=== $distro tests started at "(date)" ===" > $log_path

    if not require_machine_exists $distro | tee -a $log_path
        return 1
    end

    # Check for zygote (needed for e2e tests)
    if test "$category" = "e2e" -o "$category" = "all"
        if not require_zygote_exists $distro | tee -a $log_path
            return 1
        end
    end

    set test_files (find_tests $category)
    set total (count $test_files)
    set passed 0
    set failed 0
    set interrupted 0
    set -l failed_tests

    echo "Running $total tests..." >> $log_path

    for test_file in $test_files
        set rel_path (get_relative_path $test_file)
        set is_e2e (string match -q "e2e/*" $rel_path && echo true || echo false)

        echo "" >> $log_path
        echo "--- $rel_path ---" >> $log_path

        # E2E tests: restore from zygote for clean state
        if test "$is_e2e" = true
            prepare_machine_for_e2e $distro >> $log_path 2>&1
        end

        # Run test
        run_in_machine $distro $rel_path >> $log_path 2>&1
        set result $status

        # Check if machine was killed externally during test
        if not machine_running $distro
            set interrupted (math $interrupted + 1)
            echo "INTERRUPTED (machine was killed)" >> $log_path
            # Don't continue - machine is gone
            break
        else if test $result -eq 0
            set passed (math $passed + 1)
            echo "PASS" >> $log_path
        else
            set failed (math $failed + 1)
            set -a failed_tests $rel_path
            echo "FAIL" >> $log_path
        end
    end

    # Write summary to log
    echo "" >> $log_path
    if test $interrupted -gt 0
        echo "=== $distro tests INTERRUPTED at "(date)" ===" >> $log_path
        echo "Tests were interrupted (machine was killed externally)" >> $log_path
        echo "Results before interruption: $passed passed, $failed failed" >> $log_path
        return 130  # Standard interrupted exit code
    end

    echo "=== $distro tests completed at "(date)" ===" >> $log_path
    echo "Results: $passed passed, $failed failed" >> $log_path

    if test $failed -gt 0
        echo "Failed tests:" >> $log_path
        for t in $failed_tests
            echo "  - $t" >> $log_path
        end
        # Stop the machine before returning
        stop_machine $distro
        return 1
    end

    # Stop the machine after all tests complete
    stop_machine $distro
    return 0
end

# ============================================================================
# Parallel Test Runner
# ============================================================================

function run_tests_parallel -a category
    echo "=== Running Tests (Parallel) ==="
    echo ""
    echo "Category: $category"
    echo "Distros: "(string join ", " $ALL_DISTROS)
    echo ""
    echo "Log files:"
    for distro in $ALL_DISTROS
        echo "  $distro: "(get_log_path $distro)
    end
    echo ""
    echo "Monitor progress with: tail -f $LOG_DIR/test-*.log"
    echo ""

    # Check at least one machine exists
    set machines_found 0
    for distro in $ALL_DISTROS
        if machine_exists $distro
            set machines_found (math $machines_found + 1)
        else
            echo "Warning: $distro-test not found (skipping)"
        end
    end

    if test $machines_found -eq 0
        echo "No machines found. Run ./setup.fish first."
        return 1
    end

    echo "Starting tests..."
    echo ""

    # Start tests in parallel (background jobs)
    set -l pids
    for distro in $ALL_DISTROS
        if machine_exists $distro
            # Run each distro's tests in a subprocess
            # Note: We call run.fish with arguments to avoid re-running parallel code
            $tests_dir/run.fish $distro $category &
            set -a pids $last_pid
        end
    end

    # Wait for all to complete
    set -l results
    set idx 1
    for pid in $pids
        wait $pid
        set -a results $status
        set idx (math $idx + 1)
    end

    # Show summary
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo "Summary"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

    set all_passed true
    set idx 1
    for distro in $ALL_DISTROS
        if machine_exists $distro
            set result $results[$idx]
            set log_path (get_log_path $distro)

            if test $result -eq 0
                echo "$distro: PASS"
            else
                echo "$distro: FAIL (see $log_path)"
                set all_passed false
            end
            set idx (math $idx + 1)
        end
    end

    echo ""

    if test "$all_passed" = true
        echo "All tests passed!"
        return 0
    else
        echo "Some tests failed. Check log files for details."
        return 1
    end
end

# ============================================================================
# Single Test Runner
# ============================================================================

function run_single_test -a distro test_path
    set log_path (get_log_path $distro)

    # Clear log file with header
    echo "=== $distro single test started at "(date)" ===" > $log_path
    echo "Test: $test_path" >> $log_path
    echo "" >> $log_path

    if not require_machine_exists $distro | tee -a $log_path
        return 1
    end

    # Check if this is an e2e test
    set is_e2e (string match -q "e2e/*" $test_path && echo true || echo false)

    # E2E tests: restore from zygote for clean state
    if test "$is_e2e" = true
        if not require_zygote_exists $distro | tee -a $log_path
            return 1
        end
        prepare_machine_for_e2e $distro 2>&1 | tee -a $log_path
    end

    echo "=== Running $test_path on $distro ===" | tee -a $log_path
    echo "" | tee -a $log_path

    # Run test and capture output to log (also show on console via tee)
    run_in_machine $distro $test_path 2>&1 | tee -a $log_path
    set result $pipestatus[1]

    # Check if machine was killed externally during test
    # (machinectl shell returns 0 when connection is terminated, which is misleading)
    set was_interrupted false
    if not machine_running $distro
        set was_interrupted true
    end

    # Stop the machine after test completes (if still running)
    stop_machine $distro

    echo "" | tee -a $log_path
    if test "$was_interrupted" = true
        echo "INTERRUPTED (machine was killed during test)" | tee -a $log_path
        echo "" >> $log_path
        echo "=== $distro single test INTERRUPTED at "(date)" ===" >> $log_path
        return 130  # Standard interrupted exit code (128 + SIGINT)
    else if test $result -eq 0
        echo "PASS" | tee -a $log_path
    else
        echo "FAIL (exit code: $result)" | tee -a $log_path
    end

    # Write completion to log
    echo "" >> $log_path
    echo "=== $distro single test completed at "(date)" ===" >> $log_path

    return $result
end

# ============================================================================
# Argument Parsing
# ============================================================================

function is_distro -a arg
    contains $arg $ALL_DISTROS
end

function is_category -a arg
    test "$arg" = "unit" -o "$arg" = "e2e" -o "$arg" = "all"
end

function is_test_path -a arg
    string match -q "*.fish" $arg
    or string match -q "*/*" $arg
end

# ============================================================================
# Main
# ============================================================================

set arg1 $argv[1]
set arg2 $argv[2]

# No arguments = run all tests on all distros
if test -z "$arg1"
    if not ensure_host_prereqs
        exit 1
    end
    # Clean up stale machines from previous runs
    echo "Stopping any stale machines..."
    stop_all_machines
    run_tests_parallel all
    exit $status
end

# Handle special commands
switch $arg1
    case -h --help help
        show_usage
        exit 0

    case start
        # Start machine(s) without running tests
        if test -z "$arg2"
            # Start all machines
            echo "=== Starting All Machines ==="
            for distro in $ALL_DISTROS
                if machine_exists $distro
                    boot_machine $distro
                else
                    echo "$distro-test: not found (run ./setup.fish first)"
                end
            end
            echo "Done"
        else if is_distro $arg2
            # Start single machine
            if machine_exists $arg2
                boot_machine $arg2
            else
                echo "$arg2-test: not found (run ./setup.fish first)"
                exit 1
            end
        else
            print_invalid_distro_error $arg2
        end
        exit $status

    case reboot
        # Reboot machine(s) - kills all processes but keeps machine running
        if test -z "$arg2"
            # Reboot all machines
            echo "=== Rebooting All Machines ==="
            reboot_all_machines
            echo "Done"
        else if is_distro $arg2
            # Reboot single machine
            reboot_machine $arg2
        else
            print_invalid_distro_error $arg2
        end
        exit $status

    case stop
        # Stop machine(s)
        if test -z "$arg2"
            # Stop all machines
            echo "=== Stopping All Machines ==="
            stop_all_machines
            echo "Done"
        else if is_distro $arg2
            # Stop single machine
            stop_machine $arg2
        else
            print_invalid_distro_error $arg2
        end
        exit $status

    case nuke
        # Force reset: delete + recreate from zygote
        if test -z "$arg2"
            echo "Usage: ./run.fish nuke <distro>"
            echo ""
            echo "Force reset a machine by deleting it and recreating from zygote."
            echo "Use when a machine is corrupted or you want a clean slate."
            exit 1
        end
        if not is_distro $arg2
            print_invalid_distro_error $arg2
        end
        nuke_machine $arg2
        exit $status

    case shell
        # Interactive shell
        if test -z "$arg2"
            echo "Usage: ./run.fish shell <distro>"
            exit 1
        end
        if not is_distro $arg2
            print_invalid_distro_error $arg2
        end
        run_in_machine $arg2 ""
        exit $status

    case ps
        # Show processes
        if test -z "$arg2"
            show_all_machine_processes
        else if is_distro $arg2
            show_machine_processes $arg2
        else
            print_invalid_distro_error $arg2
        end
        exit 0

    case logs
        # Show log file contents
        if test -z "$arg2"
            show_all_logs 20
        else if is_distro $arg2
            show_log $arg2 20
        else
            print_invalid_distro_error $arg2
        end
        exit 0

    case list
        # List available tests
        if test -z "$arg2"
            list_tests all
        else if is_category $arg2
            list_tests $arg2
        else
            echo "Unknown category: $arg2"
            echo "Valid categories: unit, e2e, all"
            exit 1
        end
        exit 0
end

# Ensure prerequisites for test runs
if not ensure_host_prereqs
    exit 1
end

# Parse remaining arguments
if is_category $arg1
    # ./run.fish unit [distro]
    # ./run.fish e2e [distro]
    if test -z "$arg2"
        run_tests_parallel $arg1
    else if is_distro $arg2
        run_tests_on_distro $arg2 $arg1
        print_log $arg2
    else
        print_invalid_distro_error $arg2
    end
    exit $status

else if is_distro $arg1
    if test -z "$arg2"
        # ./run.fish ubuntu = all tests on ubuntu
        run_tests_on_distro $arg1 all
        print_log $arg1
        exit $status
    else if is_category $arg2
        # ./run.fish ubuntu unit
        run_tests_on_distro $arg1 $arg2
        print_log $arg1
        exit $status
    else if is_test_path $arg2
        # ./run.fish ubuntu unit/test-foo.fish
        run_single_test $arg1 $arg2
        exit $status
    else
        echo "Unknown: $arg2"
        show_usage
        exit 1
    end

else
    echo "Unknown: $arg1"
    show_usage
    exit 1
end
