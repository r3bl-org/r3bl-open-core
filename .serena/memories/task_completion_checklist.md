# Task Completion Checklist for R3BL Open Core

When completing any development task, follow this checklist to ensure code quality and project standards:

## Required Post-Task Commands

### 1. Fast Typecheck
```bash
cargo check
```
Ensures code compiles without building artifacts.

### 2. Linting
```bash
cargo clippy --all-targets
# OR auto-fix issues:
cargo clippy --fix --allow-dirty
```
Applies workspace-level lint rules (all + pedantic clippy).

### 3. Documentation
```bash
cargo doc --no-deps
```
Fixes documentation errors and generates docs.

### 4. Testing
```bash
cargo nextest run
```
Runs all tests with parallel execution.

## Comprehensive Quality Check
```bash
# Run everything at once
nu run.nu all
```
This runs: build, test, clippy, docs, audit, and format.

## Performance Analysis (When Needed)

### Benchmarks
```bash
cargo bench
```
For tests marked with `#[bench]`.

### Profiling
```bash
cargo flamegraph
```
General profiling tool.

### TUI-Specific Profiling
```bash
# For TUI applications, ask user to run:
run_example_with_flamegraph_profiling_perf_fold
```
Located in `lib_script.nu`.

## Code Style Verification
```bash
nu run.nu rustfmt
```
Ensures consistent formatting across workspace.

## Security and Dependencies
```bash
# Security audit
nu run.nu audit-deps

# Check for unmaintained dependencies
nu run.nu unmaintained
```

## Git Guidelines
- **Never commit unless explicitly asked**
- Always verify changes before committing
- Use descriptive commit messages
- Target main branch for PRs

## Common Issues and Solutions

### Build Cache Issues
```bash
# Reset sccache if builds seem slow
sccache --zero-stats
sccache --stop-server
rm -rf ~/.cache/sccache
```

### Watch Mode for Development
```bash
# Continuous testing during development
nu run.nu watch-all-tests

# Watch specific test patterns
nu run.nu watch-one-test <pattern>
```

## Verification Steps
1. ✅ Code compiles (`cargo check`)
2. ✅ No lint violations (`cargo clippy`)
3. ✅ Documentation builds (`cargo doc`)
4. ✅ All tests pass (`cargo nextest run`)
5. ✅ Code formatted (`nu run.nu rustfmt`)
6. ✅ No security issues (`nu run.nu audit-deps`)

Only after all steps pass should the task be considered complete.