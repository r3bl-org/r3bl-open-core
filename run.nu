#!/usr/bin/env nu

# From: https://github.com/r3bl-org/nu_script_template
#
# nushell docs
# - language fundamentals: https://www.nushell.sh/book/custom_commands.html#sub-commands
#     - strings: https://www.nushell.sh/book/working_with_strings.html#string-interpolation
#     - functions (aka commands, subcommands): https://www.nushell.sh/book/custom_commands.html
#     - variables: https://www.nushell.sh/book/variables_and_subexpressions.html
# - built-in command reference: https://www.nushell.sh/commands/#command-reference
#     - length: https://www.nushell.sh/commands/docs/length.html
#     - ansi: https://www.nushell.sh/commands/docs/ansi.html
#     - print: https://www.nushell.sh/commands/docs/print.html
#     - input: https://www.nushell.sh/commands/docs/input.html
#     - input list: https://www.nushell.sh/commands/docs/input_list.html
#     - match: https://www.nushell.sh/commands/docs/match.html
# - mental model
#     - coming from bash: https://www.nushell.sh/book/coming_from_bash.html
#     - thinking in nu: https://www.nushell.sh/book/thinking_in_nu.html

source script_lib.nu

# Make sure to keep this in sync with `Cargo.toml` workspace members.
let workspace_folders = (get_cargo_projects)

# Main entry point for the script.
def main [...args: string] {
    let num_args = $args | length

    if $num_args == 0 {
        print-help all
        return
    }

    let command = $args | get 0

    match $command {
        "all" => {all}
        "all-cicd" => {all-cicd}
        "build" => {build}
        "build-full" => {build-full}
        "clean" => {clean}
        "install-cargo-tools" => {install-cargo-tools}
        "test" => {test}
        "watch-all-tests" => {watch-all-tests}
        "docs" => {docs}
        "check" => {check}
        "check-watch" => {check-watch}
        "clippy" => {clippy}
        "clippy-watch" => {clippy-watch}
        "rustfmt" => {rustfmt}
        "upgrade-deps" => {upgrade-deps}
        "serve-docs" => {serve-docs}
        "audit-deps" => {audit-deps}
        "unmaintained" => {unmaintained}
        "help" => {print-help all}
        "ramdisk-create" => {ramdisk-create}
        "ramdisk-delete" => {ramdisk-delete}
        "build-server" => {build-server}
        _ => {print $'Unknown command: (ansi red_bold)($command)(ansi reset)'}
    }
}


# Prints help for the script.
# - If "all" is passed in, prints help for all commands.
# - Otherwise, prints help for the specified command.
def print-help [command: string] {
    if $command == "all" {
        print $'Usage: (ansi magenta_bold)run(ansi reset) (ansi green_bold)<command>(ansi reset) (ansi blue_bold)[args](ansi reset)'
        print $'(ansi green_bold)<command>(ansi reset) can be:'
        print $'    (ansi green)all(ansi reset)'
        print $'    (ansi green)all-cicd(ansi reset)'
        print $'    (ansi green)build(ansi reset)'
        print $'    (ansi green)build-full(ansi reset)'
        print $'    (ansi green)clean(ansi reset)'
        print $'    (ansi green)install-cargo-tools(ansi reset)'
        print $'    (ansi green)test(ansi reset)'
        print $'    (ansi green)watch-all-tests(ansi reset)'
        print $'    (ansi green)docs(ansi reset)'
        print $'    (ansi green)check(ansi reset)'
        print $'    (ansi green)check-watch(ansi reset)'
        print $'    (ansi green)clippy(ansi reset)'
        print $'    (ansi green)clippy-watch(ansi reset)'
        print $'    (ansi green)rustfmt(ansi reset)'
        print $'    (ansi green)upgrade-deps(ansi reset)'
        print $'    (ansi green)serve-docs(ansi reset)'
        print $'    (ansi green)audit-deps(ansi reset)'
        print $'    (ansi green)unmaintained(ansi reset)'
        print $'    (ansi green)ramdisk-create(ansi reset)'
        print $'    (ansi green)ramdisk-delete(ansi reset)'
        print $'    (ansi green)build-server(ansi reset)'
        print $'    (ansi green)help(ansi reset)'
    } else {
        print $'Unknown command: (ansi red_bold)($command)(ansi reset)'
    }
}

def build-server [] {
    # Where you source files live.
    let orgn_path = "/home/nazmul/github/r3bl-open-core/"
    # Copy files to here.
    let dest_host = "nazmul-laptop.local"
    let dest_path = $"($dest_host):($orgn_path)"

    # Function to run rsync. Simple.
    def run_rsync [] {
        print $'(ansi cyan_underline)Changes detected, running rsync(ansi reset)'

        let crlf = (char newline)
        let prefix = "┤ "
        let hanging_prefix = "├ "
        let tab = (char tab)
        let msg_1 = $'(ansi yellow)($orgn_path)(ansi reset)'
        let msg_2 = $'(ansi blue)($dest_path)(ansi reset)'
        print $"($prefix)from:($tab)($msg_1)($crlf)($hanging_prefix)to:($tab)($msg_2)"
        rsync -q -avz --delete --exclude 'target/' $orgn_path $dest_path

        print $'(ansi cyan_underline)Rsync complete(ansi reset)'
    }

    # Main loop
    loop {
        # Construct the inotifywait command with all directories.
        let inotify_command = "inotifywait"
        let inotify_args = [
            "-r",
            "-e", "modify",
            "-e", "create",
            "-e", "delete",
            "-e", "move" ,
            "--exclude", "target" ,
            $orgn_path,
        ]

        print $'(ansi green_bold) (ansi yellow)($inotify_command)(ansi reset) (ansi blue)($inotify_args)(ansi reset)'

        print $'(ansi cyan)❪◕‿◕❫ (ansi reset)(ansi green)Please run bacon on build server: (ansi reset)(ansi yellow_underline)bacon nextest -W(ansi reset), (ansi yellow_underline)bacon doc -W(ansi reset), (ansi yellow_underline)bacon clippy -W(ansi reset)'

        # Execute the inotifywait command.
        ^$inotify_command ...$inotify_args

        # Run rsync
        run_rsync
    }
}

def watch-all-tests [] {
    cargo watch -x 'test --workspace --quiet --color always -- --test-threads 4' -c -q --delay 2
    # cargo watch -x 'test --workspace' -c -q --delay 2
    # cargo watch --exec check --exec 'test --quiet --color always -- --test-threads 4' --clear --quiet --delay 2
}

# https://thelinuxcode.com/create-ramdisk-linux/
def ramdisk-create [] {
    # Use findmnt -t tmpfs target_foo to check if the ramdisk is mounted.
    if (findmnt -t tmpfs target | is-empty) {
        print $'(ansi green_bold)Ramdisk is not mounted. Creating ramdisk...(ansi reset)'
    } else {
        print $'(ansi blue_bold)Ramdisk is already mounted.(ansi reset)'
        return
    }

    rm -rf target/
    sudo mkdir -p target/
    sudo mount -t tmpfs -o size=8g,noatime,nodiratime tmpfs target/
}

# https://thelinuxcode.com/create-ramdisk-linux/
def ramdisk-delete [] {
    if (findmnt -t tmpfs target | is-empty) {
        print $'(ansi blue_bold)Ramdisk is not mounted. Nothing to do...(ansi reset)'
        return
    } else {
        print $'(ansi green_bold)Ramdisk is mounted. Deleting it...(ansi reset)'
    }

    sudo umount target/
    sudo rmdir target/
}

def install-cargo-tools [] {
    cargo install bacon
    cargo install cargo-workspaces
    cargo install cargo-cache
    cargo install cargo-watch # cargo-watch is no longer maintained (as of Oct 2024). Move away from this.
    cargo install flamegraph
    cargo install cargo-outdated
    cargo install cargo-update
    cargo install cargo-deny
    cargo install cargo-unmaintained
    cargo install cargo-expand
    cargo install cargo-readme
    cargo install cargo-nextest
}

def all [] {
    cargo install cargo-deny
    cargo install cargo-unmaintained
    cargo install cargo-readme
    build
    test
    clippy
    docs
    audit-deps
    unmaintained
    rustfmt
}

# Runs everything that all does, except for cargo-unmaintained and cargo-deny.
def all-cicd [] {
    cargo install cargo-readme
    build
    test
    clippy
    docs

    cargo install cargo-deny
    audit-deps

    rustfmt

    # unmaintained
    # cargo install cargo-unmaintained
}

# https://github.com/trailofbits/cargo-unmaintained
# This is very slow to run.
def unmaintained [] {
    cargo unmaintained --color always --fail-fast --tree --verbose
}

def build [] {
    cargo build
}

def build-full [] {
    install-cargo-tools
    cargo cache -r all
    cargo clean
    cargo +nightly update
    cargo build
}

def clean [] {
    cargo clean
}

def test [] {
    for folder in ($workspace_folders) {
        cd $folder
        print $'(ansi magenta)≡ Running tests in ($folder) .. ≡(ansi reset)'
        cargo test -q -- --test-threads=20
        cd ..
    }
}

def check [] {
    cargo check --workspace
}

def check-watch [] {
    cargo watch -x 'check --workspace'
}

def clippy [] {
    for folder in ($workspace_folders) {
        cd $folder
        print $'(ansi magenta)≡ Running cargo clippy in ($folder) .. ≡(ansi reset)'
        cargo fix --allow-dirty --allow-staged
        cargo fmt --all
        cd ..
    }
}

def clippy-watch [] {
    cargo fix --allow-dirty --allow-staged
    cargo fmt --all
    cargo watch -x 'clippy --fix --allow-dirty --allow-staged' -c -q
}

def docs [] {
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Running cargo doc in ($folder) .. ≡(ansi reset)'
        cargo doc --no-deps --all-features
        # Output redirection in Nushell:
        # https://stackoverflow.com/questions/76403457/how-to-redirect-stdout-to-a-file-in-nushell
        cargo readme out> README.md
        cd ..
    }
}

def serve-docs [] {
    npm i -g serve
    serve target/doc
}

def upgrade-deps [] {
    for folder in $workspace_folders {
        cd $folder
        print $'(ansi magenta)≡ Upgrading ($folder) .. ≡(ansi reset)'
        cargo outdated --workspace --verbose
        cargo upgrade --verbose
        cd ..
    }
}

def rustfmt [] {
    cargo fmt --all
    # for folder in $workspace_folders {
    #     cd $folder
    #     print $'(ansi magenta)≡ Running cargo fmt --all in ($folder) .. ≡(ansi reset)'
    #     cargo fmt --all
    #     cd ..
    # }
}

# More info: https://github.com/EmbarkStudios/cargo-deny
# To allow exceptions, please edit the `deny.toml` file.
def audit-deps [] {
    cargo deny check licenses advisories
}
