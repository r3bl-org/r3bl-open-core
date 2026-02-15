# Claude Code Instructions for r3bl-open-core

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes—slow, steady, and careful work beats fast and careless.

## Design Philosophy

Prioritize low cognitive load, progressive disclosure, and type-safe design. Make illegal states
unrepresentable. See `design-philosophy` skill for principles and patterns.

## Crate-Specific Instructions

Some crates have additional instructions in their own `CLAUDE.md` files:

- **build-infra/**: Provides CLI tools (binaries). **After making code changes, you MUST run
  `cargo install --path build-infra --force`** to update the installed binaries in
  `~/.cargo/bin`. See `build-infra/CLAUDE.md` for details.

When working on a specific crate, always check for a local `CLAUDE.md` file in that crate's
directory for additional workflow requirements.

## Available Skills

This project uses skills to organize coding patterns and workflows. Claude will autonomously
discover and use these when relevant. All skills are in `.claude/skills/`.

### Design

- **design-philosophy** - Core principles: cognitive load, progressive disclosure, type safety,
  abstraction worth. Use when designing APIs, modules, or data structures.
  - Supporting file: `patterns.md` (good/bad examples and quick reference)

### Code Quality & Style

- **check-code-quality** - Comprehensive quality checklist (check → build → docs → clippy → tests). Use after completing code changes and before creating commits.
  - Supporting file: `reference.md` (detailed cargo command reference)

- **run-clippy** - Clippy linting, comment punctuation, cargo fmt. Use after code changes and before creating commits.
  - Supporting file: `patterns.md` (code style patterns and examples)

### Documentation

- **write-documentation** - Consolidated documentation skill covering structure (inverted pyramid),
  intra-doc links, constant conventions, and formatting. Use proactively when writing code with
  rustdoc comments, or retroactively via `/fix-intradoc-links`, `/fix-comments`, `/fix-md-tables`.
  - Supporting files: `link-patterns.md`, `constant-conventions.md`, `examples.md`, `rustdoc-formatting.md`

### Architecture & Patterns

- **organize-modules** - Private modules with public re-exports (barrel export pattern), conditional visibility for docs/tests. Use when creating or organizing modules.
  - Supporting file: `examples.md` (6 complete module organization examples)

- **check-bounds-safety** - Type-safe Index/Length patterns for arrays, cursors, viewports, and terminal cursor movement. Includes `TermRowDelta`/`TermColDelta` for safe relative cursor movements that prevent CSI zero bugs. Distinguishes navigation (`-` returns index) from measurement (`distance_from()` returns length). Use when working with bounds-sensitive code.
  - Supporting file: `decision-trees.md` (visual decision trees and flowcharts)

### Performance

- **analyze-performance** - Flamegraph-based performance regression detection. Use when optimizing or investigating performance.
  - Supporting file: `baseline-management.md` (when and how to update baselines)

### Log Analysis

- **analyze-log-files** - Strip ANSI escape sequences from log files before analysis. Use when asked to process, read, or analyze log files that may contain terminal escape codes.

## Slash Commands for Skills

You can explicitly invoke skills using slash commands:

- `/check` → check-code-quality
- `/docs` → write-documentation
- `/clippy` → run-clippy
- `/fix-intradoc-links` → write-documentation (focused on links)
- `/check-regression` → analyze-performance
- `/analyze-logs` → analyze-log-files (strips ANSI codes from `log.txt`)
- `/r3bl-task` → Task management (see below)

## Running Checks

**Always use `check.fish`** instead of running cargo commands directly. `check.fish` provides ICE
recovery, stale artifact cleanup, config change detection, toolchain validation, and tmpfs/ionice
optimizations — all of which are lost with direct cargo calls.

| Command | What it runs |
|:--------|:-------------|
| `./check.fish --check` | `cargo check` (fast typecheck) |
| `./check.fish --build` | `cargo build` (compile production) |
| `./check.fish --clippy` | `cargo clippy --all-targets` (linting) |
| `./check.fish --test` | `cargo test` + doctests |
| `./check.fish --doc` | `cargo doc --no-deps` (quick docs) |
| `./check.fish --quick-doc` | `cargo doc --no-deps` (fastest, no staging/sync) |
| `./check.fish --full` | All of the above + Windows cross-compilation check |

Commands with **no check.fish equivalent** (run directly):
- `cargo rustdoc-fmt` — format rustdoc comments
- `cargo clippy --all-targets --fix --allow-dirty` — auto-fix lints
- `cargo fmt --all` — format code

## Rust Code Guidelines

### Writing Rustdoc Comments

When writing or modifying rustdoc comments in code, **proactively apply** these conventions
(all documented in `write-documentation` skill):

1. **Intra-doc links**: Use `crate::` paths (not `super::`), reference-style links at bottom of
   doc blocks. See `link-patterns.md` for patterns.

2. **Human-readable constants**: Use binary for bitmasks (`0b0110_0000`), byte literals for
   printable chars (`b'['`), decimal for non-printables (`27`). Show hex in comments for
   cross-reference. See `constant-conventions.md`.

3. **Inverted pyramid**: High-level concepts at module/trait level, simple syntax examples at
   method level. See `examples.md`.

Don't wait for `check-code-quality` to catch issues - write docs correctly the first time.

### Cross-Platform Verification

When working with platform-specific code (`#[cfg(unix)]`, `#[cfg(not(unix))]`), verify Windows
compatibility without needing mingw-w64:

```bash
cargo rustc -p <crate_name> --target x86_64-pc-windows-gnu -- --emit=metadata
```

This performs type checking and borrow checking for Windows without code generation or linking.
Use after modifying `DirectToAnsi` input handling or other Unix-specific code.

### Testing Interactive Terminal Applications

For testing interactive terminal applications, use (both are installed):

- `tmux`
- `screen`

## Git Workflow

- Never commit unless explicitly asked
- When you do make commits, do not add an attribution to yourself in the commit message. Do not add
  the following trailing lines in a commit message:

  ```
  🤖 Generated with [Claude Code](https://claude.com/claude-code)

  Co-Authored-By: Claude <noreply@anthropic.com>
  ```

### Commit Message Format

When a commit implements work from a `task/*.md` file, add a `Task:` trailer as the last line:

```
[scope] Short summary of the change

Optional body with more detail.

Task: task/some-task-name.md
```

The `Task:` trailer links the commit to its plan/design document for traceability.

## Task Tracking System

Two-file system: active work in `todo.md`, completed work in `done.md`.

### todo.md - Active Work

- Check at session start for current state
- Latest changes at top
- Mark completed tasks with `[x]`
- Keep partial sections with mixed completion states
- Maintain task hierarchy with 2-space indentation

### done.md - Archive

- Latest changes at top for historical reference
- Move complete sections only (all subtasks `[x]`)
- Never move individual tasks - preserve context
- Example: Move "fix md parser" only after all 200+ subtasks complete

### Task Format

- `[x]` completed, `[ ]` pending
- Group under descriptive headers
- Include GitHub issue links
- Add technical notes for complex tasks

### The "./task/" folder

The custom slash command "/r3bl-task" is available to manage all the details of a long running task. The
"todo.md" and "done.md" files are simply "pointers" to what tasks are active and which ones are
done. For the details and to create, update, or load a task, use the "/r3bl-task" command.

---

## Skill Details

For detailed information on any skill, see `.claude/skills/<skill-name>/SKILL.md`. Each skill includes:

- **SKILL.md** - Main instructions and workflow
- **Supporting files** - Detailed examples, patterns, references, and decision trees

The skills contain all the detailed guidance that was previously in this file, now organized for autonomous discovery and reuse.
