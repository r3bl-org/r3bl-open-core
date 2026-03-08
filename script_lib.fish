#!/usr/bin/env fish
# This file contains utility functions that shared between all the `run` scripts
# in various sub folders in this workspace.

# Cross-platform file watcher that monitors filesystem changes and runs commands.
#
# This function provides a unified interface for file watching across macOS (fswatch)
# and Linux (inotifywait). It continuously monitors a directory for changes and
# executes a specified command whenever changes are detected.
#
# Parameters:
# - command: The shell command to execute when changes are detected
# - dir: Directory to watch (default: current directory)
#
# Features:
# - Automatic platform detection (Darwin/Linux)
# - Excludes target/ and .git/ directories
# - Graceful Ctrl+C handling
# - Continuous monitoring loop
#
# Watched events:
# - File modifications
# - File creation
# - File deletion
# - File moves/renames
#
# Prerequisites:
# - macOS: fswatch installed (via brew install fswatch)
# - Linux: inotifywait installed (via apt install inotify-tools)
#
# Usage:
#   watch-files "cargo test"
#   watch-files "cargo build" "src/"
#
# Example:
#   # Watch for changes and run tests
#   watch-files "cargo test --workspace"
function watch-files
    set command $argv[1]
    set dir $argv[2]
    if test -z "$dir"
        set dir "."
    end

    set watcher ""
    if test (uname) = "Darwin"
        set watcher "fswatch"
    else
        set watcher "inotifywait"
    end

    if not command -v $watcher >/dev/null
        echo "Install $watcher first. Run ./bootstrap.sh to set up."
        return 1
    end

    while true
        echo "Watching $dir for changes..."

        if test (uname) = "Darwin"
            if not fswatch -r --exclude "target|.git" -1 $dir >/dev/null 2>&1
                # User pressed Ctrl+C, exit gracefully
                return
            end
        else
            if not inotifywait -r -e modify,create,delete,move --exclude "target|.git" $dir >/dev/null 2>&1
                # User pressed Ctrl+C, exit gracefully
                return
            end
        end

        echo "Running: $command"
        bash -c "$command"
    end
end

# Conditionally installs a tool if it's not already present on the system.
#
# This helper function checks if a tool exists in PATH and installs it
# if missing. Provides user feedback on installation status.
#
# Parameters:
# - tool: Name of the tool/binary to check for
# - cmd: Shell command to install the tool
#
# Features:
# - Idempotent - safe to call multiple times
# - Clear feedback on tool status
# - Uses bash for command execution
#
# Usage:
#   install_if_missing "bacon" "cargo install bacon"
#   install_if_missing "rg" "apt install ripgrep"
function install_if_missing
    set tool $argv[1]
    set cmd $argv[2]

    if not command -v $tool >/dev/null
        echo "Installing $tool..."
        bash -c "$cmd"
    else
        echo "✓ $tool installed"
    end
end

# Installs a cargo tool using cargo-binstall with fallback to cargo install.
#
# This function provides a unified interface for installing cargo tools with
# automatic detection of cargo-binstall availability for faster installations.
#
# Features:
# - Uses cargo-binstall when available for faster binary downloads
# - Falls back to cargo install with --locked flag for reproducible builds
# - Idempotent - skips installation if tool already exists
# - Consistent status messages for installation progress
#
# Prerequisites:
# - cargo must be available in PATH
# - cargo-binstall recommended for faster installations
#
# Usage:
#   install_cargo_tool "bacon"
#   install_cargo_tool "flamegraph"
function install_cargo_tool --argument-names tool_name
    if not command -v $tool_name >/dev/null
        echo "Installing $tool_name..."
        if command -v cargo-binstall >/dev/null
            # Use --force because binstall metadata may be stale: the binary
            # was deleted but binstall still thinks it's installed and skips
            # the download. Since we only reach here when command -v fails
            # (binary genuinely missing), --force is safe.
            cargo binstall -y --force $tool_name
        else
            # Fallback to cargo install with --locked for all tools
            cargo install $tool_name --locked
        end
    else
        echo "✓ $tool_name installed"
    end
end

# Detects and returns the appropriate package manager command for the current system.
#
# This function automatically identifies the system's package manager and returns
# the appropriate install command with sudo where needed.
#
# Supported systems:
# - macOS: brew install
# - Debian/Ubuntu: sudo apt install -y
# - Fedora/RHEL: sudo dnf install -y
# - Arch/Manjaro: sudo pacman -S --noconfirm
#
# Returns:
# - Package manager install command string
# - empty string if no supported package manager found
#
# Usage:
#   set pkg_mgr (get_package_manager)
#   if test -n "$pkg_mgr"
#       bash -c "$pkg_mgr neovim"
#   end
function get_package_manager
    if test (uname) = "Darwin"
        echo "brew install"
        return
    end

    if command -v apt-get >/dev/null
        echo "sudo apt install -y"
    else if command -v dnf >/dev/null
        echo "sudo dnf install -y"
    else if command -v pacman >/dev/null
        echo "sudo pacman -S --noconfirm"
    else
        echo ""
    end
end

# Ensures required build dependencies are installed for cargo operations.
#
# This function checks for critical dependencies needed by the project's
# cargo configuration (.cargo/config.toml uses clang + wild linker).
#
# Dependencies checked:
# - clang: Required as linker driver
# - wild: The actual linker (installed via cargo by bootstrap.sh)
#
# Installation strategy:
# - If anything is missing, run bootstrap.sh (handles all distros, idempotent)
# - bootstrap.sh uses install_if_missing, so it won't reinstall existing tools
# - Simple: one code path, predictable behavior
#
# Returns: 0 = all dependencies available, 1 = installation failed
#
# Usage:
#   ensure_build_dependencies || return 1
function ensure_build_dependencies
    # Check what's missing
    set -l missing_deps
    if not command -v clang >/dev/null
        set -a missing_deps "clang"
    end
    if not command -v wild >/dev/null
        set -a missing_deps "wild"
    end

    # If nothing is missing, we're good
    if test (count $missing_deps) -eq 0
        return 0
    end

    echo "🔧 Missing build dependencies: $missing_deps"
    echo "   Running bootstrap.sh to install..."

    # Run bootstrap.sh (idempotent - safe to run even if some deps exist)
    set -l script_dir (dirname (status filename))
    if not test -x "$script_dir/bootstrap.sh"
        echo "   ❌ bootstrap.sh not found at $script_dir/bootstrap.sh"
        return 1
    end

    if not bash "$script_dir/bootstrap.sh"
        echo "   ❌ bootstrap.sh failed"
        return 1
    end

    # Verify all dependencies are now available
    if not command -v clang >/dev/null
        echo "   ❌ clang still not available after bootstrap.sh"
        return 1
    end
    if not command -v wild >/dev/null
        echo "   ❌ wild still not available after bootstrap.sh"
        return 1
    end

    echo "   ✅ All build dependencies installed"
    return 0
end

# Executes a closure in a specific directory and safely returns to the original.
#
# This utility ensures that directory changes are properly managed, always
# returning to the original directory even if the closure fails.
#
# Parameters:
# - dir: Target directory to change to
# - command: Command to execute in the target directory
#
# Features:
# - Automatic directory restoration
# - Exception safety
#
# Usage:
#   run_in_directory "src" "ls -la"
function run_in_directory
    set dir $argv[1]
    set cmd $argv[2]

    set original_dir $PWD
    cd $dir
    if bash -c "$cmd"
        cd $original_dir
        return 0
    else
        set exit_code $status
        cd $original_dir
        return $exit_code
    end
end

# Discovers all Cargo projects in subdirectories of the current workspace.
#
# This function scans immediate subdirectories for Cargo.toml files to identify
# Rust projects in a workspace structure.
#
# Returns:
# - List of directory names containing Cargo.toml files
#
# Features:
# - Only checks immediate subdirectories (not recursive)
# - Filters out non-Cargo directories
# - Returns clean directory names
#
# Usage:
#   set projects (get_cargo_projects)
#   for project in $projects
#       cd $project
#       cargo build
#       cd ..
#   end
#
# Example workspace structure:
#   workspace/
#   ├── tui/Cargo.toml     -> returns "tui"
#   ├── cmdr/Cargo.toml    -> returns "cmdr"
#   └── docs/              -> ignored (no Cargo.toml)
function get_cargo_projects
    set projects
    for dir in */
        set dir_name (string replace -r '/$' '' $dir)
        if test -f "$dir_name/Cargo.toml"
            set projects $projects $dir_name
        end
    end
    printf '%s\n' $projects
end

# Runs a selected example with configurable build and logging options.
#
# This function provides an interactive fuzzy-search menu for example selection
# and handles the cargo execution with appropriate flags.
#
# Parameters:
# - options: List of available examples to choose from (passed as separate args)
# - release: Build in release mode if "true"
# - no_log: Disable logging output if "true"
#
# Features:
# - Fuzzy search for example selection
# - Graceful cancellation (Ctrl+C)
# - Debug/release mode support
# - Optional logging control
# - Detailed execution feedback
#
# Usage:
#   set examples (get_example_binaries)
#   run_example $examples "true" "false"  # Release mode with logging
#   run_example $examples "false" "true"  # Debug mode without logging
function run_example
    # Extract the last two arguments as flags
    set release $argv[-2]
    set no_log $argv[-1]

    # Everything except the last two arguments are the options
    set options $argv[1..-3]

    set selection (printf '%s\n' $options | fzf --prompt 'Select an example to run: ')
    set fzf_status $status

    if test $fzf_status -ne 0
        # User pressed Ctrl+C, exit gracefully without error
        return
    end

    if test -z "$selection"
        echo "No example selected."
    else
        set release_flag ""
        if test "$release" = "true"
            set release_flag "--release"
        end

        set log_flag ""
        if test "$no_log" = "true"
            set log_flag "--no-log"
        end

        echo "Running example with options: $options, release: $release, selection: $selection, log: $no_log"
        echo "Current working directory: $PWD"
        echo "cargo run -q $release_flag --example $selection -- $log_flag"
        cargo run --example $selection $release_flag -q -- $log_flag
    end
end

# Automatically discovers all available examples in the examples/ directory.
#
# This function scans the examples directory for both individual .rs files
# and subdirectories (for multi-file examples), returning a clean list of
# example names that can be run with cargo.
#
# Discovery process:
# 1. Finds all .rs files in examples/
# 2. Finds all subdirectories in examples/
# 3. Strips extensions and path prefixes
# 4. Returns folders first, then files
#
# Returns:
# - List of example names (without .rs extension or path)
#
# Example structure:
#   examples/
#   ├── simple.rs           -> "simple"
#   ├── complex.rs          -> "complex"
#   └── multi_file/         -> "multi_file"
#       └── main.rs
#
# Usage:
#   set examples (get_example_binaries)
#   # Returns: multi_file simple complex
function get_example_binaries
    set result

    # Get folders first
    if test -d examples
        for dir in examples/*/
            set dir_name (basename $dir)
            set result $result $dir_name
        end

        # Get .rs files
        for file in examples/*.rs
            if test -f $file
                set file_name (basename $file .rs)
                set result $result $file_name
            end
        end
    end

    printf '%s\n' $result
end

# Runs an example with flamegraph profiling and generates an interactive SVG visualization.
#
# This advanced profiling function creates detailed flame graphs for performance
# analysis. It uses a special 'profiling-detailed' Cargo profile that balances
# optimization with symbol visibility.
#
# Parameters:
# - options: List of available examples to profile (passed as separate args)
#
# Technical configuration:
# - Profile: profiling-detailed (custom profile with debug symbols)
# - Sampling: 99Hz to minimize overhead
# - Call graphs: Frame pointer-based with 8-level depth
# - Symbol handling: Forced frame pointers, readable symbol names
# - Inlining: Disabled to preserve function boundaries
#
# Kernel parameters temporarily modified:
# - kernel.perf_event_paranoid=-1 (allows CPU event access)
# - kernel.kptr_restrict=0 (allows kernel symbol access)
# These are reset after profiling for security
#
# Prerequisites:
# - perf (Linux profiling tool)
# - cargo-flamegraph
# - sudo access for kernel parameters
# - firefox-beta (optional, falls back to default browser)
#
# Output:
# - flamegraph.svg: Interactive SVG visualization
#   - Width represents time spent
#   - Height shows call stack depth
#   - Click to zoom into functions
#
# Process cleanup:
# - Automatically terminates lingering flamegraph/perf processes
# - Resets kernel security parameters
#
# Usage:
#   set examples (get_example_binaries)
#   run_example_with_flamegraph_profiling_svg $examples
#
# Note: The profiling-detailed profile must be defined in Cargo.toml
function run_example_with_flamegraph_profiling_svg
    set options $argv

    set selection (printf '%s\n' $options | fzf --prompt 'Select an example to run: ')
    set fzf_status $status

    if test $fzf_status -ne 0
        # User pressed Ctrl+C, exit gracefully
        return
    end

    if test -z "$selection"
        echo "No example selected."
    else
        echo "Running example with options: $options, selection: $selection"
        echo "Current working directory: $PWD"

        # Check if required tools are available
        if not command -v perf >/dev/null
            echo "Error: perf is not installed."
            echo "Please run the following from the repo root:"
            echo "  1. ./setup-dev-tools.sh      # Installs system tools like perf"
            echo "  2. fish run.fish install-cargo-tools  # Installs cargo tools like flamegraph"
            return
        end

        if not command -v cargo-flamegraph >/dev/null
            echo "Error: cargo-flamegraph is not installed."
            echo "Please run from the repo root:"
            echo "  fish run.fish install-cargo-tools"
            return
        end

        echo "cargo flamegraph --profile profiling-detailed --example $selection"

        # Change the kernel parameters to allow perf to access kernel symbols.
        sudo sysctl -w kernel.perf_event_paranoid=-1
        sudo sysctl -w kernel.kptr_restrict=0

        # Enhanced profiling with better symbol resolution using profiling-detailed profile
        # The profile settings handle debug symbols, LTO, and optimization level
        # RUSTFLAGS:
        # - force-frame-pointers=yes: Ensures stack traces work properly
        # - symbol-mangling-version=v0: Use more readable symbol names
        # cargo flamegraph options:
        # - --profile profiling-detailed: Use the detailed profiling profile
        # - --no-inline: Prevent inlining to preserve function boundaries
        # - -c "record -g --call-graph=fp,8 -F 99": Use frame pointer-based call graphs with 8 stack frame limit and 99Hz sampling
        echo "Using 'profiling-detailed' profile with --no-inline."
        env RUSTFLAGS="-C force-frame-pointers=yes -C symbol-mangling-version=v0" \
            cargo flamegraph \
            --profile profiling-detailed \
            --no-inline \
            -c "record -g --call-graph=fp,8 -F 99" \
            --example $selection

        # Find PIDs for cargo flamegraph
        set flamegraph_pids (pgrep -f "cargo flamegraph" 2>/dev/null || true)

        # Find PIDs for perf script
        set perf_script_pids (pgrep -f "perf script" 2>/dev/null || true)

        # Combine all found PIDs and get only the unique ones
        set all_pids $flamegraph_pids $perf_script_pids
        set all_pids (printf '%s\n' $all_pids | sort -u)

        if test (count $all_pids) -eq 0
            echo "No cargo flamegraph or perf script processes found to kill. 🧐"
        else
            echo "Attempting to terminate the following process IDs: "(string join ', ' $all_pids)" 🔪"
            for pid in $all_pids
                if test -n "$pid"
                    echo "  - Trying to gracefully kill PID: $pid (SIGTERM)"
                    sudo kill $pid 2>/dev/null || true
                end
            end
            echo "All targeted processes should now be terminated. ✅"
        end

        # Open the flamegraph in browser
        if not command -v firefox-beta >/dev/null
            echo "firefox-beta not found, using system default browser"
            xdg-open flamegraph.svg
        else
            firefox-beta --new-window flamegraph.svg
        end

        # Reset kernel parameters (optional but recommended for security)
        echo "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2 # Default paranoid level (often 2)
        sudo sysctl -w kernel.kptr_restrict=1      # Default restrict level (often 1)
        echo "Kernel parameters reset."
    end
end

# Generates collapsed stack traces (perf-folded format) for detailed performance analysis.
#
# This function profiles an example and outputs data in the collapsed stack format,
# which is more compact than SVG and suitable for further processing or integration
# with other profiling tools.
#
# Parameters:
# - options: List of available examples to profile (passed as separate args)
#
# Output format:
# Each line contains: stack;trace;functions sample_count
# Example:
#   main;process_data;calculate 150
#   main;render;draw_frame 89
#
# Advantages over SVG:
# - Much smaller file size (text vs. XML/SVG)
# - Easier to parse programmatically
# - Can be converted to various formats
# - Suitable for diff comparisons
# - Can be aggregated across multiple runs
#
# Technical details:
# - Uses same profiling configuration as SVG version
# - Profile: profiling-detailed
# - Sampling rate: 99Hz
# - Call graph: Frame pointer-based
#
# Processing pipeline:
# 1. Build with profiling-detailed profile
# 2. Run with perf record
# 3. Convert with perf script
# 4. Collapse with inferno-collapse-perf
#
# Prerequisites:
# - perf (Linux profiling tool)
# - inferno tools (via cargo install inferno)
# - sudo access for kernel parameters
#
# Output files:
# - flamegraph.perf-folded: Collapsed stack format
# - perf.data: Raw profiling data (can be reprocessed)
#
# File ownership:
# - Automatically fixes ownership of files created with sudo
#
# Usage:
#   set examples (get_example_binaries)
#   run_example_with_flamegraph_profiling_perf_fold $examples
#
# Post-processing:
#   # Convert to flamegraph later:
#   cat flamegraph.perf-folded | flamegraph > output.svg
#
#   # Diff two profiles:
#   inferno-diff-folded before.perf-folded after.perf-folded | flamegraph > diff.svg

# Runs benchmark with scripted input using expect for reproducible performance testing.
#
# This function executes a predefined sequence of keystrokes in a large terminal viewport
# to stress-test the rendering pipeline and generate consistent profiling data.
#
# Features:
# - Large viewport (220x60 = 13,200 cells) to exercise rendering
# - 25 scripted operations (typing, navigation, scrolling)
# - Fixed 30-second duration
# - Zero user interaction required
#
# Scripted actions:
# 1. Type text ("Hello World", "xyz", "Testing performance", etc.)
# 2. Cursor navigation (arrows, Home, End)
# 3. Screen operations (Ctrl+L redraw, PageUp/PageDown)
# 4. Mode changes (Esc)
# 5. Editing commands (Ctrl+K)
# 6. Idle rendering (18 seconds of continuous screen updates)
#
# Arguments:
#   $argv[1] - Binary path to execute under profiling
#
# Prerequisites:
#   - expect installed (via bootstrap.sh)
#   - sudo access for perf
#
# Output:
#   - perf.data created by perf record
function run_benchmark_with_scripted_input
    set binary_path $argv[1]

    # Check if expect is installed
    if not command -v expect >/dev/null
        echo "Error: expect is not installed (required for benchmark mode)."
        echo "Please run from the repo root:"
        echo "  ./bootstrap.sh"
        echo "Or manually:"
        echo "  Ubuntu/Debian: sudo apt install expect"
        echo "  Fedora/RHEL:   sudo dnf install expect"
        echo "  Arch:          sudo pacman -S expect"
        return
    end

    echo "Running perf record for ~8 seconds with continuous rendering workload..."
    echo "Sampling: 999 Hz for accurate hot path capture"
    echo "Viewport size: 60 rows x 220 columns (exercises rendering pipeline)"

    # Use expect to send scripted keystrokes, wrapped in timeout for safety
    timeout 10s sudo perf record -g --call-graph=fp,8 -F 999 -o perf.data -- \
        expect -c "
            set timeout 8

            # Set large terminal size to exercise rendering pipeline
            # (matches viewport from screenshot: ~220 cols x 60 rows)
            set stty_init \"rows 60 cols 220\"
            spawn $binary_path

            # Wait for app to initialize with large viewport
            sleep 1

            # tui_apps shows a menu - select ex_editor (option 3)
            send \"3\"
            sleep 0.2
            # Press Enter to select ex_editor
            send \"\r\"
            sleep 0.5

            # Now we're in ex_editor, start the benchmark sequence (25 operations)
            # Type \"Hello World\"
            send \"Hello World\"

            # Move cursor left 3 times
            send \"\x1b\[D\"
            send \"\x1b\[D\"
            send \"\x1b\[D\"

            # Type \"xyz\"
            send \"xyz\"

            # Press Enter
            send \"\r\"

            # Type \"Testing performance\"
            send \"Testing performance\"

            # Type Ctrl+L (show simple dialog)
            send \"\x0c\"

            # Type \"abc\"
            send \"abc\"

            # Press Esc (exit simple dialog)
            send \"\x1b\"

            # Type Ctrl+K (show complex dialog)
            send \"\x0b\"

            # Type \"def\"
            send \"def\"

            # Move cursor down 3 times
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"

            # Press Esc again (exit complex dialog)
            send \"\x1b\"

            # Press cursor down 8 times
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"

            # Press Home (move to beginning of line)
            send \"\x1b\[H\"

            # Press End (move to end of line)
            send \"\x1b\[F\"

            # Press PageDown 2 times
            send \"\x1b\[6~\"
            send \"\x1b\[6~\"

            # Press PageUp 2 times
            send \"\x1b\[5~\"
            send \"\x1b\[5~\"

            # Continuous typing to stress rendering (pangrams and lorem ipsum)
            send \"The quick brown fox jumps over the lazy dog. \"
            send \"Pack my box with five dozen liquor jugs. \"
            send \"How vexingly quick daft zebras jump! \"
            send \"Waltz, bad nymph, for quick jigs vex. \"
            send \"Sphinx of black quartz, judge my vow. \"
            send \"Lorem ipsum dolor sit amet, consectetur adipiscing elit. \"
            send \"Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. \"
            send \"Ut enim ad minim veniam, quis nostrud exercitation ullamco. \"
            send \"Duis aute irure dolor in reprehenderit in voluptate velit. \"

            # Rapid cursor movements (stress cursor positioning code)
            send \"\x1b\[A\"
            send \"\x1b\[A\"
            send \"\x1b\[A\"
            send \"\x1b\[A\"
            send \"\x1b\[A\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[B\"
            send \"\x1b\[C\"
            send \"\x1b\[C\"
            send \"\x1b\[C\"
            send \"\x1b\[C\"
            send \"\x1b\[C\"
            send \"\x1b\[D\"
            send \"\x1b\[D\"
            send \"\x1b\[D\"
            send \"\x1b\[D\"
            send \"\x1b\[D\"

            # More typing
            send \"Additional text to maximize rendering activity during profiling. \"
            send \"Every keystroke triggers a complete render cycle with ANSI sequences. \"
            send \"\r\"
            send \"This benchmark measures DirectToAnsi backend performance. \"
            send \"Stack-allocated number formatting should eliminate heap allocations. \"

            # Quit gracefully
            send \"q\"
            expect eof
        "
end

function run_example_with_flamegraph_profiling_perf_fold
    # Last argument is benchmark_mode (true/false)
    set benchmark_mode $argv[-1]
    # All other arguments are example options
    set options $argv[1..-2]

    # In benchmark mode, use tui_apps by default; otherwise use fzf selection
    if test "$benchmark_mode" = "true"
        set selection "tui_apps"
        echo (set_color cyan --bold)"=== BENCHMARK MODE ===" (set_color normal)
        echo "Using default example: $selection (ex_editor will be auto-selected)"
        echo "Duration: 30 seconds (timeout controlled)"
        echo "Output: flamegraph-benchmark.perf-folded"
        echo ""
    else
        set selection (printf '%s\n' $options | fzf --prompt 'Select an example to run: ')
        set fzf_status $status

        if test $fzf_status -ne 0
            # User pressed Ctrl+C, exit gracefully
            return
        end
    end

    if test -z "$selection"
        echo "No example selected."
    else
        echo "Running example to generate collapsed stacks: $selection"
        echo "Current working directory: $PWD"

        # Check if required tools are available
        if not command -v perf >/dev/null
            echo "Error: perf is not installed."
            echo "Please run the following from the repo root:"
            echo "  1. ./setup-dev-tools.sh      # Installs system tools like perf"
            echo "  2. fish run.fish install-cargo-tools  # Installs cargo tools like inferno"
            return
        end

        # Change the kernel parameters to allow perf to access kernel symbols
        sudo sysctl -w kernel.perf_event_paranoid=-1
        sudo sysctl -w kernel.kptr_restrict=0

        # Build the example with profiling-detailed profile and same RUSTFLAGS as SVG version
        echo "Building example with profiling-detailed profile..."
        env RUSTFLAGS="-C force-frame-pointers=yes -C symbol-mangling-version=v0" \
            cargo build --profile profiling-detailed --example $selection

        # Wait a moment for the build to complete
        sleep 1

        # Get the binary path - target is in parent directory
        set binary_path "../target/profiling-detailed/examples/$selection"

        # Check if the binary exists
        if not test -f $binary_path
            echo "Error: Binary not found at $binary_path"
            echo "Please ensure the example builds successfully."
            return
        end

        # Run perf record with same options as the SVG version
        # In benchmark mode, use expect for scripted input and timeout for fixed duration
        if test "$benchmark_mode" = "true"
            run_benchmark_with_scripted_input $binary_path
        else
            echo "Running perf record with enhanced symbol resolution..."
            sudo perf record -g --call-graph=fp,8 -F 99 -o perf.data -- $binary_path
        end

        # Fix ownership of perf.data files so they can be accessed without sudo
        set current_user $USER
        sudo chown "$current_user:$current_user" perf.data
        if test -f perf.data.old
            sudo chown "$current_user:$current_user" perf.data.old
        end

        # Fix ownership of log.txt if it was created (happens when running with sudo)
        if test -f log.txt
            sudo chown "$current_user:$current_user" log.txt
        end

        # Check if inferno-collapse-perf is available
        if not command -v inferno-collapse-perf >/dev/null
            echo "Error: inferno-collapse-perf is not installed."
            echo "Please run from the repo root:"
            echo "  fish run.fish install-cargo-tools"
            return
        end

        # Convert perf data to collapsed stacks format using inferno (comes with cargo flamegraph)
        # In benchmark mode, use a different output filename
        if test "$benchmark_mode" = "true"
            set output_file "flamegraph-benchmark.perf-folded"
        else
            set output_file "flamegraph.perf-folded"
        end

        echo "Converting to collapsed stacks format..."
        sudo perf script -f -i perf.data | inferno-collapse-perf > $output_file

        # Fix ownership of generated files
        sudo chown "$current_user:$current_user" $output_file

        # Show file size comparison
        set folded_size (wc -c < $output_file)
        echo "Generated $output_file: $folded_size bytes"

        # Count total samples (with error handling for empty files)
        if test $folded_size -gt 0
            set total_samples (awk '{sum += $NF} END {print sum}' $output_file)
            echo "Total samples: $total_samples"
        else
            echo "Warning: $output_file is empty. Check if perf recording was successful."
        end

        # Reset kernel parameters
        echo "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2
        sudo sysctl -w kernel.kptr_restrict=1
        echo "Kernel parameters reset."
    end
end

# ============================================================================
# Cross-Platform I/O Priority Wrapper
# ============================================================================

# Runs a command with I/O priority on Linux, or directly on macOS.
#
# Wraps a command with reduced CPU and I/O priority to keep the system
# responsive for interactive use (terminal input, IDE, desktop compositor).
#
# On Linux: Uses nice -n 10 (lower CPU priority) + ionice -c2 -n0 (higher I/O priority).
# On macOS: Uses nice -n 10 only. ionice doesn't exist on macOS.
#
# Why nice -n 10?
#   Without nice, cargo/rustdoc processes run at the same CPU priority (nice 0)
#   as your terminal emulator, shell, and desktop compositor. When all cores are
#   busy with equal-priority processes, the CFS scheduler gives your terminal
#   1/(N+1) of a timeslice — making keystrokes visibly laggy. nice -n 10 tells
#   the scheduler to prefer interactive processes over build processes whenever
#   they compete for the same core.
#
# Why ionice -c2 -n0?
#   Gives cargo the highest I/O priority within the best-effort class (no sudo
#   needed). This helps when reading source files from disk. Note: builds target
#   tmpfs ($CHECK_TARGET_DIR), so most write I/O bypasses the block layer
#   entirely — ionice mainly affects source file reads from the SSD.
#
# Parameters:
#   $argv: The command and its arguments to run
#
# Features:
# - Cross-platform: nice on all platforms, ionice added on Linux only
# - Transparent: Command output and exit codes pass through unchanged
#
# Usage:
#   ionice_wrapper cargo test --all-targets
#   ionice_wrapper cargo doc --no-deps
function ionice_wrapper
    if command -v ionice >/dev/null 2>&1
        # Linux: Lower CPU priority (nice 10) + higher I/O priority (ionice -c2 -n0).
        # nice ensures interactive processes (terminal, IDE) win CPU scheduling.
        # ionice ensures cargo still gets fast disk reads for source files.
        nice -n 10 ionice -c2 -n0 $argv
    else
        # macOS/other: nice is POSIX (available on macOS), ionice is not.
        # Still lower CPU priority so interactive processes win scheduling.
        nice -n 10 $argv
    end
end

# ============================================================================
# Cross-Platform Notification Utilities
# ============================================================================

# Sends a system notification with cross-platform support (macOS and Linux).
#
# On macOS: Uses osascript (AppleScript) to send notifications
# On Linux: Uses gdbus to send notifications with proper auto-dismiss support
# Falls back gracefully if notification tools are unavailable
#
# Parameters:
# - title: Notification title (required)
# - message: Notification message (required)
# - urgency: Urgency level - "normal", "critical", or "success" (optional, default: "normal")
#           Only affects Linux; macOS uses system defaults based on title
# - expire_ms: Auto-dismiss timeout in milliseconds (optional, Linux only)
#              If not specified, uses system default (notification stays until dismissed)
#              On GNOME: Uses gdbus + CloseNotification to force auto-dismiss
#              On other DEs: Falls back to notify-send --expire-time (may be ignored)
#
# Features:
# - Platform detection (Darwin for macOS, others for Linux)
# - Non-blocking: Runs in background so script doesn't wait
# - Error handling: Continues gracefully if notification system unavailable
# - Urgency levels: normal, critical, success (Linux only)
# - Auto-expiration: Works reliably on GNOME via gdbus CloseNotification
#
# Usage:
#   send_system_notification "Title" "Message"
#   send_system_notification "Error" "Something went wrong" "critical"
#   send_system_notification "Success" "Operation completed" "success"
#   send_system_notification "Watch" "Tests passed" "normal" 5000  # expires in 5s
function send_system_notification
    set -l title $argv[1]
    set -l message $argv[2]
    set -l urgency $argv[3]
    set -l expire_ms $argv[4]

    # Default to normal urgency if not specified
    if test -z "$urgency"
        set urgency "normal"
    end

    if test (uname) = "Darwin"
        # macOS: Use osascript (AppleScript)
        # osascript is built-in and always available
        # Note: macOS notifications auto-dismiss; no manual expiration control
        osascript -e "display notification \"$message\" with title \"$title\"" \
            2>/dev/null &
    else
        # Linux: Use gdbus for reliable auto-dismiss on GNOME
        # GNOME ignores notify-send's --expire-time, so we use gdbus to send
        # the notification and then explicitly close it after the timeout
        if test -n "$expire_ms" && command -v gdbus >/dev/null 2>&1
            # Convert expire_ms to seconds for sleep
            set -l expire_secs (math "$expire_ms / 1000")

            # Spawn a background fish child process that:
            #   1. Displays the notification (via gdbus)
            #   2. Waits for the expiration duration (sleep)
            #   3. Removes the notification (via CloseNotification)
            #
            # The `fish -c "..." &` pattern creates a detached child process.
            # Parent returns immediately (non-blocking), child handles the timed dismiss.
            #
            # Variable escaping:
            #   - $title, $message, $expire_ms, $expire_secs: NOT escaped, interpolated from parent
            #   - \$result, \$notify_id: Escaped, evaluated inside the child process
            fish -c "
                # Step 1: Display notification via gdbus and capture the notification ID
                set -l result (gdbus call --session \
                    --dest org.freedesktop.Notifications \
                    --object-path /org/freedesktop/Notifications \
                    --method org.freedesktop.Notifications.Notify \
                    'check.fish' 0 '' '$title' '$message' '[]' '{}' $expire_ms 2>/dev/null)

                # Extract notification ID from result like '(uint32 123,)'
                set -l notify_id (echo \$result | sed -n 's/.*uint32 \\([0-9]*\\).*/\\1/p')

                if test -n \"\$notify_id\"
                    # Step 2: Wait for the expiration duration
                    sleep $expire_secs

                    # Step 3: Remove/dismiss the notification
                    gdbus call --session \
                        --dest org.freedesktop.Notifications \
                        --object-path /org/freedesktop/Notifications \
                        --method org.freedesktop.Notifications.CloseNotification \
                        \"uint32 \$notify_id\" 2>/dev/null
                end
            " >/dev/null 2>&1 &
        else if command -v notify-send >/dev/null 2>&1
            # Fallback to notify-send (expiration may be ignored by some DEs)
            # Map urgency levels for notify-send compatibility
            set -l notify_urgency $urgency
            if test "$urgency" = "success"
                set notify_urgency "normal"
            end

            if test -n "$expire_ms"
                notify-send --urgency=$notify_urgency --expire-time=$expire_ms "$title" "$message" 2>/dev/null &
            else
                notify-send --urgency=$notify_urgency "$title" "$message" 2>/dev/null &
            end
        end
    end
end

# ============================================================================
# Rust Toolchain Management Utilities
# ============================================================================

# Reads the toolchain channel from rust-toolchain.toml
#
# Returns: toolchain string (e.g., "nightly-2025-10-15")
# Exit codes: 0=success, 1=error
#
# Usage:
#   set toolchain (read_toolchain_from_toml)
function read_toolchain_from_toml
    set -l toolchain_file "./rust-toolchain.toml"

    if not test -f $toolchain_file
        echo "ERROR: rust-toolchain.toml not found" >&2
        return 1
    end

    set -l channel_line (grep '^channel = ' $toolchain_file)

    if test -z "$channel_line"
        echo "ERROR: No channel entry found in rust-toolchain.toml" >&2
        return 1
    end

    set -l toolchain (echo $channel_line | sed -n 's/.*channel = "\([^"]*\)".*/\1/p')

    if test -z "$toolchain"
        echo "ERROR: Failed to parse channel value" >&2
        return 1
    end

    echo $toolchain
    return 0
end

# Checks if a toolchain is installed via rustup
#
# Usage: is_toolchain_installed "nightly-2025-10-15"
function is_toolchain_installed
    set -l toolchain $argv[1]
    rustup toolchain list | grep -q "^$toolchain"
    return $status
end

# Checks if a component is installed for a toolchain
#
# Usage: is_component_installed "nightly-2025-10-15" "rust-analyzer"
function is_component_installed
    set -l toolchain $argv[1]
    set -l component $argv[2]
    rustup component list --toolchain $toolchain --installed 2>/dev/null | grep -q "^$component"
    return $status
end

# Updates the channel value in rust-toolchain.toml
#
# Usage: set_toolchain_in_toml "nightly-2025-10-15"
function set_toolchain_in_toml
    set -l toolchain $argv[1]
    set -l toolchain_file "./rust-toolchain.toml"

    # Replace the channel line (cross-platform: macOS uses BSD sed, Linux uses GNU sed)
    if test (uname) = "Darwin"
        # macOS: BSD sed requires -i '' for in-place without backup
        sed -i '' "s/^channel = .*/channel = \"$toolchain\"/" $toolchain_file
    else
        # Linux: GNU sed works with -i directly
        sed -i "s/^channel = .*/channel = \"$toolchain\"/" $toolchain_file
    end
    return $status
end

# Installs the Windows cross-compilation target for verifying platform-specific code.
#
# This target allows checking that #[cfg(unix)] and #[cfg(not(unix))] gates compile
# correctly on Windows without needing a full cross-compiler toolchain (mingw-w64).
# Uses `cargo rustc --target x86_64-pc-windows-gnu -- --emit=metadata` for verification.
#
# Features:
# - Idempotent: Safe to call multiple times
# - Non-blocking: Continues with warning if installation fails
# - Logs success/failure status
#
# Prerequisites:
# - rustup must be available in PATH
#
# Usage:
#   install_windows_target
#   # Then verify: cargo rustc -p <crate> --target x86_64-pc-windows-gnu -- --emit=metadata
function install_windows_target
    set -l target "x86_64-pc-windows-gnu"

    if rustup target list --installed | grep -q $target
        echo "✅ $target target already installed"
    else
        echo "Installing Windows cross-compilation target..."
        if rustup target add $target
            echo "✅ $target target installed"
        else
            echo "⚠️  Failed to install Windows target (non-critical, continuing)"
        end
    end
    return 0
end

# ============================================================================
# Toolchain Script Locking Utilities
# ============================================================================

# Gets the age of the lock in seconds by reading the timestamp file.
#
# Returns: age in seconds, or -1 if timestamp file is missing/invalid
#
# Usage:
#   set age (get_lock_age_seconds)
#   if test $age -gt 600  # 10 minutes
#       echo "Lock is stale"
#   end
function get_lock_age_seconds
    set -l lock_dir "./rust-toolchain-script.lock"
    set -l timestamp_file "$lock_dir/timestamp"

    # Check if timestamp file exists
    if not test -f $timestamp_file
        echo "-1"
        return
    end

    # Read stored timestamp
    set -l stored_time (cat $timestamp_file 2>/dev/null)
    if test -z "$stored_time"
        echo "-1"
        return
    end

    # Get current time
    set -l current_time (date +%s)

    # Calculate age (current - stored)
    set -l age (math "$current_time - $stored_time")
    echo $age
end

# Acquires an exclusive lock for toolchain operations to prevent concurrent conflicts.
#
# This function ensures that only one toolchain operation runs at a time by using
# mkdir (atomic directory creation). mkdir is atomic: check-and-create happens in ONE
# indivisible kernel operation, preventing all race conditions.
#
# Why mkdir for locking:
# - mkdir is ATOMIC: check-if-exists AND create happen in ONE kernel operation
# - Only ONE process can successfully create a directory with a given name
# - Stale lock detection: Automatically removes locks older than 10 minutes (600 seconds)
# - Best practice for shell script locking (used by systemd, init systems, etc.)
# - Works reliably across all Unix systems
#
# Lock mechanism with stale lock detection:
# 1. Attempt to create lock directory with mkdir (fails if exists)
# 2. If mkdir succeeds:
#    - Write current timestamp to lock_dir/timestamp
#    - This process has exclusive lock
# 3. If mkdir fails (lock exists):
#    - Check age of lock via timestamp file
#    - If age > 10 minutes (600 seconds): Remove stale lock and retry once
#    - If age <= 10 minutes: Lock is active, return failure
# 4. Lock cleanup: Process removes directory (including timestamp) when done
#
# Returns: 0 = lock acquired, 1 = lock held by another operation
#
# Usage:
#   acquire_toolchain_lock
function acquire_toolchain_lock
    set -l lock_dir "./rust-toolchain-script.lock"
    set -l timestamp_file "$lock_dir/timestamp"
    set -l stale_threshold_seconds 600  # 10 minutes

    # Try to create lock directory (mkdir is atomic)
    # Only ONE process can successfully create it - all others will fail
    # This is the standard Unix pattern for shell script locking
    if mkdir $lock_dir 2>/dev/null
        # Successfully acquired lock - write timestamp
        date +%s > $timestamp_file
        echo "✅ Acquired toolchain operation lock" >&2
        return 0
    else
        # Directory already exists - check if it's a stale lock
        set -l lock_age (get_lock_age_seconds)

        if test $lock_age -eq -1
            # Can't determine age (missing/invalid timestamp)
            echo "🔒 Another toolchain operation in progress (unknown age)" >&2
            return 1
        else if test $lock_age -gt $stale_threshold_seconds
            # Stale lock detected - clean up and retry
            set -l age_minutes (math "round($lock_age / 60)")
            echo "🧹 Removing stale lock (age: $age_minutes minutes)" >&2
            command rm -rf $lock_dir 2>/dev/null

            # Retry lock acquisition once (avoid infinite recursion)
            if mkdir $lock_dir 2>/dev/null
                date +%s > $timestamp_file
                echo "✅ Acquired toolchain operation lock after stale lock cleanup" >&2
                return 0
            else
                # Another process grabbed the lock during cleanup
                echo "🔒 Another toolchain operation acquired lock during cleanup" >&2
                return 1
            end
        else
            # Active lock - show age for transparency
            set -l age_minutes (math "round($lock_age / 60)")
            echo "🔒 Another toolchain operation in progress (age: $age_minutes minutes)" >&2
            return 1
        end
    end
end

# Releases the toolchain operation lock.
#
# Removes the lock directory that was created by acquire_toolchain_lock.
# This allows other waiting processes to acquire the lock.
#
# Safe to call (idempotent) - won't error if lock doesn't exist.
#
# Usage:
#   release_toolchain_lock
function release_toolchain_lock
    set -l lock_dir "./rust-toolchain-script.lock"

    # Remove the lock directory (including timestamp file) if it exists
    # Using rm -rf to handle directory with contents
    if test -d $lock_dir
        command rm -rf $lock_dir 2>/dev/null
        echo "🔓 Released toolchain operation lock" >&2
    end
end

# Returns current local time in HH:MM:SS AM/PM format.
#
# Useful for timestamping log output in scripts that run for extended periods
# (like watch modes) so users can correlate events with wall-clock time.
#
# Output format: "HH:MM:SS AM" or "HH:MM:SS PM" (12-hour clock)
#
# Usage:
#   echo "["(timestamp)"] Starting build..."
#   # Output: [02:34:56 PM] Starting build...
function timestamp
    date "+%I:%M:%S %p"
end

# Formats a duration in seconds to a human-readable string.
#
# Converts raw seconds into a compact, readable format that scales appropriately:
# - Sub-second: "0.5s"
# - Seconds only: "5s"
# - Minutes and seconds: "2m 30s"
# - Hours, minutes, seconds: "1h 5m 30s"
#
# Parameters:
#   $argv[1]: Duration in seconds (can be decimal, e.g., "1.5")
#
# Usage:
#   format_duration 90    # Output: "1m 30s"
#   format_duration 3661  # Output: "1h 1m 1s"
#   format_duration 0.5   # Output: "0.5s"
function format_duration
    set -l total_seconds $argv[1]

    # Handle decimal seconds (sub-second precision)
    set -l int_seconds (math "floor($total_seconds)")

    if test $int_seconds -lt 1
        # Sub-second: show decimal
        printf "%.1fs" $total_seconds
        return
    end

    set -l hours (math "floor($int_seconds / 3600)")
    set -l minutes (math "floor(($int_seconds % 3600) / 60)")
    set -l seconds (math "$int_seconds % 60")

    if test $hours -gt 0
        echo "$hours"h "$minutes"m "$seconds"s
    else if test $minutes -gt 0
        echo "$minutes"m "$seconds"s
    else
        echo "$seconds"s
    end
end

# ============================================================================
# Build Config Change Detection
# ============================================================================

# Checks if build config files have changed since last build.
#
# Computes a SHA256 hash of key config files and compares it to a stored hash
# in the target directory. If the hash differs, cleans the target directory
# to avoid stale artifact issues.
#
# This handles scenarios like:
# - Toggling incremental compilation on/off
# - Changing optimization levels or profiles
# - Updating the Rust toolchain version
# - Modifying cargo build flags
# - Adding/removing dependencies in any workspace crate
#
# Parameters:
#   $argv[1]: Target directory to clean on change (e.g., $CHECK_TARGET_DIR)
#   $argv[2...]: Config files to watch (e.g., Cargo.toml rust-toolchain.toml)
#
# The hash is stored in $CHECK_BUILD_CONFIG_HASH_FILE (private tree) and updated
# after each check. Cleaning the target directory is a side effect.
#
# Returns: 0 always
#
# Usage:
#   check_config_changed $CHECK_TARGET_DIR $CONFIG_FILES_TO_WATCH
function check_config_changed
    set -l target_dir $argv[1]
    set -l config_files $argv[2..-1]

    # Compute hash of all config files that affect builds
    # Using cat with 2>/dev/null to handle missing files gracefully
    # SHA256 is preferred over MD5 as a modern, more robust hash algorithm
    set -l config_hash (cat $config_files 2>/dev/null | sha256sum | cut -d' ' -f1)
    set -l hash_file $CHECK_BUILD_CONFIG_HASH_FILE

    # Ensure hash file directory exists (private tree root)
    mkdir -p (dirname $hash_file)

    # Check if hash file exists and compare
    if test -f $hash_file
        set -l stored_hash (cat $hash_file)
        if test "$config_hash" != "$stored_hash"
            echo ""
            set_color yellow
            echo "⚠️  Build config changed (Cargo.toml, rust-toolchain.toml, or .cargo/config.toml)"
            echo "🧹 Cleaning $target_dir to avoid stale artifacts..."
            set_color normal
            if test -d "$target_dir"
                command rm -rf "$target_dir"
            end
            echo $config_hash > $hash_file
            echo ""
        end
    else
        # No hash file yet - create it (first run or after manual clean)
        echo $config_hash > $hash_file
    end

    return 0
end

# ============================================================================
# Logging Utilities
# ============================================================================

# Prints a message to stdout AND appends it to a log file.
#
# This function provides dual-output logging: messages appear on the terminal
# for immediate feedback AND are persisted to a log file for later debugging.
# Uses tee internally to avoid duplicating output logic.
#
# Parameters:
#   $argv[1]: Log file path (will be created if doesn't exist, appended if exists)
#   $argv[2...]: Message to print (all remaining arguments joined with spaces)
#
# Features:
# - Creates parent directories if needed
# - Appends to existing log file (doesn't overwrite)
# - Handles multi-word messages correctly
# - Preserves exit status
#
# Usage:
#   log_and_print /tmp/my.log "Starting build..."
#   log_and_print $LOG_FILE "["(timestamp)"] ✅ Build complete!"
#   log_and_print /tmp/my.log "Error:" $error_message
#
# Example:
#   set -g LOG_FILE $CHECK_LOG_FILE
#   log_and_print $LOG_FILE "["(timestamp)"] 🔨 Starting doc build..."
#   # Output appears on terminal AND is appended to $CHECK_LOG_FILE
function log_and_print
    set -l log_file $argv[1]
    set -l message $argv[2..-1]

    # Ensure log directory exists
    mkdir -p (dirname $log_file)

    # Print to stdout AND append to log file
    echo $message | tee -a $log_file
end

# ============================================================================
# Watch Mode Doc Build Utilities
# ============================================================================
# Functions for the --watch-doc mode in check.fish.
# These are extracted for reusability and to simplify the forked subprocess logic.
#
# ARCHITECTURE OVERVIEW
# =====================
#
# The watch-doc mode provides fast feedback while eventually reaching a
# consistent state with correct cross-crate links. It uses a two-tier build
# system with catch-up mechanisms for changes that occur during builds.
#
# Build Types:
# - Quick build: `cargo doc --workspace --no-deps` (~5-7s)
#   * Fast feedback for doc changes
#   * Broken cross-crate links (e.g., links to crossterm, tokio don't work)
#   * Acceptable trade-off: user sees changes quickly, links fixed by full build
#
# - Full build: `cargo doc` (~90s)
#   * Builds all workspace crates AND all dependencies
#   * All cross-crate links are correct
#   * Slow, so runs in background while user continues editing
#
# WHY BROKEN LINKS IN QUICK BUILD?
# ================================
# The `--no-deps` flag tells rustdoc not to document dependencies. This makes
# the build fast, but rustdoc can't generate correct links to external crates
# (crossterm, tokio, etc.) because it doesn't know they exist. The resulting
# HTML contains broken relative links like `href="crossterm"` instead of
# correct links like `href="../crossterm/index.html"`.
#
# The full build (without --no-deps) documents everything, so rustdoc knows
# about all crates and generates correct cross-crate links.
#
# ARCHITECTURE DIAGRAM
# ====================
#
#   File change detected
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Quick build (~5-7s)                         │
#   │ • cargo doc --workspace --no-deps           │
#   │ • Fast feedback, broken links OK            │
#   └─────────────────────────────────────────────┘
#       │
#       ├──► Catch-up check (if changes during quick build)
#       │         └──► Quick build → forks Full build
#       │
#       ▼
#   ┌─────────────────────────────────────────────┐
#   │ Full build (~5-90s) [FORKED TO BACKGROUND]  │
#   │ • cargo doc (dep-doc caching: --no-deps if  │
#   │   Cargo.lock + rust-toolchain.toml unchanged)│
#   │ • Fixes all cross-crate links               │
#   └─────────────────────────────────────────────┘
#       │
#       └──► Catch-up check (if changes during full build)
#                 │
#                 ▼
#            Quick build (~5-7s) → forks Full build (~5-90s)
#                 │                      │
#                 ▼                      ▼
#            [fast feedback]      [fixes links eventually]
#
# EVENTUAL CONSISTENCY MODEL
# ==========================
# The system forms a cycle: quick → full → (if changes) → quick → full → ...
#
# Termination condition: When no source files change during a build, the cycle
# stops. At this point:
# - The full build has completed without interruption
# - All cross-crate links are correct
# - The docs reflect the latest source code
#
# This provides the best of both worlds:
# - User always gets fast feedback (~5-7s) on their doc changes
# - Links eventually become correct (after full build completes without changes)
#
# BLIND SPOTS
# ===========
# While a build is running, inotifywait is not watching for changes. This
# creates "blind spots" where file changes could be missed:
#
# 1. Quick build blind spot (~5-7s):
#    - Handled by catch-up check using `has_source_changes_since`
#    - If changes detected: runs another quick build, which forks a full build
#
# 2. Full build blind spot (~90s):
#    - Handled by catch-up check in `run_full_doc_build_task`
#    - If changes detected: runs quick build (fast feedback), forks another full build
#
# STAGING DIRECTORIES
# ===================
# Quick and full builds use separate staging directories to avoid conflicts:
# - Quick staging: $CHECK_TARGET_DIR_DOC_STAGING_QUICK
# - Full staging:  $CHECK_TARGET_DIR_DOC_STAGING_FULL
# - Serving dir:   $CHECK_TARGET_DIR/doc (what browser loads)
#
# Both sync to the same serving directory using rsync. The staging approach
# prevents users from seeing incomplete docs during a build.
#
# RUST MIGRATION NOTES
# ====================
# When migrating to Rust, consider:
# - Use `std::process::Command` for cargo doc invocations
# - Use `notify` crate for file watching (cross-platform, unlike inotifywait)
# - Use `tokio::spawn` or threads for background full builds
# - The catch-up detection can use `std::fs::metadata().modified()`
# - Consider a message-passing architecture (channels) instead of forking
# ============================================================================

# ============================================================================
# Global Variables (Shared)
# ============================================================================

# Project name/path for notifications. Shortened version of PWD (e.g., ~/g/roc).
# Fallback if not already set by a configuration module (like check_constants.fish).
if not set -q WORKSPACE_NAME
    set -g WORKSPACE_NAME (prompt_pwd)
end

# Source directories to watch for changes and to check for modifications.
#
# This constant defines the directories containing Rust source files that:
# 1. inotifywait monitors for file changes (triggers build cycles)
# 2. Catch-up detection scans for recently modified files
#
# Keeping this as a single source of truth ensures consistency between
# the watch loop and catch-up detection.
#
# Rust migration: This would be a const Vec<&str> or similar.
set -g SRC_DIRS cmdr/src analytics_schema/src tui/src

# Checks if source files changed since a given epoch timestamp.
#
# PURPOSE:
# This implements "blind spot" detection. While a build is running, inotifywait
# is not watching for changes. If a user saves a file during this window, the
# change would be missed. After each build, we use `find -newermt` to scan for
# files modified since the build started.
#
# ALGORITHM:
# 1. Receive epoch timestamp from when build started
# 2. Use `find` with `-newermt "@$epoch"` to find files modified after that time
# 3. Return 0 (true) if any files found, 1 (false) otherwise
#
# Parameters:
#   $argv[1]: Epoch timestamp (seconds since 1970-01-01, from `date +%s`)
#
# Returns:
#   0 = Changes detected (files were modified during the build)
#   1 = No changes (safe to consider build results current)
#
# Usage:
#   set -l build_start (date +%s)
#   # ... build runs for some time ...
#   if has_source_changes_since $build_start
#       echo "Files changed during build - need catch-up!"
#   end
#
# Rust migration: Use std::fs::metadata().modified() and compare SystemTime.
function has_source_changes_since
    set -l since_epoch $argv[1]
    set -l changed (find $SRC_DIRS -type f -name "*.rs" -newermt "@$since_epoch" 2>/dev/null)
    test (count $changed) -gt 0
end

# ============================================================================
# Centralized cargo doc Runner & Dep-doc Caching
# ============================================================================
#
# ARCHITECTURE NOTE (for future Rust rewrite):
# run_cargo_doc is the single source of truth for invoking `cargo doc`.
# All doc build paths (--doc, --quick-doc, --full, --watch-doc) route through
# here. Custom CSS is applied via RUSTDOCFLAGS env var with an absolute path
# (not .cargo/config.toml) because rustdoc resolves --extend-css relative to
# each crate's source directory - which fails for dependency crates outside
# the workspace root.
#
# Dep-doc caching: Hash of Cargo.lock + rust-toolchain.toml, stored in the
# FULL staging directory (tmpfs at $CHECK_TARGET_DIR_DOC_STAGING_FULL).
# By storing it in the staging directory, it survives when the serving directory
# is wiped by check_config_changed (which reacts to any Cargo.toml change).
# The cache only invalidates when dependencies (Cargo.lock) or the toolchain
# change. It self-heals on reboot (tmpfs wiped) or cargo clean (dir gone).
#
# DEP_DOCS_WERE_CACHED global variable pattern: check_docs_full sets this so
# callers (check.fish --doc and --full cases) know which sync mode to use.
# Exit codes don't work here because check_docs_full's exit code flows through
# run_check_with_recovery (maps codes to 0=success, 1=failure, 2=recoverable)
# and run_full_checks (aggregates 7 check results into a single code). A global
# variable sidesteps this cleanly. Rust rewrite should use a proper return type.
#
# Used by: check_docs_quick, check_docs_full, build_and_sync_quick_docs,
#          build_and_sync_full_docs
# ============================================================================

# Central cargo doc runner with custom CSS support.
#
# Always sets RUSTDOCFLAGS with absolute path for monospace font CSS.
# Accepts optional --timeout=SECS for builds that need time limits.
# All other arguments pass through to `cargo doc`.
function run_cargo_doc
    set -lx RUSTDOCFLAGS "--extend-css $PWD/docs/rustdoc/custom.css"

    set -l timeout_secs 0
    set -l cargo_args
    for arg in $argv
        if string match -q -- '--timeout=*' $arg
            set timeout_secs (string replace -- '--timeout=' '' $arg)
        else
            set -a cargo_args $arg
        end
    end

    if test "$timeout_secs" -gt 0
        ionice_wrapper timeout --foreground $timeout_secs cargo doc $cargo_args
    else
        ionice_wrapper cargo doc $cargo_args
    end
end

# Check if dependency docs are still valid (no dep changes since last full build).
#
# CACHE INVALIDATION:
# Hash is based on Cargo.lock + rust-toolchain.toml. These two files capture
# all scenarios that require rebuilding dependency docs:
#   - Cargo.lock: dependency version changes, additions, removals
#   - rust-toolchain.toml: toolchain changes (doc format can differ between nightlies)
#
# Hash file lives in the staging dir (e.g. $CHECK_TARGET_DIR_DOC_STAGING_FULL/),
# so it survives when the serving dir ($CHECK_TARGET_DIR) is wiped by
# check_config_changed.
#
# Parameters:
#   $argv[1]: staging_dir (e.g., $CHECK_TARGET_DIR_DOC_STAGING_FULL)
#
# Returns: 0 if dep docs are current, 1 if they need rebuilding.
function dep_docs_are_current
    set -l staging_dir $argv[1]
    set -l hash_file $staging_dir/.dep-docs-hash
    if not test -f $hash_file
        return 1
    end
    set -l current_hash (cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1)
    set -l stored_hash (cat $hash_file)
    test "$current_hash" = "$stored_hash"
end

# Update dep docs hash after successful full build.
#
# Parameters:
#   $argv[1]: staging_dir (e.g., $CHECK_TARGET_DIR_DOC_STAGING_FULL)
function update_dep_docs_hash
    set -l staging_dir $argv[1]
    cat Cargo.lock rust-toolchain.toml 2>/dev/null | md5sum | cut -d' ' -f1 > $staging_dir/.dep-docs-hash
end

# Builds quick docs (workspace-wide) and syncs to serving directory.
#
# PURPOSE:
# Provides fast feedback (~5-7s) when a user modifies documentation. This is
# the first build that runs after a file change, giving the user immediate
# visual feedback on their doc changes.
#
# TRADE-OFF: SPEED VS LINK CORRECTNESS
# This build uses `cargo doc --workspace --no-deps` which is fast but produces
# no external crate links. For example:
#   - Broken: href="crossterm" (relative path that doesn't exist)
#   - Correct: href="../crossterm/index.html" (only from full build)
#
# This is acceptable because:
# 1. Every quick build automatically forks a full build
# 2. The full build will fix all links when it completes
# 3. User gets fast feedback on their doc content changes
# 4. Links become correct after ~90s (full build duration)
#
# STAGING DIRECTORY:
# We build into a staging directory, then rsync to serving. This prevents
# users from seeing incomplete/broken docs during the build process.
#
# Parameters:
#   $argv[1]: Staging directory (e.g., $CHECK_TARGET_DIR_DOC_STAGING_QUICK)
#   $argv[2]: Serving directory (e.g., $CHECK_TARGET_DIR/doc)
#
# Returns: 0 on success, non-zero on failure
#
# Usage:
#   build_and_sync_quick_docs $CHECK_TARGET_DIR_DOC_STAGING_QUICK $CHECK_TARGET_DIR
#
# Rust migration: Use std::process::Command to run cargo doc, then use the
# `fs_extra` or `walkdir` crate for the sync operation.
function build_and_sync_quick_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    set -lx CARGO_TARGET_DIR $staging_dir
    # Fast mode: --workspace --no-deps (~5-7s)
    # No external crate links - full build will fix them soon
    run_cargo_doc --workspace --no-deps > /dev/null 2>&1
    set -l result $status

    if test $result -eq 0
        mkdir -p "$serving_dir/doc"
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
    end

    return $result
end


# Builds full docs (with dep-doc caching) and syncs to serving directory.
# Used by --watch-doc's forked background process for correct cross-crate links.
#
# Dep-doc caching: Checks hash of Cargo.lock + rust-toolchain.toml. If unchanged,
# builds only workspace crates (--no-deps).
#
# Sync behavior: This function always performs a full rsync from staging to
# serving. This is crucial because even if the cache is hit (meaning dependency
# artifacts are already in the staging directory), the serving directory may
# have been wiped by check_config_changed. The rsync ensures all docs (including
# dependencies) are restored from the staging cache to the serving directory.
#
# Does NOT use the DEP_DOCS_WERE_CACHED global (unlike check_docs_full) because
# this function handles its own sync internally - rsync -a without --delete is
# safe regardless of whether deps were rebuilt or cached.
function build_and_sync_full_docs
    set -l staging_dir $argv[1]
    set -l serving_dir $argv[2]

    set -lx CARGO_TARGET_DIR $staging_dir
    if dep_docs_are_current $staging_dir
        run_cargo_doc --no-deps > /dev/null 2>&1
    else
        run_cargo_doc > /dev/null 2>&1
    end
    set -l result $status

    if test $result -eq 0
        # Ensure serving doc directory exists
        mkdir -p "$serving_dir/doc"
        # Sync with -a (archive mode preserves permissions, timestamps)
        # Note: We don't use --delete here because the quick build patch
        # that follows will overlay our crates' docs anyway
        rsync -a "$staging_dir/doc/" "$serving_dir/doc/"
        # Update hash only when deps were actually rebuilt
        if not dep_docs_are_current $staging_dir
            update_dep_docs_hash $staging_dir
        end
    end

    return $result
end

# Waits for file changes using platform-appropriate file watcher.
#
# On Linux: Uses inotifywait (efficient kernel-level inotify subsystem)
# On macOS: Uses fswatch (FSEvents backend)
#
# Common options used:
# - Recursive watching
# - Quiet output
# - Events: modify, create, delete, move/rename
#
# Parameters:
#   $argv[1]: Timeout in seconds (0 for no timeout, waits forever)
#   $argv[2...]: Directories/files to watch
#
# Returns:
#   0 = File change detected
#   1 = Error
#   2 = Timeout expired (no changes within timeout period)
#
# Usage:
#   # Wait up to 10 seconds for changes
#   wait_for_file_changes 10 $watch_dirs
#   if test $status -eq 0
#       echo "Change detected!"
#   else if test $status -eq 2
#       echo "Timeout - no changes"
#   end
#
#   # Wait forever for first change
#   wait_for_file_changes 0 $watch_dirs
function wait_for_file_changes
    set -l timeout_secs $argv[1]
    set -l watch_targets $argv[2..-1]

    # Use inotifywait on Linux, fswatch on macOS
    if command -v inotifywait >/dev/null 2>&1
        # Linux: inotifywait has built-in timeout support
        if test $timeout_secs -eq 0
            inotifywait -q -r -e modify,create,delete,move \
                --format '%w%f' $watch_targets >/dev/null 2>&1
        else
            inotifywait -q -r -t $timeout_secs -e modify,create,delete,move \
                --format '%w%f' $watch_targets >/dev/null 2>&1
        end
        return $status
    else if command -v fswatch >/dev/null 2>&1
        # macOS: fswatch doesn't have built-in timeout, use background process pattern
        if test $timeout_secs -eq 0
            # No timeout - wait forever for first event
            fswatch -1 --recursive \
                --event Created --event Updated --event Removed --event Renamed \
                $watch_targets >/dev/null 2>&1
            return $status
        else
            # With timeout: run fswatch directly (no sh wrapper to avoid orphan processes)
            # Create a temp file - fswatch writes detected file path to it
            set -l signal_file (mktemp)

            # Run fswatch directly so $last_pid is actually fswatch's PID
            # fswatch -1 outputs the changed file path and exits after first event
            fswatch -1 --recursive \
                --event Created --event Updated --event Removed --event Renamed \
                $watch_targets >$signal_file 2>/dev/null &
            set -l fswatch_pid $last_pid

            # Wait for timeout or event
            set -l elapsed 0
            while test $elapsed -lt $timeout_secs
                sleep 0.1
                set elapsed (math "$elapsed + 0.1")

                # Check if fswatch detected something (wrote to signal file)
                if test -s $signal_file
                    command rm -f $signal_file
                    return 0  # Change detected
                end

                # Check if fswatch is still running
                if not kill -0 $fswatch_pid 2>/dev/null
                    # fswatch exited - check if it signaled
                    if test -s $signal_file
                        command rm -f $signal_file
                        return 0  # Change detected
                    end
                    command rm -f $signal_file
                    return 1  # Error - fswatch died unexpectedly
                end
            end

            # Timeout expired - kill fswatch directly (no wrapper = no orphans)
            kill $fswatch_pid 2>/dev/null
            wait $fswatch_pid 2>/dev/null
            command rm -f $signal_file
            return 2  # Timeout
        end
    else
        echo "Error: No file watcher available (need inotifywait or fswatch)" >&2
        return 1
    end
end

# Runs the full doc build workflow as a background task.
#
# PURPOSE:
# This is the main orchestrator for full builds. It runs in a forked background
# process and handles the complete lifecycle:
# 1. Build full docs (with all dependencies)
# 2. Sync to serving directory
# 3. Check for changes that occurred during the ~90s build ("blind spot")
# 4. If changes: run catch-up quick build, then fork another full build
# 5. Send desktop notifications at each stage
#
# EVENTUAL CONSISTENCY ALGORITHM:
# ===============================
# This function implements the "eventual consistency" model:
#
#   run_full_doc_build_task:
#       1. Record start time
#       2. Run full build (~90s)
#       3. Sync to serving (links now correct)
#       4. Check: did files change during build?
#          - NO:  Done! Docs are current and links are correct.
#          - YES: Run quick build (fast feedback, ~5-7s)
#                 Fork ANOTHER full build (recursive call)
#                 This new full build will eventually fix links.
#
# The recursion terminates when a full build completes without any file
# changes during its execution. At that point:
# - All docs reflect the latest source code
# - All cross-crate links are correct
#
# WHY QUICK BUILD FOR CATCH-UP (NOT FULL)?
# ========================================
# When changes are detected after a full build, we run a quick build first
# because the user wants fast feedback. Yes, this temporarily breaks links,
# but:
# 1. The user sees their changes immediately (~5-7s)
# 2. We immediately fork another full build to fix links
# 3. Links become correct again after ~90s
#
# The alternative (waiting for another full build) would mean the user waits
# ~90s to see their changes, which defeats the purpose of watch mode.
#
# FORKING MODEL:
# ==============
# This function is designed to be called via `fish -c "..." &` which creates
# a completely independent background process. The parent (watch loop) returns
# immediately and goes back to watching for file changes.
#
# When catch-up detects changes, it forks ANOTHER instance of this function,
# creating a chain: full → (changes) → quick + fork full → (changes) → ...
#
# Parameters:
#   $argv[1]: Full build staging directory
#   $argv[2]: Quick build staging directory
#   $argv[3]: Serving directory (where browser loads from)
#   $argv[4]: Log file path
#   $argv[5]: Notification expire time in milliseconds
#
# Usage (typically called via fish -c from parent process):
#   fish -c "
#       cd $PWD
#       source script_lib.fish
#       run_full_doc_build_task \\
#           $CHECK_TARGET_DIR_DOC_STAGING_FULL \\
#           $CHECK_TARGET_DIR_DOC_STAGING_QUICK \\
#           $CHECK_TARGET_DIR \\
#           $CHECK_LOG_FILE \\
#           $NOTIFICATION_EXPIRE_MS
#   " &
#
# Rust migration: Use tokio::spawn or std::thread::spawn. Consider using
# channels (mpsc) for communication instead of forking. The recursive
# forking could become a loop with proper async/await.
function run_full_doc_build_task
    set -l staging_full $argv[1]
    set -l staging_quick $argv[2]
    set -l serving_dir $argv[3]
    set -l log_file $argv[4]
    set -l notify_expire_ms $argv[5]

    # Capture build start time for catch-up detection
    set -l full_build_start (date +%s)

    log_and_print $log_file "["(timestamp)"] [bg] 🔨 Full build starting (with deps)..."

    # Build full docs
    if build_and_sync_full_docs $staging_full $serving_dir
        log_and_print $log_file "["(timestamp)"] [bg] ✅ Full build done, synced to serving"

        # Catch-up check: did source files change during our ~90s build?
        if has_source_changes_since $full_build_start
            log_and_print $log_file "["(timestamp)"] [bg] ⚡ Changes during build, running catch-up..."

            # Run quick build for fast feedback (broken links OK - full build will fix)
            if build_and_sync_quick_docs $staging_quick $serving_dir
                log_and_print $log_file "["(timestamp)"] [bg] ✅ Quick catch-up complete!"
                log_and_print $log_file "["(timestamp)"] [bg] 🔀 Forking another full build to fix links..."

                # Fork another full build to eventually fix the broken links
                # This creates a cycle: quick build → full build → (if changes) → quick → full...
                # Eventually, no changes occur during a build, and we reach consistent state.
                fish -c "
                    cd $PWD
                    source script_lib.fish
                    run_full_doc_build_task $staging_full $staging_quick $serving_dir $log_file $notify_expire_ms
                " &

                send_system_notification "Watch ($WORKSPACE_NAME): Quick Docs Ready ⚡" "Workspace ready (no ext crate links) - full build starting..." "success" $notify_expire_ms
            else
                log_and_print $log_file "["(timestamp)"] [bg] ⚠️ Quick catch-up failed (full docs still available)"
                send_system_notification "Watch ($WORKSPACE_NAME): Full Docs Ready ✅" "Full build done, but latest edits have errors ❌" "normal" $notify_expire_ms
            end
        else
            # No changes during build - full docs are already up to date
            send_system_notification "Watch ($WORKSPACE_NAME): Full Docs Built ✅" "All documentation including dependencies built" "success" $notify_expire_ms
        end
    else
        log_and_print $log_file "["(timestamp)"] [bg] ❌ Full build failed!"
        send_system_notification "Watch ($WORKSPACE_NAME): Full Doc Build Failed ❌" "cargo doc failed" "critical" $notify_expire_ms
    end
end

# ============================================================================
# Shared Toolchain Script Functions
# ============================================================================
# These functions are shared between rust-toolchain-update.fish and
# rust-toolchain-sync-to-toml.fish to avoid code duplication.
#
# Convention: Functions use global variables set by the calling script:
#   - $LOG_FILE: Path to log file
#   - $PROJECT_DIR: Project root directory
#   - $TOOLCHAIN_FILE: Path to rust-toolchain.toml
#   - $target_toolchain: The toolchain being installed/validated
# ============================================================================

# Logs a message to both stdout and the log file.
#
# Uses global: $LOG_FILE
#
# Usage:
#   toolchain_log "Installing components..."
function toolchain_log
    set -l message $argv[1]
    echo $message | tee -a $LOG_FILE
end

# Logs a message without trailing newline.
#
# Uses global: $LOG_FILE
#
# Usage:
#   toolchain_log_no_newline "Progress: "
function toolchain_log_no_newline
    set -l message $argv[1]
    echo -n $message | tee -a $LOG_FILE
end

# Runs a command and logs its output.
#
# Uses global: $LOG_FILE
#
# Usage:
#   toolchain_log_command "Installing toolchain..." rustup install nightly
function toolchain_log_command
    set -l description $argv[1]
    toolchain_log $description
    $argv[2..] 2>&1 | tee -a $LOG_FILE
    return $pipestatus[1]
end

# Validates that required prerequisites exist.
#
# Uses globals: $PROJECT_DIR, $TOOLCHAIN_FILE
#
# Checks:
# - Project directory exists
# - rust-toolchain.toml exists
# - Build dependencies (clang, wild) are available
#
# Returns: 0 if valid, 1 if not
function toolchain_validate_prerequisites
    toolchain_log "Validating prerequisites..."

    # Check if project directory exists
    if not test -d $PROJECT_DIR
        toolchain_log "ERROR: Project directory not found: $PROJECT_DIR"
        return 1
    end

    # Check if rust-toolchain.toml exists
    if not test -f $TOOLCHAIN_FILE
        toolchain_log "ERROR: rust-toolchain.toml not found: $TOOLCHAIN_FILE"
        return 1
    end

    # Ensure build dependencies (clang, wild) are available
    if not ensure_build_dependencies
        toolchain_log "ERROR: Failed to install required build dependencies"
        return 1
    end

    toolchain_log "✅ Prerequisites validated successfully"
    return 0
end

# Shows current toolchain state.
#
# Uses globals: $PROJECT_DIR, $LOG_FILE
function toolchain_show_current_state
    toolchain_log "Changing to project directory: $PROJECT_DIR"
    cd $PROJECT_DIR

    if not toolchain_log_command "Current toolchain information:" rustup show
        toolchain_log "WARNING: Failed to get current toolchain information"
    end
end

# Verifies final toolchain state after installation.
#
# Uses global: $LOG_FILE
function toolchain_verify_final_state
    if not toolchain_log_command "Final installed toolchains:" rustup toolchain list
        toolchain_log "WARNING: Failed to list final toolchains"
    end

    if not toolchain_log_command "Verifying project toolchain:" rustup show
        toolchain_log "WARNING: Failed to verify project toolchain"
    end
end

# Installs the target toolchain if not already installed.
#
# Uses globals: $target_toolchain, $LOG_FILE
#
# Returns: 0 on success, 1 on failure
function toolchain_install_target
    if not toolchain_log_command "Installing toolchain $target_toolchain (if not already installed)..." rustup toolchain install $target_toolchain
        toolchain_log "❌ Failed to install $target_toolchain"
        return 1
    end

    toolchain_log "✅ Successfully installed/verified $target_toolchain"
    return 0
end

# Installs the rust-analyzer component for the target toolchain.
#
# Uses globals: $target_toolchain, $LOG_FILE
#
# Returns: 0 on success, 1 on failure
function toolchain_install_rust_analyzer
    toolchain_log "Installing rust-analyzer component for $target_toolchain..."
    if not toolchain_log_command "Adding rust-analyzer component..." rustup component add rust-analyzer --toolchain $target_toolchain
        toolchain_log "❌ Failed to install rust-analyzer component"
        return 1
    end

    toolchain_log "✅ Successfully installed rust-analyzer component"
    return 0
end

# Installs additional components (rust-src) for IDE support.
#
# Uses globals: $target_toolchain, $LOG_FILE
#
# Note: Failures are non-fatal (logs warning but returns 0)
function toolchain_install_additional_components
    toolchain_log "Installing additional components for $target_toolchain..."

    # Install rust-src for better IDE support (go-to-definition for std library)
    if toolchain_log_command "Adding rust-src component..." rustup component add rust-src --toolchain $target_toolchain
        toolchain_log "✅ Successfully installed rust-src component"
    else
        toolchain_log "⚠️  Failed to install rust-src component (continuing anyway)"
    end

    return 0
end

# Installs all required components for a toolchain.
#
# This is a convenience function that calls:
# - toolchain_install_rust_analyzer
# - toolchain_install_additional_components
# - install_windows_target
#
# Uses globals: $target_toolchain, $LOG_FILE
#
# Returns: 0 on success, 1 if rust-analyzer fails (other failures are warnings)
function toolchain_install_all_components
    # Install rust-analyzer component (required)
    if not toolchain_install_rust_analyzer
        return 1
    end

    # Install additional components (rust-src for IDE support)
    toolchain_install_additional_components

    # Install Windows cross-compilation target for verifying platform-specific code
    install_windows_target

    return 0
end
# Find and purge zombie processes and their project-related parents.
#
# Zombie processes (STAT 'Z') are already dead but remain in the process table
# until their parent reaps them via wait(). In test runs, bugs can cause parents
# to hang or crash without reaping children, leading to zombie leaks.
#
# This function:
# 1. Finds all zombie processes
# 2. Identifies parents that are project-related (r3bl_tui*)
# 3. Kills those parents to allow init to reap the zombies
#
# This is a shared utility used by both check.fish and run.fish.
function purge_zombie_processes
    # Find all zombie processes and their parents
    # ps -eo pid,ppid,stat,comm
    # STAT 'Z' or 'Z+' indicates a zombie
    # awk filter: $3 (stat) matches Z, print $1 (pid), $2 (ppid), $4 (comm)
    set -l zombies (ps -eo pid,ppid,stat,comm | awk '$3 ~ /^Z/ {print $1, $2, $4}')

    if test -z "$zombies"
        return 0
    end

    set -l parents_to_kill
    for z in $zombies
        # $z looks like "1234 1111 my_zombie"
        set -l parts (string split " " $z)
        if test (count $parts) -lt 3
            continue
        end
        set -l pid $parts[1]
        set -l ppid $parts[2]
        set -l z_comm $parts[3]

        # Identify the parent process name
        set -l parent_comm (ps -o comm= -p $ppid 2>/dev/null | string trim)

        # Only kill parents that are project-related or our own test binaries.
        # This prevents accidental killing of system processes or unrelated apps.
        # r3bl_tui-* is the pattern for our test binaries and main crate.
        if string match -q "r3bl*" "$parent_comm"; or string match -q "r3bl*" "$z_comm"
            if not contains $ppid $parents_to_kill
                # Suicide prevention: Avoid killing PID 1 (init) or our own shell ($fish_pid).
                # This ensures that running this purge doesn't terminate the script itself.
                if test "$ppid" -gt 1; and test "$ppid" -ne $fish_pid
                    set -a parents_to_kill $ppid
                end
            end
        end
    end

    if test (count $parents_to_kill) -gt 0
        # If log_message is available (check.fish), use it. Otherwise echo.
        if functions -q log_message
            log_message "🧟 Found "(count $zombies)" zombie(s). Purging project-related parents..."
            for ppid in $parents_to_kill
                set -l pcomm (ps -o comm= -p $ppid 2>/dev/null | string trim)
                if test -n "$pcomm"
                    log_message "   🔪 Killing parent $pcomm (PID $ppid)"
                    kill -9 $ppid 2>/dev/null
                end
            end
        else
            echo "🧟 Found "(count $zombies)" zombie(s). Purging project-related parents..."
            for ppid in $parents_to_kill
                set -l pcomm (ps -o comm= -p $ppid 2>/dev/null | string trim)
                if test -n "$pcomm"
                    echo "   🔪 Killing parent $pcomm (PID $ppid)"
                    kill -9 $ppid 2>/dev/null
                end
            end
        end
    end
end
