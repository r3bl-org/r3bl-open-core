#!/usr/bin/env fish

# 1. Make sure to install cargo-outdated via `cargo install --locked cargo-outdated`.
# More info about cargo-outdated: https://crates.io/crates/cargo-outdated

cargo outdated --workspace --verbose
cargo upgrade --to-lockfile --verbose
cargo update
