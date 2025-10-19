#!/usr/bin/env fish

# r3bl Development Tmux Session
#
# Purpose: Creates a 3-pane tmux session for r3bl development
#
# Layout: 2 top panes + 1 bottom spanning pane
#   ├─ Top-left:    watch-doc
#   ├─ Top-right:   watch-test
#   └─ Bottom:      (empty)
#
# Usage: ./tmux-r3bl-dev.fish
#
# This script creates or attaches to a session named "r3bl-dev"

set SESSION_NAME r3bl-dev

function run_in_pane
    set -l pane_id $argv[1]
    set -l command $argv[2..-1]
    tmux send-keys -t $SESSION_NAME:$pane_id "$command" C-m
end

function main
    if tmux has-session -t $SESSION_NAME 2>/dev/null
        echo "Found existing session '$SESSION_NAME'. Attaching to it."
        tmux attach-session -t $SESSION_NAME
    else
        echo "Creating new session '$SESSION_NAME'..."

        # Create a new session with first pane
        tmux new-session -d -s $SESSION_NAME

        # Split window vertically (top and bottom)
        tmux split-window -v

        # Explicitly select the top pane
        tmux select-pane -t "$SESSION_NAME:0.0"

        # Split top pane horizontally (top-left and top-right)
        tmux split-window -h

        # Now we have 3 panes:
        # 0.0 = top-left
        # 0.1 = top-right
        # 0.2 = bottom (spanning full width)

        echo "Running commands in panes..."

        # Top-left: watch-doc
        run_in_pane "0.0" "cd ~/github/r3bl-open-core ; ./check.fish --watch-doc"

        # Top-right: watch-test
        run_in_pane "0.1" "cd ~/github/r3bl-open-core ; ./check.fish --watch-test"

        # Bottom: empty (no command)

        # Select the top-left pane as active
        tmux select-pane -t "$SESSION_NAME:0.0"

        # Attach to the created session
        tmux attach-session -t $SESSION_NAME
    end
end

main
