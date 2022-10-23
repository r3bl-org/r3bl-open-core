#!/usr/bin/env fish

# 1. Make sure to install cargo-outdated via `cargo install --locked cargo-outdated`.
# More info about cargo-outdated: https://crates.io/crates/cargo-outdated

set -l folders . core macro redux tui

for folder in $folders
    pushd $folder
    echo (set_color brmagenta)"≡ Upgrading '$folder' .. ≡"(set_color normal)
    sh -c "cargo outdated --workspace --verbose"
    sh -c "cargo upgrade --to-lockfile --verbose"
    sh -c "cargo update"
    popd
end
