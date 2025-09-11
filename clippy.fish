#!/usr/bin/env fish
# This is a temporary fix for clippy nightly toolchain not working in Sept 2025
set -e RUSTFLAGS
cargo +nightly-2025-09-01 clippy --all-targets
# cargo +nightly-2025-09-01 clippy --all-targets 2>&1 | setclip
