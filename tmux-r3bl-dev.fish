#!/usr/bin/env fish

# r3bl Development Tmux Session
#
# Purpose: Creates a 4-pane tmux session for r3bl development
#
# Layout: 2x2 grid
#   ├─ Top-left:    nextest
#   ├─ Top-right:   doc
#   ├─ Bottom-left: doctests
#   └─ Bottom-right: watch
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

        # Split window vertically (left and right)
        tmux split-window -h

        # Split left pane horizontally (top-left and bottom-left)
        tmux split-window -v -t "$SESSION_NAME:0.0"

        # Split bottom-left pane horizontally (middle-left and bottom-left)
        tmux split-window -v -t "$SESSION_NAME:0.2"

        # Now we have 4 panes:
        # 0.0 = top-left
        # 0.1 = top-right
        # 0.2 = bottom-left
        # 0.3 = bottom-right

        echo "Running commands in panes..."

        # Top-left: nextest
        run_in_pane "0.0" "cd ~/github/r3bl-open-core ; bacon nextest --headless"

        # Top-right: doc
        run_in_pane "0.1" "cd ~/github/r3bl-open-core ; bacon doc --headless"

        # Bottom-left: doctests
        run_in_pane "0.2" "cd ~/github/r3bl-open-core ; bacon doctests --headless"

        # Bottom-right: watch
        run_in_pane "0.3" "cd ~/github/r3bl-open-core ; watch -n 60 ./check.fish"
        #run_in_pane "0.3" "cd ~/github/r3bl-open-core ; bacon --headless"

        # Select the top-left pane as active
        tmux select-pane -t "$SESSION_NAME:0.0"

        # Attach to the created session
        tmux attach-session -t $SESSION_NAME
    end
end

main
