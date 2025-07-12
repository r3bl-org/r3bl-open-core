# This file contains utility functions that shared between all the `run` scripts
# in various sub folders in this workspace.

def run_example [options: list<string>, release: bool, no_log: bool] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
        print "No example selected.";
    } else {
        let release_flag = if $release { "--release" } else { "" }
        let log_flag = if $no_log { "-- --no-log" } else { "" }
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

def run_example_with_flamegraph_profiling [options: list<string>] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
        print "No example selected.";
    } else {
        print $'(ansi cyan)Running example with options: (ansi green)($options)(ansi cyan), selection: (ansi green)($selection)(ansi reset)'
        print $'(ansi cyan)Current working directory: (ansi green)($env.PWD)(ansi reset)'
        print $"cargo flamegraph --example ($selection)"

        # Change the kernel parameters to allow perf to access kernel symbols.
        sudo sysctl -w kernel.perf_event_paranoid=-1
        sudo sysctl -w kernel.kptr_restrict=0

        CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --freq 99 --example $selection

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

        firefox-beta --new-window flamegraph.svg

        # Reset kernel parameters (optional but recommended for security)
        print "Resetting kernel parameters..."
        sudo sysctl -w kernel.perf_event_paranoid=2 # Default paranoid level (often 2)
        sudo sysctl -w kernel.kptr_restrict=1      # Default restrict level (often 1)
        print "Kernel parameters reset."
    }
}

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
