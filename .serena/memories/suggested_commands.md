# R3BL Open Core - Essential Development Commands

## Setup Commands
```bash
# Automated setup (recommended)
./bootstrap.sh

# Manual Rust installation
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
nu run.nu install-cargo-tools
```

## Core Development Workflow

### Main Build Commands
```bash
# Run all major checks (build, test, clippy, docs, audit, format)
nu run.nu all

# Build entire workspace
nu run.nu build

# Full build with clean and update
nu run.nu build-full

# Test entire workspace
nu run.nu test

# Clean workspace
nu run.nu clean
```

### Code Quality Commands
```bash
# Fast typecheck
cargo check

# Lint with clippy
cargo clippy --all-targets
# Auto-fix clippy issues
cargo clippy --fix --allow-dirty

# Generate documentation (fix doc errors)
cargo doc --no-deps

# Run tests with nextest
cargo nextest run
```

### Watch Commands (Development)
```bash
# Watch files, run all tests
nu run.nu watch-all-tests

# Watch files, run specific test pattern
nu run.nu watch-one-test <pattern>

# Watch files, run clippy
nu run.nu watch-clippy

# Watch files, run cargo check
nu run.nu watch-check
```

### Application Commands
```bash
# Run TUI examples interactively
nu run.nu run-examples [--release] [--no-log]

# Run cmdr binaries (edi, giti, rc) interactively
nu run.nu run-binaries

# Install cmdr binaries system-wide
nu run.nu install-cmdr
```

### Performance Analysis
```bash
# Run benchmarks
nu run.nu bench

# Generate SVG flamegraph
nu run.nu run-examples-flamegraph-svg

# Generate perf-folded format
nu run.nu run-examples-flamegraph-fold
```

### Cache Management
```bash
# Check sccache status
sccache --show-stats

# Reset sccache
sccache --zero-stats
sccache --stop-server
rm -rf ~/.cache/sccache
```

### Development Utilities
```bash
# Format all code
nu run.nu rustfmt

# Security audit dependencies
nu run.nu audit-deps

# Check for unmaintained dependencies
nu run.nu unmaintained

# Upgrade dependencies
nu run.nu upgrade-deps

# Monitor logs
nu run.nu log

# View help
nu run.nu help
```

## Platform-Specific Notes
- **Linux**: Uses `inotifywait` for file watching
- **macOS**: Uses `fswatch` for file watching
- **Windows**: Manual installation recommended

## Git Workflow
- Never commit unless explicitly asked
- Use standard git commands
- Project uses main branch for PRs