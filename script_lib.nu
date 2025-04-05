# This file contains utility functions that shared between all the `run` scripts
# in various sub folders in this workspace.

def run_example [options: list<string>, release: bool] {
    let selection = $options | input list --fuzzy 'Select an example to run: '

    if $selection == "" {
        print "No example selected.";
    } else {
        let release_flag = if $release { "--release" } else { "" }
        print $'(ansi cyan)Running example with options: (ansi green)($options)(ansi cyan), release: (ansi green)($release)(ansi cyan), selection: (ansi green)($selection)(ansi reset)'
        print $'(ansi cyan)Current working directory: (ansi green)($env.PWD)(ansi reset)'
        print $"cargo run -q ($release_flag) --example ($selection)"
        cargo run --example $selection $release_flag -q
    }
}

def get_example_binaries [] {
    let example_files = (ls examples | where type == "file" | where name ends-with ".rs" | get name)
    let example_binaries = $example_files | each { str replace ".rs" "" }
    let result = $example_binaries | each { str replace "examples/" "" }
    $result
}

