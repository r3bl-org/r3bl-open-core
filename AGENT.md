# Agent Instructions for r3bl-open-core

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes—slow, steady, and careful work beats fast and careless.

## Design Philosophy

Prioritize low cognitive load, progressive disclosure, and type-safe design. Make illegal states
unrepresentable. See `design-philosophy` skill for principles and patterns.

## Tooling & Capabilities

**A. Semantic Rust Tools (MCP):** When connected to a Rust MCP server (`rust-refactor`), use its
tools for code navigation (go-to-definition, finding references), deep architectural analysis
(call graphs), and precise compiler-driven refactoring. Always provide precise `file_path`,
`line`, and `character` coordinates.

**B. Local Workflows (.agent/):** For repo-specific workflows (clippy, formatting, log analysis),
capabilities are defined in the `.agent/` directory. When a task matches a skill, agent, or
command:
1. Look inside the `.agent/` directory.
2. Read the markdown instructions inside that folder.
3. Execute the underlying shell/scripts exactly as instructed.

## Context Guardrail

You do not have the full codebase in memory. Actively use search and file-reading tools to gather
local context. If a request requires system-wide knowledge, global refactoring, or sweeping
architectural changes, **DO NOT GUESS**. Stop and ask the user to provide broader context.

## Research Efficiency

- **Batch tool calls:** Execute research and file-reading tools in parallel to build context rapidly.
- **Deep investigation:** When mapping unfamiliar layers, proactively use multiple search and read
  calls in a single turn.
- **Autonomous progress:** In autonomous mode, do not stop for minor clarifications. Complete
  research and propose a high-signal plan before pausing.
- **Milestone delivery:** Aim for one high-signal turn (e.g., a complete research summary or task
  file) rather than many low-signal turns.

## Skills, Agents & Commands Location

All skills, agents, and slash commands are in the `.agent/` directory (not `.claude/`).
When loading a skill, agent, or command, look in `.agent/skills/`, `.agent/agents/`,
and `.agent/commands/` respectively.

## Crate-Specific Instructions

Some crates have additional instructions in their own `AGENT.md` files:

- **build-infra/**: Provides CLI tools (binaries). **After making code changes, you MUST run
  `cargo install --path build-infra --force`** to update the installed binaries in
  `~/.cargo/bin`. See `build-infra/AGENT.md` for details.

- **tui/**: Main crate (`r3bl_tui`). PTY test architecture:
  - Tests use `generate_pty_test!` macro for single-feature PTY tests
  - `spawn_controlled_in_pty()` for multi-backend comparison tests
  - Use Controller/Controlled terminology (not master/slave)
  - `drain_pty_and_wait()` prevents macOS PTY buffer deadlocks
  - `try_clone_reader()` returns owned `Box<dyn Read>` (not a borrow), so reader and PtyPair
    are independent

When working on a specific crate, always check for a local `AGENT.md` file in that crate's
directory for additional workflow requirements.

## Available Skills

This project uses skills to organize coding patterns and workflows. All skills are in
`.agent/skills/`. When loading a skill, also check for and read any supporting `.md` files
in that skill's directory (e.g., `patterns.md`, `reference.md`, `examples.md`).

### Design

- **design-philosophy** - Core principles: cognitive load, progressive disclosure, type safety,
  abstraction worth. Use when designing APIs, modules, or data structures.
  - Supporting file: `patterns.md` (good/bad examples and quick reference)

### Code Quality & Style

- **check-code-quality** - Comprehensive quality checklist (check -> build -> docs -> clippy -> tests).
  Use after completing code changes and before creating commits.
  - Supporting file: `reference.md` (detailed cargo command reference)

- **run-clippy** - Clippy linting, comment punctuation, cargo fmt. Use after code changes and
  before creating commits.
  - Supporting file: `patterns.md` (code style patterns and examples)

### Documentation

- **write-documentation** - Consolidated documentation skill covering structure (inverted pyramid),
  intra-doc links, constant conventions, and formatting. Use proactively when writing code with
  rustdoc comments, or retroactively via `/fix-intradoc-links`, `/fix-comments`, `/fix-md-tables`.
  - Supporting files: `link-patterns.md`, `constant-conventions.md`, `examples.md`,
    `rustdoc-formatting.md`

### Architecture & Patterns

- **organize-modules** - Private modules with public re-exports (barrel export pattern), conditional
  visibility for docs/tests. Use when creating or organizing modules.
  - Supporting file: `examples.md` (6 complete module organization examples)

- **check-bounds-safety** - Type-safe Index/Length patterns for arrays, cursors, viewports, and
  terminal cursor movement. Includes `TermRowDelta`/`TermColDelta` for safe relative cursor
  movements that prevent CSI zero bugs. Use when working with bounds-sensitive code.
  - Supporting file: `decision-trees.md` (visual decision trees and flowcharts)

### Performance

- **analyze-performance** - Flamegraph-based performance regression detection. Use when optimizing
  or investigating performance.
  - Supporting file: `baseline-management.md` (when and how to update baselines)

### Release

- **release-crate** - Full crate release workflow: version bump, changelog, publish to crates.io,
  git tag, GitHub release. Use when releasing a new version of any workspace crate.

### Log Analysis

- **analyze-log-files** - Strip ANSI escape sequences from log files before analysis. Use when
  asked to process, read, or analyze log files that may contain terminal escape codes.

## Available Agents (`.agent/agents/`)

| Agent | Purpose |
|:------|:--------|
| **test-runner** | Expert in running tests and fixing failures |
| **clippy-runner** | Expert in linting and fixing style issues |
| **code-formatter** | Expert in bulk code formatting |
| **perf-checker** | Expert in performance regression analysis |

## Slash Commands

| Command | Skill |
|:--------|:------|
| `/check` | check-code-quality |
| `/clippy` | run-clippy |
| `/docs` | write-documentation |
| `/fix-intradoc-links` | write-documentation (focused on links) |
| `/fix-comments` | write-documentation (constant conventions) |
| `/fix-md-tables` | write-documentation (table formatting) |
| `/check-regression` | analyze-performance |
| `/analyze-logs` | analyze-log-files |
| `/release` | release-crate |
| `/r3bl-task` | Task management (see below) |
| `/boxes` | Unicode box-drawing character set |

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
| `./check.fish --doc` | `cargo doc --workspace` (full, with dep-doc caching) |
| `./check.fish --quick-doc` | `cargo doc --workspace --no-deps` (fastest, no staging/sync) |
| `./check.fish --full` | All of the above + Windows cross-compilation check + lychee link rot check |

Commands with **no check.fish equivalent** (run directly):
- `cargo rustdoc-fmt` — format rustdoc comments
- `cargo clippy --all-targets --fix --allow-dirty` — auto-fix lints
- `cargo fmt --all` — format code

## Rust Code Guidelines

### Writing Rustdoc Comments

When writing or modifying rustdoc comments in code, **proactively apply** these conventions
(all documented in `write-documentation` skill):

1. **Intra-doc links**: Prefer `crate::` paths (shorter). Use `super::` when `crate::` paths
   get too long and symbols are co-located. Reference-style links at bottom of doc blocks.
   See `link-patterns.md` for patterns.

2. **Human-readable constants**: Use binary for bitmasks (`0b0110_0000`), byte literals for
   printable chars (`b'['`), decimal for non-printables (`27`). Show hex in comments for
   cross-reference. See `constant-conventions.md`.

3. **Inverted pyramid**: High-level concepts at module/trait level, simple syntax examples at
   method level. See `examples.md`.

4. **Sidebar headings**: Only `#` and `##` headings appear in the rustdoc sidebar navigation.
   Use `**bold**` text instead of `###` for sub-sections within doc comments.

5. **No em dashes**: Use regular dashes (`-`), never em dashes (`—`) in documentation.

Don't wait for `check-code-quality` to catch issues - write docs correctly the first time.

### Macro Imports

Do NOT use `#[macro_use]` on module declarations. For `#[macro_export]` macros, use explicit
imports: `use crate::macro_name;`. Each `mod` block that uses a macro needs its own import -
parent scope imports don't propagate into child modules.

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

- Never use `git stash` / `git stash pop` to test against clean state - it destroys the staging
  area (index). Use the Task tool with `isolation: "worktree"` to run tests in a separate git
  worktree without touching the main working tree.
- Use `git mv` instead of `mv` when moving or renaming files to preserve move history in git.
- Never commit unless explicitly asked
- When you do make commits, do not add an attribution to yourself in the commit message.
  Do not add the following trailing lines (or similar) in a commit message:

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

The `/r3bl-task` slash command is available to manage all the details of a long-running
task. All tasks are stored in the `./task/` directory as individual Markdown files.

### Folder Structure

- `task/` - Active tasks (currently being worked on).
- `task/pending/` - Future tasks (not yet started).
- `task/done/` - Completed tasks.
- `task/archive/` - Abandoned tasks kept for historical reference.

See `task/AGENT.md` for detailed rules on managing individual task files.

---

## Skill Details

For detailed information on any skill, see `.agent/skills/<skill-name>/SKILL.md`. Each skill
includes:

- **SKILL.md** - Main instructions and workflow
- **Supporting files** - Detailed examples, patterns, references, and decision trees

The skills contain all the detailed guidance that was previously in this file, now organized for
autonomous discovery and reuse.
