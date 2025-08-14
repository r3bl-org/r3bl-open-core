#!/bin/bash

# This is primarily for use with `cmdr/src/analytics_client/upgrade_check.rs`:
# 1. Script to remove all rustup toolchains for testing upgrade progress.
# 2. This ensures rustup toolchain install will show meaningful output.

set -e  # Exit on any error

echo "🔧 Removing all rustup toolchains for testing..."

# Save current directory and change to temp directory to avoid rust-toolchain.toml interference
TEMP_DIR=$(mktemp -d)
echo "📁 Working from temp directory: $TEMP_DIR"
pushd "$TEMP_DIR" > /dev/null

# List current toolchains for reference
echo "📋 Current toolchains:"
rustup toolchain list

echo ""
echo "🗑️  Removing ALL existing toolchains..."

# Remove all toolchains
for toolchain in $(rustup toolchain list | cut -d' ' -f1); do
    echo "  Removing: $toolchain"
    rustup toolchain uninstall "$toolchain" || echo "    ⚠️  Failed to remove $toolchain"
done

# Clean up any remaining toolchain directories
echo "🧹 Cleaning up toolchain directories..."
rm -rf "$HOME/.rustup/toolchains/"*

# Verify no toolchains remain
echo ""
echo "📋 Toolchains after cleanup:"
rustup toolchain list

echo ""
echo "✅ All toolchains removed!"
echo "🚀 Now when you run the upgrade process, it will show full progress"
echo "📝 The upgrade will install nightly with visible download/installation output"
echo "⚠️  Note: You'll need to set a default toolchain after upgrading"
echo ""

# Restore original directory and clean up temp directory
popd > /dev/null
rm -rf "$TEMP_DIR"