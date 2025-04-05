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
    let result: list<string> = ($cleaned_binaries | append $cleaned_folders)

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

        # sudo sysctl -w kernel.perf_event_paranoid=-1
        # CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --example demo
        # firefox-beta flamegraph.svg

        sudo sysctl -w kernel.perf_event_paranoid=-1
        CARGO_PROFILE_RELEASE_DEBUG=true cargo flamegraph --example $selection
        firefox-beta --new-window flamegraph.svg
    }
}
