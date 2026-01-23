#!/usr/bin/env fish

# Show status of systemd-nspawn test machines
#
# Usage:
#   ./status.fish           # Show machine and zygote status
#   ./status.fish -v        # Verbose: also show process trees for running machines
#   ./status.fish --verbose # Same as -v
#
# Shows runtime status (RUNNING/STOPPED) for each machine, plus zygote status.

set tests_dir (dirname (status filename))
source $tests_dir/lib/nspawn.fish

# ============================================================================
# Status Display
# ============================================================================

function show_machine_status
    echo "=== Machine Status ==="
    for distro in $ALL_DISTROS
        set machine_name (get_machine_name $distro)
        set machine_path (get_machine_path $distro)

        # Determine status
        if machine_running $distro
            set status_str (set_color green)"RUNNING"(set_color normal)
        else if machine_exists $distro
            set status_str (set_color yellow)"STOPPED"(set_color normal)
        else
            set status_str (set_color red)"NOT FOUND"(set_color normal)
        end

        # Get size if exists
        if machine_exists $distro
            set size (get_dir_size $machine_path)

            # Get resource usage for running machines
            if machine_running $distro
                set resources (get_machine_resources $distro)
                printf "  %-14s %s  (%s)  [%s]\n" "$machine_name:" "$status_str" "$size" "$resources"
            else
                printf "  %-14s %s  (%s)\n" "$machine_name:" "$status_str" "$size"
            end
        else
            printf "  %-14s %s\n" "$machine_name:" "$status_str"
        end
    end
end

function show_zygotes
    echo ""
    echo "=== Zygotes ==="
    if not test -d $ZYGOTES_DIR
        echo "  (none - run ./setup.fish to create)"
        return
    end

    for distro in $ALL_DISTROS
        set machine_name (get_machine_name $distro)
        set zygote_path (get_zygote_path $distro)
        if test -d $zygote_path
            set size (get_dir_size $zygote_path)
            printf "  %-14s %s\n" "$machine_name:" "$size"
        else
            printf "  %-14s %s\n" "$machine_name:" (set_color brblack)"not found"(set_color normal)
        end
    end
end

function show_running_processes
    echo ""
    echo "=== Running Processes ==="

    set any_running false
    for distro in $ALL_DISTROS
        if machine_running $distro
            set any_running true
            show_machine_processes $distro
        end
    end

    if test "$any_running" = false
        echo "  (no machines running)"
    end
end

# ============================================================================
# Main
# ============================================================================

set verbose false

for arg in $argv
    switch $arg
        case -v --verbose
            set verbose true
        case -h --help
            echo "Usage: ./status.fish [-v|--verbose]"
            echo ""
            echo "Shows runtime status of test machines and zygotes."
            echo ""
            echo "Options:"
            echo "  -v, --verbose  Also show process trees for running machines"
            echo ""
            echo "Status meanings:"
            echo "  RUNNING    Machine is booted and active"
            echo "  STOPPED    Machine exists but not running"
            echo "  NOT FOUND  Machine directory doesn't exist (run ./setup.fish)"
            echo ""
            echo "Log file: /tmp/status.log"
            exit 0
        case '*'
            echo "Unknown option: $arg"
            echo "Usage: ./status.fish [-v|--verbose]"
            exit 1
    end
end

# Log all output
set log_path (get_status_log_path)
echo "=== Status check at "(date)" ===" > $log_path
echo "Log file: $log_path"

# Run status checks and log output
begin
    show_machine_status
    show_zygotes

    if test "$verbose" = true
        show_running_processes
    end
end 2>&1 | tee -a $log_path
