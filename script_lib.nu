# This file contains utility functions that shared between all the `run` scripts
# in various sub folders in this workspace.

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

def run_example [options: list<string>, release: bool, no_log: bool] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
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

# Run an example with flamegraph profiling using the profiling-detailed profile.
# This provides detailed profiling with less optimization for granular data.
def run_example_with_flamegraph_profiling_svg [options: list<string>] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
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

# Run an example with profiling to generate collapsed stacks format (perf-folded) instead of SVG
# This generates a much smaller text file with stack traces and sample counts
def run_example_with_flamegraph_profiling_perf_fold [options: list<string>] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
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

        # Build the example with profiling-detailed profile
        print $'(ansi cyan)Building example with profiling-detailed profile...(ansi reset)'
        cargo build --profile profiling-detailed --example $selection

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

        # Run perf record directly
        print $'(ansi cyan)Running perf record...(ansi reset)'
        sudo perf record -g --call-graph=fp,8 -F 99 -o perf.data -- $binary_path

        # Check if inferno-collapse-perf is available
        if (which inferno-collapse-perf | is-empty) {
            print $'(ansi red)Error: inferno-collapse-perf is not installed.(ansi reset)'
            print $'(ansi yellow)Please run from the repo root:(ansi reset)'
            print $'  nu run.nu install-cargo-tools'
            return
        }

        # Convert perf data to collapsed stacks format using inferno (comes with cargo flamegraph)
        print $'(ansi cyan)Converting to collapsed stacks format...(ansi reset)'
        sudo perf script -i perf.data | inferno-collapse-perf | save --force flamegraph.perf-folded

        # Show file size comparison
        let folded_size = (ls flamegraph.perf-folded | get size | first)
        print $'(ansi green)Generated flamegraph.perf-folded: ($folded_size)(ansi reset)'

        # Count total samples
        let total_samples = (open flamegraph.perf-folded | lines | each { |line|
            $line | split row ' ' | last | into int
        } | math sum)
        print $'(ansi cyan)Total samples: (ansi green)($total_samples)(ansi reset)'

        # Reset kernel parameters
        print "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2
        sudo sysctl -w kernel.kptr_restrict=1
        print "Kernel parameters reset."
    }
}
