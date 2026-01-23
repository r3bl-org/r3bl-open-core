#!/usr/bin/env fish

# Stop and delete systemd-nspawn machines and zygotes
#
# Usage:
#   ./teardown.fish           # Stop + delete all machines + zygotes
#   ./teardown.fish ubuntu    # Stop + delete ubuntu only + its zygote
#   ./teardown.fish fedora    # Stop + delete fedora only + its zygote
#   ./teardown.fish arch      # Stop + delete arch only + its zygote

set tests_dir (dirname (status filename))
source $tests_dir/lib/nspawn.fish

# ============================================================================
# Teardown Function
# ============================================================================

function teardown_machine -a distro
    # Step 1: Stop if running (uses nspawn.fish helper)
    stop_machine $distro

    # Step 2: Delete machine directory (uses nspawn.fish helper)
    delete_machine $distro

    # Step 3: Delete zygote
    delete_zygote $distro
end

function teardown_all
    echo "=== Teardown All Machines and Zygotes ==="
    echo ""

    # Stop all running machines (uses nspawn.fish helper)
    stop_all_machines

    echo ""

    # Delete all machine directories (uses nspawn.fish helper)
    delete_all_machines

    echo ""

    # Delete all zygotes
    delete_all_zygotes

    echo ""
    echo "Teardown complete"

    # Show remaining disk usage
    if test -d $MACHINES_DIR
        set remaining (get_dir_size $MACHINES_DIR)
        echo "$MACHINES_DIR/ usage: $remaining"
    end
end

# ============================================================================
# Logging Helpers
# ============================================================================

function teardown_distro_with_logging -a distro
    set log_path (get_teardown_log_path $distro)
    echo "=== Teardown $distro started at "(date)" ===" > $log_path
    echo "Log file: $log_path"

    teardown_machine $distro 2>&1 | tee -a $log_path
    set result $pipestatus[1]

    echo "" >> $log_path
    echo "=== Teardown $distro completed at "(date)" ===" >> $log_path

    return $result
end

function teardown_all_with_logging
    # Log to all distro log files
    for distro in $ALL_DISTROS
        set log_path (get_teardown_log_path $distro)
        echo "=== Teardown all started at "(date)" ===" > $log_path
    end
    echo "Log files: /tmp/teardown-{ubuntu,fedora,arch}.log"

    teardown_all 2>&1 | tee -a (get_teardown_log_path ubuntu) | tee -a (get_teardown_log_path fedora) | tee -a (get_teardown_log_path arch)

    for distro in $ALL_DISTROS
        set log_path (get_teardown_log_path $distro)
        echo "" >> $log_path
        echo "=== Teardown all completed at "(date)" ===" >> $log_path
    end
end

# ============================================================================
# Main
# ============================================================================

set target $argv[1]
if test -z "$target"
    set target "all"
end

switch $target
    case ubuntu fedora arch
        teardown_distro_with_logging $target
    case all
        teardown_all_with_logging
    case -h --help
        echo "Usage: ./teardown.fish [ubuntu|fedora|arch|all]"
        echo ""
        echo "Stops running machines, deletes machine directories and zygotes."
        echo "This reclaims disk space used by $MACHINES_DIR/*-test"
        echo "and $ZYGOTES_DIR/"
        echo ""
        echo "Log files: /tmp/teardown-<distro>.log"
        exit 0
    case '*'
        echo "Unknown: $target"
        echo "Usage: ./teardown.fish [ubuntu|fedora|arch|all]"
        exit 1
end
