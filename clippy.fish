#!/usr/bin/env fish
# Use a nightly toolchain from one month ago to avoid issues with the latest nightly
set -e RUSTFLAGS

# Calculate the date one month ago and format it as YYYY-MM-DD for the toolchain
set one_month_ago (date -d "1 month ago" "+%Y-%m-%d")

# Clean up any previous build artifacts to ensure a fresh linting run
rm rustc-ice*.txt
cargo cache -r all ; cargo clean ; sccache-clear

cargo +nightly-$one_month_ago clippy --all-targets
# cargo +nightly-$one_month_ago clippy --all-targets 2>&1 | setclip
