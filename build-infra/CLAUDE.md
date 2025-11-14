# Claude Code Instructions for r3bl-build-infra

This crate provides CLI tools for the r3bl workspace. After making code changes, you **must** reinstall the binaries for the changes to take effect.

## Important: Binary Installation Workflow

This crate provides command-line tools (binaries) that are installed to `~/.cargo/bin`:
- `cargo-rustdoc-fmt` - Format markdown tables and links in rustdoc comments
- Other utility commands

### After Making Code Changes

**Always run this command to install the updated binary:**

```bash
cargo install --path . --force
```

Or from the workspace root:

```bash
cargo install --path build-infra --force
```

### Why This Matters

- The binaries in `~/.cargo/bin` are **separate files** from your source code
- Running `cargo build` or `cargo build --release` only compiles the code in `target/`
- Without `cargo install`, the old binary in `~/.cargo/bin` will be executed
- This can lead to confusing situations where your code changes don't appear to work

### Testing Workflow

When working on changes to this crate, follow this workflow:

1. **Make code changes**
2. **Run tests to verify logic:**
   ```bash
   cargo test --lib
   cargo test --all-targets
   ```
3. **Install the updated binary:**
   ```bash
   cargo install --path . --force
   ```
4. **Test the installed binary:**
   ```bash
   # Example: test cargo-rustdoc-fmt on an actual file
   cd .. && cargo rustdoc-fmt tui/src/some/file.rs
   ```

## Development Guidelines

### Code Quality Checklist

After making changes, run these checks in order:

1. `cargo check` - Fast typecheck
2. `cargo build` - Compile in debug mode
3. `cargo test --lib` - Run unit tests
4. `cargo test --all-targets` - Run all tests (including integration tests)
5. `cargo clippy -- -D warnings` - Lint with clippy
6. `cargo install --path . --force` - **Install updated binary**
7. Test the installed binary with actual files

### Module Organization

Follow the standard Rust module organization:
- Private implementation modules
- Public re-exports in `mod.rs` for clean API
- See root `CLAUDE.md` for general Rust guidelines

### Testing Best Practices

- Write unit tests for individual functions
- Write integration tests in `tests/` directory for end-to-end flows
- Use test fixtures in `tests/test_data/` for complex test cases
- Include both positive and negative test cases

## Project Structure

```
build-infra/
├── src/
│   ├── bin/                    # Binary entry points
│   │   └── cargo-rustdoc-fmt.rs
│   ├── cargo_rustdoc_fmt/      # cargo-rustdoc-fmt implementation
│   │   ├── cli_arg.rs          # CLI argument parsing
│   │   ├── processor.rs        # File processing orchestration
│   │   ├── extractor.rs        # Rustdoc block extraction
│   │   ├── link_converter.rs   # Link conversion and aggregation
│   │   ├── table_formatter.rs  # Markdown table formatting
│   │   └── validation_tests/   # Integration tests with fixtures
│   └── common/                 # Shared utilities
└── tests/                      # Integration tests
```

## Debugging Tips

### Binary Not Using Latest Code?

Check which binary is being executed:
```bash
which cargo-rustdoc-fmt
ls -la ~/.cargo/bin/cargo-rustdoc-fmt
```

Compare the timestamp with your last code change. If it's older, run:
```bash
cargo install --path . --force
```

### Test Shows Different Behavior Than Binary?

This usually means you forgot to run `cargo install`. Tests use the compiled code directly from `target/`, but the binary in `~/.cargo/bin` is a separate file.
