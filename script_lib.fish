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
#   install_if_missing "cargo-nextest" "cargo install cargo-nextest"
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

# Generates cargo configuration based on available tools.
#
# This function creates a .cargo/config.toml file with optimized build settings:
# - Uses sccache for faster compilation via build caching
# - Enables parallel frontend compilation with 8 threads
# - Configures Wild linker when both clang and wild are available
# - Falls back to standard parallel compilation if Wild linker unavailable
#
# Features:
# - Automatic detection of Wild linker availability
# - Platform-specific configuration (Linux x86_64 and aarch64)
# - Graceful fallback when tools are missing
# - Clear user feedback about configuration choices
#
# Prerequisites:
# - sccache should be installed for build caching
# - clang and wild should be installed for optimal linking performance
#
# Usage:
#   generate_cargo_config
function generate_cargo_config
    echo "Generating cargo configuration..."

    # Base configuration with sccache and parallel compilation
    echo '[build]
rustc-wrapper = "sccache"
rustflags = ["-Z", "threads=8"]  # Parallel frontend compiler' > .cargo/config.toml

    # Add Wild linker configuration if both clang and wild are available
    if command -v clang >/dev/null && command -v wild >/dev/null
        echo "✓ Wild linker available - adding to configuration"
        echo '
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = [
    "-Z", "threads=8",  # Parallel compilation
    "-C", "link-arg=--ld-path=wild"  # Wild linker
]

[target.aarch64-unknown-linux-gnu]
linker = "clang"
rustflags = [
    "-Z", "threads=8",  # Parallel compilation
    "-C", "link-arg=--ld-path=wild"  # Wild linker
]' >> .cargo/config.toml
    else
        echo "✓ Wild linker not available - using default configuration"
        echo '
[target.x86_64-unknown-linux-gnu]
rustflags = ["-Z", "threads=8"]  # Parallel compilation only

[target.aarch64-unknown-linux-gnu]
rustflags = ["-Z", "threads=8"]  # Parallel compilation only' >> .cargo/config.toml
    end

    echo "✓ Cargo configuration generated"
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
#   install_cargo_tool "cargo-nextest"
function install_cargo_tool --argument-names tool_name
    if not command -v $tool_name >/dev/null
        echo "Installing $tool_name..."
        if command -v cargo-binstall >/dev/null
            # Use cargo binstall for faster installation
            cargo binstall -y $tool_name
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

# Stops all running Docker containers and prunes the system.
#
# This utility function provides complete Docker cleanup by stopping all
# running containers and removing unused resources.
#
# Operations performed:
# 1. Lists all container IDs (running and stopped)
# 2. Stops each container gracefully
# 3. Prunes system to remove:
#    - Stopped containers
#    - Unused networks
#    - Dangling images
#    - Build cache
#
# Features:
# - Safe handling of empty container list
# - Progress feedback for each container
# - Automatic system cleanup
#
# Usage:
#   docker_stop_all_containers
#
# Note: Requires Docker to be installed and running
function docker_stop_all_containers
    set running_containers (docker ps -aq 2>/dev/null | grep -v '^$')

    if test (count $running_containers) -gt 0
        echo "Stopping "(count $running_containers)" running containers..."
        for container_id in $running_containers
            echo "Stopping container: $container_id"
            docker stop $container_id
        end
        echo "Pruning system..."
        docker system prune -af
    end
end

# Removes all Docker images from the local system.
#
# This function performs a complete cleanup of Docker images, useful for
# freeing disk space or ensuring clean builds.
#
# Operations:
# 1. Lists all image IDs
# 2. Force removes each image
#
# Features:
# - Handles images with dependent containers (force removal)
# - Safe handling of empty image list
# - Progress feedback for each image
#
# Warning: This will remove ALL images, including those in use.
# Containers using these images will need to re-download them.
#
# Usage:
#   docker_remove_all_images
function docker_remove_all_images
    set images (docker image ls -q 2>/dev/null | grep -v '^$')

    if test (count $images) -gt 0
        echo "Removing "(count $images)" existing images..."
        for image_id in $images
            echo "Removing image: $image_id"
            docker image rm -f $image_id
        end
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
function run_example_with_flamegraph_profiling_perf_fold
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
        echo "Running perf record with enhanced symbol resolution..."
        sudo perf record -g --call-graph=fp,8 -F 99 -o perf.data -- $binary_path

        # Fix ownership of perf.data files so they can be accessed without sudo
        set current_user $USER
        sudo chown "$current_user:$current_user" perf.data
        if test -f perf.data.old
            sudo chown "$current_user:$current_user" perf.data.old
        end

        # Check if inferno-collapse-perf is available
        if not command -v inferno-collapse-perf >/dev/null
            echo "Error: inferno-collapse-perf is not installed."
            echo "Please run from the repo root:"
            echo "  fish run.fish install-cargo-tools"
            return
        end

        # Convert perf data to collapsed stacks format using inferno (comes with cargo flamegraph)
        echo "Converting to collapsed stacks format..."
        sudo perf script -f -i perf.data | inferno-collapse-perf > flamegraph.perf-folded

        # Fix ownership of generated files
        sudo chown "$current_user:$current_user" flamegraph.perf-folded

        # Show file size comparison
        set folded_size (wc -c < flamegraph.perf-folded)
        echo "Generated flamegraph.perf-folded: $folded_size bytes"

        # Count total samples (with error handling for empty files)
        if test $folded_size -gt 0
            set total_samples (awk '{sum += $NF} END {print sum}' flamegraph.perf-folded)
            echo "Total samples: $total_samples"
        else
            echo "Warning: flamegraph.perf-folded is empty. Check if perf recording was successful."
        end

        # Reset kernel parameters
        echo "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2
        sudo sysctl -w kernel.kptr_restrict=1
        echo "Kernel parameters reset."
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

    # Only replace the first uncommented channel line
    sed -i "0,/^channel = .*/s//channel = \"$toolchain\"/" $toolchain_file
    return $status
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
            /bin/rm -rf $lock_dir 2>/dev/null

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
        /bin/rm -rf $lock_dir 2>/dev/null
        echo "🔓 Released toolchain operation lock" >&2
    end
end