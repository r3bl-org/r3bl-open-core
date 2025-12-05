# Cargo Commands Reference

This document provides detailed explanations of each cargo command used in the quality checklist.

## cargo check

**Purpose:** Fast typecheck without code generation

**When to use:**
- Quick feedback during development
- First step in quality checks (fastest way to catch compile errors)

**What it does:**
- Parses and typechecks code
- Does NOT generate executable binaries
- Much faster than `cargo build`

**Output:**
- Compilation errors
- Type errors
- Borrow checker errors

---

## cargo build

**Purpose:** Compile production code

**When to use:**
- After `cargo check` passes
- To verify the full compilation pipeline works
- Before running the actual binary

**What it does:**
- Full compilation with code generation
- Generates debug build by default (in `target/debug/`)
- Applies all compiler optimizations for debug profile

**Output:**
- Compiled binaries in `target/debug/`
- Compilation warnings and errors

---

## cargo rustdoc-fmt

**Purpose:** Format rustdoc comments (custom tool from build-infra)

**When to use:**
- After writing or modifying documentation comments
- To fix markdown table formatting
- To convert inline links to reference-style

**What it does:**
- Formats markdown tables in rustdoc comments
- Converts `[foo](path)` to reference-style `[foo]` with `[foo]: path` at bottom
- Preserves code examples and other content

**Installation:**
```bash
cd build-infra
cargo install --path . --force
```

**Usage:**
```bash
# Format specific file
cargo rustdoc-fmt path/to/file.rs

# Format all git-changed files
cargo rustdoc-fmt

# Format entire workspace
cargo rustdoc-fmt --workspace
```

---

## cargo doc --no-deps

**Purpose:** Generate documentation and verify no warnings

**When to use:**
- After formatting rustdoc comments
- To verify all intra-doc links resolve correctly
- Before committing documentation changes

**What it does:**
- Generates HTML documentation in `target/doc/`
- Checks all intra-doc links
- Validates rustdoc syntax

**Output:**
- Generated docs in `target/doc/`
- Warnings about broken links
- Errors about invalid rustdoc syntax

**The `--no-deps` flag:**
- Only documents your crate, not dependencies
- Faster doc generation
- Focuses on your code's documentation

---

## cargo clippy --all-targets

**Purpose:** Lint code for common mistakes and style issues

**When to use:**
- After code builds successfully
- To catch potential bugs and anti-patterns
- To enforce consistent style

**What it does:**
- Runs hundreds of linting rules
- Checks for correctness, performance, style issues
- Suggests idiomatic Rust patterns

**Output:**
- Linting warnings and errors
- Suggestions for improvements

**Auto-fix:**
```bash
cargo clippy --all-targets --fix --allow-dirty
```

**The `--all-targets` flag:**
- Lints main code, tests, benches, examples
- Ensures test code also follows standards

---

## cargo test --no-run

**Purpose:** Compile test code without running tests

**When to use:**
- After production code compiles
- To verify test code compiles
- Faster than running all tests

**What it does:**
- Compiles all test code
- Does NOT execute tests
- Catches test compilation errors early

**Output:**
- Compiled test binaries in `target/debug/deps/`
- Compilation errors in test code

---

## cargo test --all-targets

**Purpose:** Run all tests (unit, integration, examples, benches)

**When to use:**
- After test code compiles
- To verify functionality
- Before committing code

**What it does:**
- Runs all tests in the workspace
- Includes unit tests, integration tests, doc tests in lib.rs
- Does NOT run doc tests in other files (use `cargo test --doc` for that)

**Output:**
- Test results (passed/failed)
- Test output and failures

**Common flags:**
```bash
# Run specific test
cargo test test_name

# Show test output even if passing
cargo test -- --nocapture

# Run ignored tests
cargo test -- --ignored
```

**The `--all-targets` flag:**
- Tests lib.rs, bin/*.rs, tests/*.rs, examples/*.rs, benches/*.rs
- Comprehensive test coverage

---

## cargo test --doc

**Purpose:** Run documentation examples (doctests)

**When to use:**
- After `cargo test --all-targets` passes
- To verify documentation examples work
- Before committing documentation

**What it does:**
- Extracts code blocks from rustdoc comments
- Compiles and runs them as tests
- Verifies examples are correct

**Output:**
- Doctest results
- Compilation or runtime errors in examples

**Example doctest:**
```rust
/// Adds two numbers.
///
/// ```
/// let result = my_crate::add(2, 3);
/// assert_eq!(result, 5);
/// ```
pub fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

---

## cargo rustc --target x86_64-pc-windows-gnu -- --emit=metadata

**Purpose:** Cross-platform verification without full cross-compilation

**When to use:**
- After adding `#[cfg(unix)]` or `#[cfg(not(unix))]` attributes
- When modifying platform-specific code paths
- To verify Windows compatibility of Unix-gated code

**What it does:**
- Checks Rust code compiles for Windows target
- Uses `--emit=metadata` to skip code generation and linking
- Avoids need for mingw-w64 or Windows cross-compiler
- Validates all `#[cfg]` gates resolve correctly

**Why `--emit=metadata`:**
- Full cross-compilation requires platform-specific linkers (mingw-w64)
- Metadata-only mode performs type checking and borrow checking
- Still processes all `#[cfg(...)]` attributes
- Catches import errors, missing types, and cfg-gated code path issues

**Example:**
```bash
# Check specific crate for Windows compatibility
cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata

# If successful, your #[cfg] gates are correctly configured
```

**Prerequisites:**
- Windows target installed: `rustup target add x86_64-pc-windows-gnu`
- Automatically installed by `bootstrap.sh` and toolchain scripts

**Common issues this catches:**
- Missing `#[cfg(not(unix))]` fallback for Unix-only code
- Imports of Unix-only types without cfg gates
- Platform-specific dependencies not properly gated in Cargo.toml

---

## Build Optimizations (Configured in .cargo/config.toml)

The project uses several build optimizations:

### Parallel Compilation

`-Z threads=8` for nightly builds

**What it does:**
- Uses 8 threads for compilation
- Speeds up large builds

### Wild Linker (Linux only)

Fast alternative linker when `clang` and `wild-linker` are installed.

**What it does:**
- Faster linking than default linker
- Significantly improves iteration time
- Auto-configured via bootstrap.sh

**Check if active:**
```bash
cat .cargo/config.toml | grep -A5 "target.x86_64-unknown-linux-gnu"
```

---

## Quality Check Workflow Summary

```
cargo check          → Fast typecheck (catch errors early)
cargo build          → Full compilation (verify builds)
cargo rustdoc-fmt    → Format docs (via skill)
cargo doc --no-deps  → Generate docs (verify links)
cargo clippy         → Lint code (via skill)
cargo test --no-run  → Compile tests (verify test code)
cargo test --all     → Run tests (verify functionality)
cargo test --doc     → Run doctests (verify examples)
```

This ensures:
- ✅ Code compiles
- ✅ Documentation builds
- ✅ Code is idiomatic
- ✅ Tests pass
- ✅ Examples work

Ready to commit!
