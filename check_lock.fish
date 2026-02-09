# Single Instance Enforcement (watch modes only)
#
# Only ONE watch mode instance can run at a time.
# One-off commands (--test, --doc, default) can run concurrently without issues.
# Uses PID file with liveness check (fish doesn't support bash-style flock FD syntax).
# PID file: /tmp/roc/check.fish.pid (checked via kill -0 for cross-platform support).
# When entering watch mode, kills existing instance and orphaned file watcher processes.
# Prevents race conditions where multiple watch instances clean target/check simultaneously.

# Cross-platform check if a process is alive.
# Uses kill -0 which works on both Linux and macOS (doesn't actually send a signal).
# Returns: 0 if alive, 1 if not alive or invalid PID
function is_process_alive
    set -l pid $argv[1]
    test -n "$pid" && kill -0 $pid 2>/dev/null
end

# Acquire exclusive "lock" for watch mode, killing any existing holder.
#
# Uses a PID file approach that works in fish:
# 1. Check if PID file exists and that process is still alive
# 2. If alive: kill it and wait for it to exit
# 3. Write our PID to the file
#
# Note: Unlike flock, PID files can become stale on SIGKILL. We handle this
# by checking if the process is alive (cross-platform: kill -0).
#
# Also kills orphaned file watcher processes from previous sessions.
function acquire_watch_lock
    mkdir -p (dirname $CHECK_LOCK_FILE)

    # Check if another instance is running
    if test -f $CHECK_LOCK_FILE
        set -l old_pid (cat $CHECK_LOCK_FILE 2>/dev/null | string trim)

        if is_process_alive $old_pid
            # Process is alive - kill it
            echo ""
            set_color yellow
            echo "⚠️  Another watch instance running (PID: $old_pid)"
            echo "🔪 Killing to prevent race conditions..."
            set_color normal

            kill $old_pid 2>/dev/null

            # Wait for process to exit (up to 5 seconds)
            set -l waited 0
            while is_process_alive $old_pid && test $waited -lt 50
                sleep 0.1
                set waited (math $waited + 1)
            end

            if is_process_alive $old_pid
                echo "❌ Failed to terminate previous instance" >&2
                return 1
            end

            echo "✅ Previous instance terminated"
            echo ""
        end
        # else: stale PID file, process already gone - just overwrite
    end

    # Write our PID
    echo $fish_pid > $CHECK_LOCK_FILE

    # Clean up any orphaned file watcher processes (inotifywait/fswatch)
    kill_orphaned_watchers

    return 0
end

# Kill orphaned file watcher processes from previous watch mode sessions.
# These can be left behind if the parent was killed with SIGKILL.
# Supports both inotifywait (Linux) and fswatch (macOS).
function kill_orphaned_watchers
    # Kill orphaned inotifywait (Linux)
    set -l inotify_pids (pgrep -f "inotifywait.*cmdr/src" 2>/dev/null)
    if test (count $inotify_pids) -gt 0
        for pid in $inotify_pids
            kill $pid 2>/dev/null
        end
    end

    # Kill orphaned fswatch (macOS)
    set -l fswatch_pids (pgrep -f "fswatch.*cmdr/src" 2>/dev/null)
    if test (count $fswatch_pids) -gt 0
        for pid in $fswatch_pids
            kill $pid 2>/dev/null
        end
    end
end

# NOTE: acquire_watch_lock is called only for watch modes (see watch_mode function)
# One-off commands (--test, --doc, default) can run concurrently without conflict
