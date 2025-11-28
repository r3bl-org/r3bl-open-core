#!/usr/bin/env fish

# r3bl Development Tmux Session
#
# Purpose: Creates a 2-pane tmux session for r3bl development
#
# Layout: 2 vertical panes
#   ├─ Top:     watch-doc (./check.fish --watch-doc)
#   └─ Bottom:  (empty, focused)
#
# Usage: ./tmux-r3bl-dev.fish
#
# This script creates or attaches to a session named "r3bl"

set SESSION_NAME r3bl

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

        # Split window vertically (top 15%, bottom 85%)
        # -p specifies the percentage for the NEW pane (bottom)
        tmux split-window -v -p 85

        # Now we have 2 panes:
        # 0.0 = top
        # 0.1 = bottom

        echo "Running commands in panes..."

        # Top: watch-doc
        run_in_pane "0.0" "cd ~/github/r3bl-open-core ; ./check.fish --watch-doc"

        # Bottom: empty (no command)

        # Select the bottom pane as active (focused)
        tmux select-pane -t "$SESSION_NAME:0.1"

        # Attach to the created session
        tmux attach-session -t $SESSION_NAME
    end
end

main
