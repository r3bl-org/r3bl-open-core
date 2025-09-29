#!/usr/bin/env bash

# This script provides a quick visual indicator of test status for display in the GNOME top bar.
# It runs cargo nextest with all output suppressed and returns a simple emoji status that can
# be consumed by the Executor extension (https://extensions.gnome.org/extension/2932/executor/).
#
# The script outputs either " 🧪✔️" when all tests pass or " 🧪❌" when tests fail,
# making it perfect for at-a-glance status monitoring during development.

pushd $HOME/github/r3bl-open-core/ >/dev/null

# Clean up any rustc ICE (Internal Compiler Error) files and cargo cache if ICE files exist
if ls rustc-ice*.txt >/dev/null 2>&1; then
    cargo cache -r all >/dev/null 2>&1
    cargo clean >/dev/null 2>&1
    sccache-clear >/dev/null 2>&1
    rm -f rustc-ice*.txt 2>/dev/null
fi

# Run cargo nextest with minimal output and provide one-line status
cargo nextest run --cargo-quiet --cargo-quiet --hide-progress-bar --status-level none --color never >/dev/null 2>&1

# Check the exit code and print appropriate message
if [ $? -eq 0 ]; then
    echo " 🧪✔️"
else
    echo " 🧪❌"
    exit 1
fi

popd >/dev/null
