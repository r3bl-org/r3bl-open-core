---
name: check-code-quality
description: Run comprehensive Rust code quality checks including compilation, linting, documentation, and tests. Use after completing code changes and before creating commits.
---

# Rust Code Quality Checks

## When to Use

- After completing significant code changes
- Before creating commits
- Before creating pull requests
- When user says "check code quality", "run quality checks", "make sure code is good", etc.

## Instructions

Run this essential quality checklist in order. These are the core checks that must pass before committing code:

### 1. Fast Typecheck

```bash
cargo check
```

Quickly verifies the code compiles without generating artifacts.

### 2. Compile Production Code

```bash
cargo build
```

Ensures production code builds successfully.

### 3. Format Rustdoc Comments

Invoke the `write-documentation` skill to format rustdoc comments using `cargo rustdoc-fmt`.

This formats markdown tables and converts inline links to reference-style.

### 4. Generate Documentation

```bash
cargo doc --no-deps
```

Verify there are no documentation build warnings or errors.

If there are link warnings, invoke the `fix-intradoc-links` skill to resolve them.

**CRITICAL: Never remove intra-doc links to fix warnings.** When you encounter:
- Unresolved link to a symbol → Fix the path using `crate::` prefix (see `write-documentation` skill)
- Unresolved link to a test module → Add `#[cfg(any(test, doc))]` visibility (see `organize-modules` skill)
- Unresolved link to a platform-specific module → Use `#[cfg(all(any(test, doc), target_os = "..."))]`

Links provide refactoring safety - `cargo doc` catches stale references. Converting to plain backticks removes this protection.

### 5. Linting

Invoke the `run-clippy` skill to run clippy and enforce code style standards.

### 6. Compile Test Code

```bash
cargo test --no-run
```

Ensures test code compiles without running the tests.

### 7. Run All Tests

```bash
cargo test --all-targets
```

Runs all tests (unit, integration, etc.) but **does not run doctests**.

If tests fail, use the Task tool with `subagent_type='test-runner'` to fix failures.

### 8. Run Doctests

```bash
cargo test --doc
```

Runs documentation examples to ensure they work correctly.

### 9. Cross-Platform Verification (Optional)

For code with platform-specific `#[cfg]` gates (especially Unix-only code), verify Windows compatibility:

```bash
cargo rustc -p <crate_name> --target x86_64-pc-windows-gnu -- --emit=metadata
```

This checks that `#[cfg(unix)]` and `#[cfg(not(unix))]` gates compile correctly on Windows without needing a full cross-compiler toolchain.

**When to run:**
- After adding or modifying `#[cfg(unix)]` or `#[cfg(target_os = "...")]` attributes
- When working on platform-abstraction code
- Before committing changes to `DirectToAnsi` input handling or other Unix-specific code

## Reporting Results

After running all checks, report results concisely to the user:

- ✅ All checks passed → "All quality checks passed! Ready to commit."
- ⚠️ Some checks failed → Summarize which steps failed and what needs fixing
- 🔧 Auto-fixed issues → Report what was automatically fixed

## Optional Performance Analysis

For performance-critical code changes, consider also running:

- `cargo bench` - Benchmarks (mark tests with `#[bench]`)
- `cargo flamegraph` - Profiling (requires flamegraph crate)
- Invoke the `analyze-performance` skill for flamegraph-based regression detection

## Supporting Files in This Skill

This skill includes additional reference material:

- **`reference.md`** - Comprehensive guide to all cargo commands used in the quality checklist. Includes detailed explanations of what each command does, when to use it, common flags, and build optimizations (sccache, wild linker, etc.). **Read this when:**
  - You need to understand what a specific cargo command does
  - Troubleshooting build issues
  - Want to know about build optimizations in `.cargo/config.toml`
  - Understanding the difference between `cargo test --all-targets` and `cargo test --doc`

## Related Skills

- `write-documentation` - For rustdoc formatting (step 3)
- `fix-intradoc-links` - For fixing doc link warnings (step 4)
- `run-clippy` - For linting and code style (step 5)
- `analyze-performance` - For optional performance checks
- `test-runner` agent - For fixing test failures (step 7)

## Related Commands

- `/check` - Explicitly invokes this skill
