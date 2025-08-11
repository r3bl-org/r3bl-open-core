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
def watch-files [command: string, dir: string = "."] {
    let watcher = if ($env.OS? == "Darwin") { "fswatch" } else { "inotifywait" }
    
    if (which $watcher | is-empty) {
        error make { msg: $"Install ($watcher) first. Run ./bootstrap.sh to set up." }
    }
    
    loop {
        print $'Watching ($dir) for changes...'
        
        let watch_result = try {
            if ($env.OS? == "Darwin") {
                ^fswatch -r --exclude "target|.git" -1 $dir | complete
            } else {
                ^inotifywait -r -e modify,create,delete,move --exclude "target|.git" $dir
            }
        } catch {
            # User pressed Ctrl+C, exit gracefully
            return
        }
        
        print $'Running: ($command)'
        bash -c $command
    }
}

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
def install_if_missing [tool: string, cmd: string] {
    if (which $tool | is-empty) { 
        print $'Installing ($tool)...'; bash -c $cmd 
    } else { 
        print $'✓ ($tool) installed' 
    }
}

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
# - null if no supported package manager found
#
# Usage:
#   let pkg_mgr = get_package_manager
#   if ($pkg_mgr != null) {
#       bash -c $"($pkg_mgr) neovim"
#   }
def get_package_manager [] {
    if ($env.OS? == "Darwin") { 
        "brew install" 
    } else {
        ["apt-get" "dnf" "pacman"] | each {|pm|
            if (which $pm | is-not-empty) {
                match $pm { 
                    "apt-get" => "sudo apt install -y", 
                    "dnf" => "sudo dnf install -y", 
                    _ => $"sudo ($pm) -S --noconfirm" 
                }
            }
        } | where $it != null | first
    }
}

# Executes a closure in a specific directory and safely returns to the original.
#
# This utility ensures that directory changes are properly managed, always
# returning to the original directory even if the closure fails.
#
# Parameters:
# - dir: Target directory to change to
# - closure: Code block to execute in the target directory
#
# Features:
# - Automatic directory restoration
# - Exception safety
# - Returns closure's result
#
# Usage:
#   let result = run_in_directory "src" {
#       ls | where type == "file" | length
#   }
#
# Example:
#   # Count Rust files in src directory
#   run_in_directory "src" {
#       ls *.rs | length
#   }
def run_in_directory [dir: string, closure: closure] {
    let original_dir = $env.PWD
    cd $dir
    try {
        let result = do $closure
        cd $original_dir
        $result
    } catch {|e|
        cd $original_dir
        error make $e
    }
}

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
def docker_stop_all_containers [] {
    let running_containers = (^docker ps -aq | lines | where $it != "")
    if ($running_containers | length) > 0 {
        print $"Stopping ($running_containers | length) running containers..."
        $running_containers | each { |container_id|
            print $"Stopping container: ($container_id)"
            ^docker stop $container_id
        }
        print "Pruning system..."
        ^docker system prune -af
    }
}

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
def docker_remove_all_images [] {
    let images = (^docker image ls -q | lines | where $it != "")
    if ($images | length) > 0 {
        print $"Removing ($images | length) existing images..."
        $images | each { |image_id|
            print $"Removing image: ($image_id)"
            ^docker image rm -f $image_id
        }
    }
}

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
#   let projects = get_cargo_projects
#   for project in $projects {
#       cd $project
#       cargo build
#       cd ..
#   }
#
# Example workspace structure:
#   workspace/
#   ├── tui/Cargo.toml     -> returns "tui"
#   ├── cmdr/Cargo.toml    -> returns "cmdr"
#   └── docs/              -> ignored (no Cargo.toml)
def get_cargo_projects [] {
    let sub_folders_with_cargo_toml = (
        ls | where type == "dir" | each { |folder|
            if (try { open $"($folder.name)/Cargo.toml" } | is-empty) == false {
                # print $"Found Cargo.toml in ($folder.name)"
                $folder.name
            } else {
                # print $"No Cargo.toml in ($folder.name)"
                null
            }
        } | compact
    )
    $sub_folders_with_cargo_toml
}

# Runs a selected example with configurable build and logging options.
#
# This function provides an interactive fuzzy-search menu for example selection
# and handles the cargo execution with appropriate flags.
#
# Parameters:
# - options: List of available examples to choose from
# - release: Build in release mode if true
# - no_log: Disable logging output if true
#
# Features:
# - Fuzzy search for example selection
# - Graceful cancellation (Ctrl+C)
# - Debug/release mode support
# - Optional logging control
# - Detailed execution feedback
#
# Usage:
#   let examples = get_example_binaries
#   run_example $examples true false  # Release mode with logging
#   run_example $examples false true  # Debug mode without logging
def run_example [options: list<string>, release: bool, no_log: bool] {
    let selection = try {
        $options | input list --fuzzy 'Select an example to run: '
    } catch {
        # User pressed Ctrl+C, exit gracefully without error
        return
    }

    if ($selection == "") or ($selection == null) {
        print "No example selected.";
    } else {
        let release_flag = if $release { "--release" } else { "" }
        let log_flag = if $no_log { "--no-log" } else { "" }
        print $'(ansi cyan)Running example with options: (ansi green)($options)(ansi cyan), release: (ansi green)($release)(ansi cyan), selection: (ansi green)($selection)(ansi cyan), log: (ansi green)($no_log)(ansi reset)'
        print $'(ansi cyan)Current working directory: (ansi green)($env.PWD)(ansi reset)'
        print $"cargo run -q ($release_flag) --example ($selection) -- ($log_flag)"
        cargo run --example $selection $release_flag -q -- $log_flag
    }
}

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
#   let examples = get_example_binaries
#   # Returns: ["multi_file", "simple", "complex"]
def get_example_binaries [] {
    let example_files = (ls examples | where type == "file" | where name ends-with ".rs" | get name)
    let example_binaries = $example_files | each { str replace ".rs" "" }
    let cleaned_binaries = $example_binaries | each { str replace "examples/" "" }

    let example_folders = (ls examples | where type == "dir" | get name)
    let cleaned_folders = $example_folders | each { str replace "examples/" "" }
    # let result: list<string> = ($cleaned_binaries | append $cleaned_folders)
    let result: list<string> = ($cleaned_folders | append $cleaned_binaries)

    $result
}

# Runs an example with flamegraph profiling and generates an interactive SVG visualization.
#
# This advanced profiling function creates detailed flame graphs for performance
# analysis. It uses a special 'profiling-detailed' Cargo profile that balances
# optimization with symbol visibility.
#
# Parameters:
# - options: List of available examples to profile
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
#   let examples = get_example_binaries
#   run_example_with_flamegraph_profiling_svg $examples
#
# Note: The profiling-detailed profile must be defined in Cargo.toml
def run_example_with_flamegraph_profiling_svg [options: list<string>] {
    let selection = try {
        $options | input list --fuzzy 'Select an example to run: '
    } catch {
        # User pressed Ctrl+C, exit gracefully
        return
    }

    if ($selection == "") or ($selection == null) {
        print "No example selected.";
    } else {
        print $'(ansi cyan)Running example with options: (ansi green)($options)(ansi cyan), selection: (ansi green)($selection)(ansi reset)'
        print $'(ansi cyan)Current working directory: (ansi green)($env.PWD)(ansi reset)'

        # Check if required tools are available
        if (which perf | is-empty) {
            print $'(ansi red)Error: perf is not installed.(ansi reset)'
            print $'(ansi yellow)Please run the following from the repo root:(ansi reset)'
            print $'  1. ./setup-dev-tools.sh      # Installs system tools like perf'
            print $'  2. nu run.nu install-cargo-tools  # Installs cargo tools like flamegraph'
            return
        }

        if (which cargo-flamegraph | is-empty) {
            print $'(ansi red)Error: cargo-flamegraph is not installed.(ansi reset)'
            print $'(ansi yellow)Please run from the repo root:(ansi reset)'
            print $'  nu run.nu install-cargo-tools'
            return
        }

        print $"cargo flamegraph --profile profiling-detailed --example ($selection)"

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
        print $"(ansi magenta)Using 'profiling-detailed' profile with --no-inline.(ansi reset)"
        (RUSTFLAGS="-C force-frame-pointers=yes -C symbol-mangling-version=v0"
            cargo flamegraph
            --profile profiling-detailed
            --no-inline
            -c "record -g --call-graph=fp,8 -F 99"
            --example $selection)

        # Find PIDs for cargo flamegraph
        let flamegraph_pids = (pgrep -f "cargo flamegraph" | lines)

        # Find PIDs for perf script
        let perf_script_pids = (pgrep -f "perf script" | lines)

        # Combine all found PIDs and get only the unique ones
        let all_pids = ($flamegraph_pids | append $perf_script_pids | uniq)

        if ($all_pids | is-empty) {
            print "No cargo flamegraph or perf script processes found to kill. 🧐"
        } else {
            print $"Attempting to terminate the following process IDs: ($all_pids | str join ', ') 🔪"
            for pid in $all_pids {
                print $"  - Trying to gracefully kill PID: ($pid) (SIGTERM)"
                sudo kill $pid
            }
            print "All targeted processes should now be terminated. ✅"
        }

        # Open the flamegraph in browser
        if (which firefox-beta | is-empty) {
            print $'(ansi yellow)firefox-beta not found, using system default browser(ansi reset)'
            xdg-open flamegraph.svg
        } else {
            firefox-beta --new-window flamegraph.svg
        }

        # Reset kernel parameters (optional but recommended for security)
        print "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2 # Default paranoid level (often 2)
        sudo sysctl -w kernel.kptr_restrict=1      # Default restrict level (often 1)
        print "Kernel parameters reset."
    }
}

# Generates collapsed stack traces (perf-folded format) for detailed performance analysis.
#
# This function profiles an example and outputs data in the collapsed stack format,
# which is more compact than SVG and suitable for further processing or integration
# with other profiling tools.
#
# Parameters:
# - options: List of available examples to profile
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
#   let examples = get_example_binaries
#   run_example_with_flamegraph_profiling_perf_fold $examples
#
# Post-processing:
#   # Convert to flamegraph later:
#   cat flamegraph.perf-folded | flamegraph > output.svg
#
#   # Diff two profiles:
#   inferno-diff-folded before.perf-folded after.perf-folded | flamegraph > diff.svg
def run_example_with_flamegraph_profiling_perf_fold [options: list<string>] {
    let selection = try {
        $options | input list --fuzzy 'Select an example to run: '
    } catch {
        # User pressed Ctrl+C, exit gracefully
        return
    }

    if ($selection == "") or ($selection == null) {
        print "No example selected.";
    } else {
        print $'(ansi cyan)Running example to generate collapsed stacks: (ansi green)($selection)(ansi reset)'
        print $'(ansi cyan)Current working directory: (ansi green)($env.PWD)(ansi reset)'

        # Check if required tools are available
        if (which perf | is-empty) {
            print $'(ansi red)Error: perf is not installed.(ansi reset)'
            print $'(ansi yellow)Please run the following from the repo root:(ansi reset)'
            print $'  1. ./setup-dev-tools.sh      # Installs system tools like perf'
            print $'  2. nu run.nu install-cargo-tools  # Installs cargo tools like inferno'
            return
        }

        # Change the kernel parameters to allow perf to access kernel symbols
        sudo sysctl -w kernel.perf_event_paranoid=-1
        sudo sysctl -w kernel.kptr_restrict=0

        # Build the example with profiling-detailed profile and same RUSTFLAGS as SVG version
        print $'(ansi cyan)Building example with profiling-detailed profile...(ansi reset)'
        (RUSTFLAGS="-C force-frame-pointers=yes -C symbol-mangling-version=v0"
            cargo build --profile profiling-detailed --example $selection)

        # Wait a moment for the build to complete
        sleep 1sec

        # Get the binary path - target is in parent directory
        let binary_path = $"../target/profiling-detailed/examples/($selection)"

        # Check if the binary exists
        if not ($binary_path | path exists) {
            print $'(ansi red)Error: Binary not found at ($binary_path)(ansi reset)'
            print $'(ansi yellow)Please ensure the example builds successfully.(ansi reset)'
            return
        }

        # Run perf record with same options as the SVG version
        print $'(ansi cyan)Running perf record with enhanced symbol resolution...(ansi reset)'
        sudo perf record -g --call-graph=fp,8 -F 99 -o perf.data -- $binary_path

        # Fix ownership of perf.data files so they can be accessed without sudo
        let current_user = $env.USER
        sudo chown $"($current_user):($current_user)" perf.data
        if (ls perf.data.old | is-empty) == false {
            sudo chown $"($current_user):($current_user)" perf.data.old
        }

        # Check if inferno-collapse-perf is available
        if (which inferno-collapse-perf | is-empty) {
            print $'(ansi red)Error: inferno-collapse-perf is not installed.(ansi reset)'
            print $'(ansi yellow)Please run from the repo root:(ansi reset)'
            print $'  nu run.nu install-cargo-tools'
            return
        }

        # Convert perf data to collapsed stacks format using inferno (comes with cargo flamegraph)
        print $'(ansi cyan)Converting to collapsed stacks format...(ansi reset)'
        sudo perf script -f -i perf.data | inferno-collapse-perf | save --force flamegraph.perf-folded

        # Fix ownership of generated files
        sudo chown $"($current_user):($current_user)" flamegraph.perf-folded

        # Show file size comparison
        let folded_size = (ls flamegraph.perf-folded | get size | first)
        print $'(ansi green)Generated flamegraph.perf-folded: ($folded_size)(ansi reset)'

        # Count total samples (with error handling for empty files)
        if ($folded_size | into int) > 0 {
            let total_samples = (open flamegraph.perf-folded | lines | each { |line|
                if ($line | str trim | is-empty) {
                    0
                } else {
                    $line | split row ' ' | last | into int
                }
            } | math sum)
            print $'(ansi cyan)Total samples: (ansi green)($total_samples)(ansi reset)'
        } else {
            print $'(ansi yellow)Warning: flamegraph.perf-folded is empty. Check if perf recording was successful.(ansi reset)'
        }

        # Reset kernel parameters
        print "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2
        sudo sysctl -w kernel.kptr_restrict=1
        print "Kernel parameters reset."
    }
}
