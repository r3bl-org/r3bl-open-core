# Claude Code Instructions for r3bl-build-infra

This crate provides CLI tools for the r3bl workspace. After making code changes, you
**must** reinstall the binary for the changes to take effect.

## Important: Binary Installation Workflow

This crate provides command-line tools (binaries) that are installed to `~/.cargo/bin`:

- `cargo-rustdoc-fmt`: Format markdown tables, convert inline links, auto-link technical
  terms, and normalize code fences in rustdoc comments

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
    # Test cargo-rustdoc-fmt on an actual file
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
- See root `AGENTS.md` for general Rust guidelines

### Testing Best Practices

- Write unit tests (Tier 1) inside each module (`cst/`, `formatters/`, `processor.rs`,
  `cli_arg.rs`) for isolated functions and options
- Write end-to-end golden-file tests (Tier 2) in
  `src/cargo_rustdoc_fmt/validation_tests/complete_file_tests.rs`
- Use test fixtures in `src/cargo_rustdoc_fmt/validation_tests/test_data/complete_file/`
  for complete `.rs` file test cases
- Include both positive and negative test cases

## Project Structure

```
build-infra/
├── src/
│   ├── bin/                             # Binary entry points
│   │   └── cargo-rustdoc-fmt.rs
│   ├── cargo_rustdoc_fmt/               # cargo-rustdoc-fmt implementation
│   │   ├── cli_arg.rs                   # CLI argument parsing
│   │   ├── constants.rs                 # Shared syntax constants and delimiters
│   │   ├── cst/                         # Lossless CST parser, nodes, and slice scanners
│   │   │   ├── mod.rs                   # Barrel re-exports and CST unit tests
│   │   │   ├── source_file_cst.rs       # SourceFileCst, FileChunk, LineEndings, TrailingNewline
│   │   │   ├── doc_block_cst.rs         # DocBlockCst, DocNode, DocCommentLine
│   │   │   ├── link_reference.rs        # LinkReference, reference def & backtick span scanners
│   │   │   └── table_data.rs            # TableData, ColumnAlignment, GFM table parser & formatter
│   │   ├── formatters/                  # Formatting rules that mutate the CST
│   │   │   ├── mod.rs                   # Barrel re-exports for formatters
│   │   │   ├── code_fence.rs            # Code fence language tag normalizer
│   │   │   ├── link.rs                  # Inline-to-reference link converter
│   │   │   ├── table.rs                 # Markdown table column aligner
│   │   │   └── technical_term/          # Technical term auto-linker sub-module
│   │   │       ├── mod.rs               # Technical term barrel re-exports
│   │   │       ├── link_terms_entry_point.rs # CST block orchestrator & node walkers
│   │   │       ├── upgrade_term.rs      # Single-line term upgrading
│   │   │       ├── markdown_spans.rs    # Protected Markdown spans & multi-line state scanners
│   │   │       ├── ref_defs.rs          # Reference definition reconciliation & CST layout
│   │   │       ├── technical_term_dictionary.rs # Technical term dictionary loader
│   │   │       └── known_technical_term_link_dictionary.jsonc # Seed dictionary
│   │   ├── processor.rs                 # File processing orchestration
│   │   ├── types.rs                     # Domain types and CLI options
│   │   ├── ui_str.rs                    # User-facing strings
│   │   └── validation_tests/            # End-to-end validation tests with fixtures
│   │       ├── mod.rs                   # Testing philosophy and module coordinator
│   │       ├── complete_file_tests.rs   # Golden-file and idempotency E2E tests
│   │       └── test_data/               # Input and expected_output fixtures
│   └── common/                          # Shared utilities
└── README.md
```

## Skipping Files

- **Per-file directive**: Add `// rustdoc-fmt: skip` to any `.rs` file to exclude it from
  `cargo rustdoc-fmt` processing. Place it after the copyright line. All source files in
  `cargo_rustdoc_fmt/` use this marker because their doc comments discuss the formatter's
  own concepts (e.g., CSI, ANSI) which would be incorrectly linkified.
- **Folder pattern matching**: By default, `cargo rustdoc-fmt` skips any file whose path
  components contain `test_data` (`--skip-folder-matches=test_data`). External crates can
  customize this with comma-separated substrings (e.g.,
  `--skip-folder-matches=testdata,snapshots`) or disable it with
  `--skip-folder-matches=""`.

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

This usually means you forgot to run `cargo install`. Tests use the compiled code directly
from `target/`, but the binary in `~/.cargo/bin` is a separate file.
