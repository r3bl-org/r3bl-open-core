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

## Quick Approach (Recommended)

Run the comprehensive check script which handles everything automatically:

```bash
./check.fish --full
```

This runs **all checks** in order: typecheck → build → clippy → tests → doctests → docs

**Benefits of using `check.fish --full`:**
- **ICE recovery**: Automatically cleans cache and retries on Internal Compiler Errors
- **Toolchain escalation**: If ICE persists, escalates to `rust-toolchain-update.fish` to find a stable nightly
- **Config change detection**: Auto-cleans stale artifacts when Cargo.toml or toolchain changes
- **Performance optimized**: Uses tmpfs, ionice, and parallel jobs for speed

### Other check.fish Commands

For granular control, use individual commands:

| Command | What it runs |
|:--------|:-------------|
| `./check.fish --check` | `cargo check` (fast typecheck) |
| `./check.fish --build` | `cargo build` (compile production) |
| `./check.fish --clippy` | `cargo clippy --all-targets` (linting) |
| `./check.fish --test` | `cargo test` + doctests |
| `./check.fish --doc` | `cargo doc --no-deps` (quick docs) |
| `./check.fish --quick-doc` | `cargo doc --no-deps` (fastest, no staging/sync) |
| `./check.fish --full` | All of the above + lychee link rot check |

## Step-by-Step Approach (Alternative)

If you need more control or want to run checks manually:

### 1. Fast Typecheck

```bash
./check.fish --check
# (runs: cargo check)
```

Quickly verifies the code compiles without generating artifacts.

### 2. Compile Production Code

```bash
./check.fish --build
# (runs: cargo build)
```

Ensures production code builds successfully.

### 3. Format Rustdoc Comments

Invoke the `write-documentation` skill to format rustdoc comments using `cargo rustdoc-fmt`.

This formats markdown tables and converts inline links to reference-style.

### 4. Generate Documentation

```bash
./check.fish --quick-doc
# (runs: cargo doc --no-deps, directly to serving dir - fastest for iteration)
```

Verify there are no documentation build warnings or errors. Use `--quick-doc` for fast feedback
during development. Use `--doc` for final verification before commits (includes staging/sync).

If there are link warnings, use the `/fix-intradoc-links` command to resolve them.

**Heading Anchor (Slug) Integrity**:
If you modified any heading text (e.g., `# My Heading`), the automatically generated HTML anchor
(e.g., `#my-heading`) will change.
- **Identify Changes**: Look for changed headings in git-dirty files.
- **Proactive Search**: Use `grep_search` to find any existing links (e.g., `path#old-slug`)
  that point to the old anchors and update them.
- **Validation**: While `cargo doc` warns about many broken fragments, proactive searching
  prevents "orphan" links in external documentation or complex intra-doc paths.

**CRITICAL: Never remove intra-doc links to fix warnings.** When you encounter:
- Unresolved link to a symbol → Fix the path using `crate::` prefix (see `write-documentation` skill)
- Unresolved link to a test module → Add `#[cfg(any(test, doc))]` visibility (see `organize-modules` skill)
- Unresolved link to a platform-specific module → Use `#[cfg(all(any(test, doc), target_os = "..."))]`

Links provide refactoring safety - `cargo doc` catches stale references. Converting to plain backticks removes this protection.

### 5. Link Rot Check (External URLs)

Included automatically in `./check.fish --full`. Runs `lychee` on git-modified files to detect
broken external URLs in rustdoc comments.

`cargo doc --no-deps` (step 4) validates intra-doc links but not external HTTP/HTTPS URLs.
lychee fills that gap. Config in `lychee.toml` (repo root) excludes known false positives
(example `file://` URIs, test fixture URLs, sites that block automated requests).

If lychee reports 404s, fix the URL by finding the new location. See the task file
`task/add-lychee-to-detect-link-rot.md` for the full categorization of findings.

### 6. Linting

```bash
./check.fish --clippy
# or invoke the `run-clippy` skill
```

Runs clippy and enforces code style standards.

### 7. Run All Tests

```bash
./check.fish --test
# (runs: cargo test --all-targets && cargo test --doc)
```

Runs all tests (unit, integration, doctests).

If tests fail, use the Task tool with `subagent_type='test-runner'` to fix failures.

### 8. Stress Test (Optional - After Major Refactors)

After major refactors or changes that affect process spawning, PTY tests, or async
infrastructure, run the full test suite 20 times back-to-back to detect flaky regressions:

```bash
for i in {1..20}; do echo "=== Run $i/20 ===" && cargo test --all-targets -- --nocapture 2>&1 | grep -E "^test result:" | head -3 || { echo "FAILED on run $i"; exit 1; }; done && echo "ALL 20 RUNS PASSED"
```

**When to run:**
- After refactoring PTY test infrastructure (`generate_pty_test!`, `spawn_controlled_in_pty`)
- After changes to process lifecycle, signal handling, or async I/O code
- After modifying the resilient reactor thread (RRT) restart logic
- Before merging large cross-cutting changes that touch many test files

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

## ICE Recovery and Toolchain Escalation

The `./check.fish --full` command includes automatic recovery from Internal Compiler Errors:

```
ICE detected → cleanup target/ → retry
                                   ↓
                            still ICE?
                                   ↓
              escalate to rust-toolchain-update.fish
              (searches 46 nightly candidates, validates each)
                                   ↓
                         new stable nightly installed
                                   ↓
                               retry checks
```

This is especially important since we use nightly Rust (for the parallel compiler frontend). Nightly toolchains occasionally have ICE bugs, and this automatic escalation finds a working version.

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

- **`reference.md`** - Comprehensive guide to all cargo commands used in the quality checklist. Includes detailed explanations of what each command does, when to use it, common flags, and build optimizations (wild linker, parallel frontend, tmpfs). **Read this when:**
  - You need to understand what a specific cargo command does
  - Troubleshooting build issues
  - Want to know about build optimizations in `.cargo/config.toml`
  - Understanding the difference between `cargo test --all-targets` and `cargo test --doc`

## Related Skills

- `write-documentation` - For rustdoc formatting (step 3) and fixing doc link warnings (step 4)
- `run-clippy` - For linting and code style (step 5)
- `analyze-performance` - For optional performance checks
- `test-runner` agent - For fixing test failures (step 6)

## Related Commands

- `/check` - Explicitly invokes this skill
