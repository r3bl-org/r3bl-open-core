#!/usr/bin/env fish

# Shared systemd-nspawn machine operations
#
# Usage: source /path/to/tests/lib/nspawn.fish
#
# Provides functions for test execution and machine inspection.

# ============================================================================
# Configuration
# ============================================================================

# Machines on real filesystem (large, can grow during tests)
# Zygotes in tmpfs (small, fast cloning)
set -g MACHINES_DIR /var/tmp/test-machines
set -g ZYGOTES_DIR /tmp/zygotes
set -g ALL_DISTROS ubuntu fedora arch
set -g LOG_DIR /tmp

# ============================================================================
# Path Helpers
# ============================================================================

function get_machine_name -a distro -d "Get machine name for a distro"
    echo "$distro-test"
end

function get_machine_path -a distro -d "Get machine directory path"
    echo "$MACHINES_DIR/$distro-test"
end

function get_zygote_path -a distro -d "Get zygote directory path"
    echo "$ZYGOTES_DIR/$distro-test"
end

function get_dir_size -a path -d "Get human-readable size of a directory"
    sudo du -sh $path 2>/dev/null | cut -f1
end

# ============================================================================
# Machine State
# ============================================================================

function machine_exists -a distro -d "Check if machine directory exists"
    sudo test -d (get_machine_path $distro)
end

function machine_running -a distro -d "Check if machine is currently running"
    set machine_name (get_machine_name $distro)
    sudo machinectl show "$machine_name" 2>/dev/null | grep -q "State=running"
end

function machine_registered -a distro -d "Check if machine is registered with machinectl (any state)"
    set machine_name (get_machine_name $distro)
    sudo machinectl show "$machine_name" >/dev/null 2>&1
end

function get_machine_leader -a distro -d "Get the leader PID of a running machine"
    set machine_name (get_machine_name $distro)
    sudo machinectl show "$machine_name" -p Leader --value 2>/dev/null
end

function cleanup_stale_mounts -a distro -d "Clean up stale mounts from crashed/interrupted runs"
    set machine_name (get_machine_name $distro)
    sudo umount /run/systemd/nspawn/unix-export/$machine_name 2>/dev/null
    sudo rm -rf /run/systemd/nspawn/unix-export/$machine_name 2>/dev/null
end

function cleanup_stale_registration -a distro -d "Clean up stale machine registration from systemd-machined"
    set machine_name (get_machine_name $distro)
    # Remove stale registration files
    sudo rm -f /run/systemd/machines/$machine_name 2>/dev/null
    sudo rm -f /run/systemd/machines/"unit:machine-$machine_name.scope" 2>/dev/null
    # Restart machined to clear internal state
    sudo systemctl restart systemd-machined 2>/dev/null
end

# ============================================================================
# Test Execution
# ============================================================================

function run_in_machine -a distro test_path -d "Run a test script in a booted machine"
    set machine_name (get_machine_name $distro)

    # Ensure machine is booted
    if not machine_running $distro
        if not boot_machine $distro
            return 1
        end
    end

    # Use nsenter instead of machinectl shell.
    # machinectl shell's PTY causes fish to hang during terminal feature detection.
    set leader (get_machine_leader $distro)
    if test -z "$leader"
        echo "Error: could not get leader PID for $machine_name"
        return 1
    end

    if test -n "$test_path"
        # sed -u (unbuffered) adds CR before LF (container outputs \n, terminal needs \r\n)
        # Note: no TTY allocated, so isatty checks in fish scripts will detect non-interactive context
        sudo nsenter -t $leader -m -u -i -n -p \
            --setuid 1000 --setgid 1000 -- \
            /usr/bin/env HOME=/home/tester SYNCED_DATA_ROOT=/home/tester/synced-data \
            /usr/bin/fish /scripts/tests/$test_path 2>&1 | sed -u 's/$/\r/'
        return $pipestatus[1]
    else
        # Interactive shell as tester user (bash for reliability)
        echo "Entering $machine_name as tester (bash shell)"
        echo "Type 'exit' to return. Use ./run.fish stop to stop machine."
        echo ""
        # script allocates a PTY inside the container, which:
        # - Eliminates "tty: ttyname error: No such device" warning from bash's .profile
        # - Enables proper terminal features (colors, line editing, job control)
        # Note: script must run INSIDE container (after nsenter) because PTY namespaces are isolated
        sudo nsenter -t $leader -m -u -i -n -p --setuid 1000 --setgid 1000 -- \
            /usr/bin/env HOME=/home/tester TERM=$TERM \
            /usr/bin/script -q /dev/null -c "/bin/bash --login"
    end
end

# ============================================================================
# Process Inspection (nsenter)
# ============================================================================

# Show processes inside a machine using nsenter
#
# Sample output:
#   === ubuntu-test (PID 1175838) ===
#       PID TTY      STAT   TIME COMMAND
#       302 pts/1    Ssl+   0:00 /usr/bin/fish /scripts/tests/e2e/02-test-fresh-install-stateful.fish
#       304 pts/1    S+     0:00  \_ (sd-pam)
#       381 pts/1    S+     0:00  \_ fish /scripts/local-backup-restore/fresh-install/01-fresh-install.fish
#     39634 pts/1    S+     0:00      \_ sudo apt install -y linux-tools-common linux-tools-generic
function show_machine_processes -a distro -d "Show processes inside a machine using nsenter"
    set machine_name (get_machine_name $distro)

    if not machine_running $distro
        echo "=== $machine_name ==="
        echo "(not running)"
        echo ""
        return 0
    end

    set leader (get_machine_leader $distro)
    if test -z "$leader"
        echo "=== $machine_name ==="
        echo "(could not get leader PID)"
        echo ""
        return 1
    end

    echo "=== $machine_name (PID $leader) ==="
    # Show fish process tree: find fish PID, get its session ID, display all processes in that session as forest
    sudo nsenter -t $leader -m -u -i -n -p -- /bin/sh -c 'fish_pid=$(pgrep -x fish | head -1); if [ -n "$fish_pid" ]; then ps f --width 200 --forest -g $(ps -o sid= -p $fish_pid | tr -d " ") 2>/dev/null; fi'
    echo ""
end

function show_all_machine_processes -d "Show processes in all running machines"
    for distro in $ALL_DISTROS
        show_machine_processes $distro
    end
end

# ============================================================================
# Machine Lifecycle
# ============================================================================

function boot_machine -a distro -d "Boot a machine with systemd as PID 1"
    set machine_name (get_machine_name $distro)
    set machine_path (get_machine_path $distro)
    set scripts_dir (realpath (dirname (status filename))/../..)

    # Reuse if already running
    if machine_running $distro
        echo "Machine $machine_name already running"
        return 0
    end

    if not machine_exists $distro
        echo "Machine not found: $machine_path"
        echo "Run ./setup.fish first"
        return 1
    end

    cleanup_stale_mounts $distro

    # Bind mounts (read-only for safety)
    # Note: fish config in ~/.config/fish/ sources from /scripts/fish/
    # This keeps ~/.config/fish/ writable for fish_variables
    set synced_data_dir (realpath $scripts_dir/../synced-data)
    set bind_opts \
        --bind-ro=$scripts_dir:/scripts \
        --bind-ro=$synced_data_dir:/synced-data \
        --bind-ro=$HOME:/host-home

    # Environment variables
    # SYNCED_DATA_ROOT is set explicitly since path detection doesn't work with /scripts mount
    set env_opts \
        --setenv=HOST_USER=$USER \
        --setenv=HOST_HOME=/host-home \
        --setenv=SYNCED_DATA_ROOT=/synced-data

    echo "Booting $machine_name..."
    set nspawn_log /tmp/nspawn-$machine_name.log
    sudo systemd-nspawn \
        -bD $machine_path \
        --machine=$machine_name \
        $bind_opts \
        $env_opts \
        --capability=CAP_NET_ADMIN \
        --resolv-conf=copy-uplink \
        >$nspawn_log 2>&1 &

    # Wait for boot to complete
    if not wait_for_boot $distro 30
        echo "Failed to boot $machine_name"
        return 1
    end

    echo "Machine $machine_name booted"
    return 0
end

function wait_for_boot -a distro -a timeout -d "Wait for machine to finish booting"
    set machine_name (get_machine_name $distro)
    set waited 0

    while test $waited -lt $timeout
        # Check if machine is running and systemd is ready
        if machine_running $distro
            # Try to run a simple command - if it works, machine is ready
            if sudo machinectl shell root@$machine_name /bin/true >/dev/null 2>&1
                return 0
            end
        end

        sleep 1
        set waited (math $waited + 1)
    end

    echo "Timeout waiting for $machine_name to boot after $timeout seconds"
    return 1
end

function wait_for_machine_stopped -a distro -a timeout -d "Wait until machine is fully unregistered"
    set machine_name (get_machine_name $distro)

    for i in (seq $timeout)
        if not sudo machinectl show "$machine_name" >/dev/null 2>&1
            return 0
        end
        sleep 1
    end

    return 1
end

function reboot_machine -a distro -d "Reboot a machine (stop + boot)"
    set machine_name (get_machine_name $distro)
    echo "Rebooting $machine_name..."
    stop_machine $distro
    if not boot_machine $distro
        echo "Failed to reboot $machine_name"
        return 1
    end
    echo "$machine_name rebooted"
    return 0
end

function reboot_all_machines -d "Reboot all machines (boot stopped ones)"
    for distro in $ALL_DISTROS
        if machine_running $distro
            reboot_machine $distro
        else if machine_exists $distro
            set machine_name (get_machine_name $distro)
            echo "$machine_name is stopped, booting..."
            boot_machine $distro
        end
    end
end

function stop_machine -a distro -d "Stop a running machine"
    set machine_name (get_machine_name $distro)

    # Not registered = already stopped
    if not machine_registered $distro
        return 0
    end

    echo "Stopping $machine_name..."

    # Try graceful shutdown first (5s timeout)
    sudo machinectl poweroff "$machine_name" 2>/dev/null
    if wait_for_machine_stopped $distro 5
        return 0
    end

    # Try terminate (5s timeout)
    sudo machinectl terminate "$machine_name" 2>/dev/null
    if wait_for_machine_stopped $distro 5
        return 0
    end

    # Try kill signal (5s timeout)
    sudo machinectl kill "$machine_name" 2>/dev/null
    if wait_for_machine_stopped $distro 5
        return 0
    end

    # Last resort: kill the nspawn process directly
    sudo pkill -9 -f "systemd-nspawn.*--machine=$machine_name" 2>/dev/null
    if wait_for_machine_stopped $distro 10
        return 0
    end

    # Nuclear option: stop the systemd scope and kill all processes in it
    echo "Force killing $machine_name..."
    set scope "machine-$machine_name.scope"
    sudo systemctl stop "$scope" 2>/dev/null
    sudo systemctl reset-failed "$scope" 2>/dev/null
    if wait_for_machine_stopped $distro 5
        return 0
    end

    # Machine still stuck - clean up stale state
    if not machine_running $distro
        echo "Warning: $machine_name in stale state, cleaning up..."
        cleanup_stale_mounts $distro
        cleanup_stale_registration $distro
        return 0
    end

    # Nuclear option: machine truly won't stop, nuke it
    echo "Error: $machine_name won't stop, nuking..."
    nuke_machine $distro
    return 0
end

function stop_all_machines -d "Stop all running machines"
    for distro in $ALL_DISTROS
        stop_machine $distro
    end
end

function nuke_machine -a distro -d "Nuclear option: force kill + delete + recreate from zygote"
    set machine_name (get_machine_name $distro)
    set machine_path (get_machine_path $distro)
    set zygote_path (get_zygote_path $distro)

    echo "=== Nuking $machine_name ==="

    # Step 1: Force kill everything
    echo "Step 1: Force killing all processes..."
    sudo pkill -9 -f "systemd-nspawn.*--machine=$machine_name" 2>/dev/null
    sudo machinectl terminate "$machine_name" 2>/dev/null
    sleep 1

    # Step 2: Clean up stale state
    echo "Step 2: Cleaning up stale state..."
    cleanup_stale_mounts $distro
    cleanup_stale_registration $distro

    # Step 3: Delete machine directory
    echo "Step 3: Deleting machine directory..."
    if sudo test -d $machine_path
        set size (get_dir_size $machine_path)
        sudo rm -rf $machine_path
        echo "  Deleted $machine_path ($size)"
    else
        echo "  $machine_path not found (already clean)"
    end

    # Step 4: Recreate from zygote
    if test -d $zygote_path
        echo "Step 4: Recreating from zygote..."
        sudo cp -a $zygote_path $machine_path
        echo "  Recreated from $zygote_path"
        echo ""
        echo "$machine_name nuked and recreated. Use './run.fish start $distro' to boot."
    else
        echo "Step 4: No zygote available"
        echo ""
        echo "$machine_name nuked. Run './setup.fish $distro' to recreate."
    end
end

function delete_machine -a distro -d "Delete a machine directory"
    set machine_name (get_machine_name $distro)
    set machine_path (get_machine_path $distro)
    if sudo test -d $machine_path
        set size (get_dir_size $machine_path)
        echo "Deleting $machine_name ($size)..."
        sudo rm -rf $machine_path
    else
        echo "$machine_name: not found (already clean)"
    end
end

function delete_all_machines -d "Delete all machine directories"
    for distro in $ALL_DISTROS
        delete_machine $distro
    end
end

# ============================================================================
# Zygotes (Golden Images)
# ============================================================================

function zygote_exists -a distro -d "Check if zygote exists for a distro"
    sudo test -d (get_zygote_path $distro)
end

function save_zygote -a distro -d "Save machine as zygote (golden image)"
    set machine_name (get_machine_name $distro)
    set machine_path (get_machine_path $distro)
    set zygote_path (get_zygote_path $distro)

    if not machine_exists $distro
        echo "Cannot save zygote: $machine_name does not exist"
        return 1
    end

    # Create zygotes directory if needed
    sudo mkdir -p $ZYGOTES_DIR

    # Remove old zygote if exists
    if sudo test -d $zygote_path
        echo "Removing old zygote for $distro..."
        sudo rm -rf $zygote_path
    end

    echo "Saving $machine_name as zygote..."
    sudo cp -a $machine_path $zygote_path
    set size (get_dir_size $zygote_path)
    echo "Zygote saved: $zygote_path ($size)"
end

function restore_from_zygote -a distro -d "Restore machine from zygote (clean state)"
    set machine_name (get_machine_name $distro)
    set machine_path (get_machine_path $distro)
    set zygote_path (get_zygote_path $distro)

    if not zygote_exists $distro
        echo "No zygote found for $distro"
        echo "Run ./setup.fish first"
        return 1
    end

    # Always stop machine (handles any state: running, stopping, or not registered)
    stop_machine $distro

    # Ensure machine is fully stopped before proceeding
    if not wait_for_machine_stopped $distro 30
        echo "Error: $machine_name not fully stopped, cannot restore"
        return 1
    end

    # Remove current machine
    if sudo test -d $machine_path
        sudo rm -rf $machine_path
        # Verify removal
        if sudo test -d $machine_path
            echo "Error: Failed to remove $machine_path"
            return 1
        end
    end

    echo "Restoring $machine_name from zygote..."
    sudo cp -a $zygote_path $machine_path
    echo "Restored clean $machine_name"
end

function delete_zygote -a distro -d "Delete a zygote"
    set machine_name (get_machine_name $distro)
    set zygote_path (get_zygote_path $distro)
    if sudo test -d $zygote_path
        set size (get_dir_size $zygote_path)
        echo "Deleting zygote $machine_name ($size)..."
        sudo rm -rf $zygote_path
    end
end

function delete_all_zygotes -d "Delete all zygotes"
    if sudo test -d $ZYGOTES_DIR
        set size (get_dir_size $ZYGOTES_DIR)
        echo "Deleting all zygotes ($size)..."
        sudo rm -rf $ZYGOTES_DIR
    end
end

function show_zygote_status -d "Show zygote status for all distros"
    echo "=== Zygotes ($ZYGOTES_DIR/) ==="
    if not test -d $ZYGOTES_DIR
        echo "  (no zygotes - run ./setup.fish to create)"
        return
    end
    for distro in $ALL_DISTROS
        set zygote_path (get_zygote_path $distro)
        if test -d $zygote_path
            set size (get_dir_size $zygote_path)
            echo "  $zygote_path ($size)"
        else
            echo "  $zygote_path (not found)"
        end
    end
end

# ============================================================================
# Resource Usage
# ============================================================================

function get_machine_resources -a distro -d "Get memory and CPU usage for a running machine"
    set machine_name (get_machine_name $distro)

    if not machine_running $distro
        return 1
    end

    # Get cgroup stats via systemctl
    # Escape the machine name for systemd scope (- becomes \x2d)
    set scope_name "machine-"(string replace -a '-' '\\x2d' $machine_name)".scope"

    # Get values separately using --value for clean output
    set mem_bytes (systemctl show "$scope_name" --property=MemoryCurrent --value 2>/dev/null)
    set cpu_ns (systemctl show "$scope_name" --property=CPUUsageNSec --value 2>/dev/null)

    # Convert memory to human-readable (MiB or GiB)
    if test -n "$mem_bytes" -a "$mem_bytes" != "[not set]"
        set mem_mib (math "round($mem_bytes / 1048576)")
        if test $mem_mib -ge 1024
            set mem_gib (math "round($mem_bytes / 1073741824 * 10) / 10")
            set mem_str "$mem_gib GiB"
        else
            set mem_str "$mem_mib MiB"
        end
    else
        set mem_str "-"
    end

    # Convert CPU time to human-readable (s, m, or h)
    if test -n "$cpu_ns" -a "$cpu_ns" != "[not set]"
        set cpu_sec (math "round($cpu_ns / 1000000000)")
        if test $cpu_sec -ge 3600
            set cpu_hr (math "round($cpu_sec / 3600 * 10) / 10")
            set cpu_str "$cpu_hr h"
        else if test $cpu_sec -ge 60
            set cpu_min (math "round($cpu_sec / 60)")
            set cpu_str "$cpu_min m"
        else
            set cpu_str "$cpu_sec s"
        end
    else
        set cpu_str "-"
    end

    echo "Mem: $mem_str, CPU: $cpu_str"
end

# ============================================================================
# Log Files
# ============================================================================

function get_log_path -a distro -d "Get log file path for a distro"
    echo "$LOG_DIR/test-$distro.log"
end

function get_setup_log_path -a distro -d "Get setup log file path for a distro"
    echo "$LOG_DIR/setup-$distro.log"
end

function get_teardown_log_path -a distro -d "Get teardown log file path for a distro"
    echo "$LOG_DIR/teardown-$distro.log"
end

function get_status_log_path -d "Get status log file path"
    echo "$LOG_DIR/status.log"
end

function clear_log -a distro -d "Clear/create log file for a distro"
    set log_path (get_log_path $distro)
    echo "" > $log_path
end

function show_log -a distro -a lines -d "Show last N lines from a distro's log file"
    test -n "$lines"; or set lines 20
    set log_path (get_log_path $distro)

    echo "=== $distro ($log_path) ==="
    if test -f $log_path
        tail -$lines $log_path
    else
        echo "(no log file)"
    end
    echo ""
end

function show_all_logs -a lines -d "Show last N lines from all log files"
    test -n "$lines"; or set lines 20
    for distro in $ALL_DISTROS
        show_log $distro $lines
    end
end
