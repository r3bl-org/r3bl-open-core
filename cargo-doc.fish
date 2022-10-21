#!/usr/bin/env fish

# 1. Make sure to install cargo-watch via `cargo install cargo-watch`.
# More info about cargo-watch: https://crates.io/crates/cargo-watch

# 2. Make sure to install cargo-limit via `cargo install cargo-limit`.
# More info about carg-limit: https://crates.io/crates/cargo-limit


set -l folders . core macro redux tui

for folder in $folders
    pushd $folder
    echo (set_color brmagenta)"≡ Running cargo doc in '$folder' .. ≡"(set_color normal)
    sh -c "cargo doc"
    popd
end
