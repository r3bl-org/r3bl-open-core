# Agent Instructions for r3bl-open-core

## AI Agent Security & System Integrity Mandates

To prevent catastrophic system failures, all AI agents (Gemini, Claude, etc.) MUST adhere to these strict guardrails. These mandates take absolute precedence over any "YOLO" mode or perceived "fixes."

### 1. Critical Directory Protection
Recursive operations (`chown -R`, `chmod -R`, `rm -rf`) are STRICTLY PROHIBITED on the following top-level system directories and their contents:
- `/` (Root)
- `/usr` (System binaries and libraries)
- `/etc` (System configuration)
- `/bin`, `/sbin`, `/lib`, `/lib64` (Essential system paths)
- `/boot` (Bootloader and kernels)
- `/var` (Variable data, including system logs and databases)

### 2. Ownership & Integrity
- **Root Ownership:** System directories and binaries MUST remain owned by `root`. The agent must NEVER suggest or execute a change of ownership for system-managed paths to a non-root user.
- **Privilege Escalation:** Do not modify the `setuid` or `setgid` bits of any system binary (e.g., `sudo`, `pkexec`, `mount`) unless specifically instructed by the user to fix a verified corruption.

### 3. Execution Safety
- **Explicit Paths Only:** All `sudo` commands involving recursive changes or deletions MUST use absolute paths. The use of wildcards (`*`) or relative paths (`.`) with `sudo chown/chmod/rm` is forbidden.
- **Verification First:** Before suggesting a permissions fix, the agent must first verify the current state using `ls -ld` or `stat`.
- **Destructive Warning:** Any command that modifies system-wide permissions or ownership must be explicitly flagged to the user with a explanation of the risks, even in YOLO mode.

---

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes—slow, steady, and careful work beats fast and careless.

## Standard Workflow (Alignment -> Plan -> Execute)

To ensure safety and alignment, always start by clarifying the scope of work. Ask the user:
"Are we starting:
1. a **new task**,
2. continuing an **existing task**, or
3. doing **one-off work**?
(Please respond with 1, 2, or 3)"

### 1. New Task (Plan -> Task File -> Execute)
Follow this "slow and steady" workflow for all non-trivial changes:
- **In-Chat Planning:** Research the problem and present a comprehensive plan in chat for
  refinement. Use code examples and specifics.
- **Task File Creation:** Once approved, formalize it via `/r3bl-task create <name>`.
- **Manual Review:** Wait for the user to manually review and **explicitly approve** the task
  file before starting implementation.
- **Iterative Implementation:** Implement step-by-step, using `/r3bl-task update <name>`.

### 2. Existing Task
- **Load Task:** Identify the active task in `task/` and use `/r3bl-task load <name>`.
- **Resume:** Resume work from the next unchecked step after confirming with the user.

### 3. One-off Work
- For simple, isolated changes that do not require formal planning or task tracking,
  proceed directly to research and implementation.

## Progress & Review Guardrails (Anti-Hallucination)

To prevent large-scale destructive errors, "hallucinations," or accidental deletions during
complex or long-running tasks, you MUST follow these loop-in-the-user rules:

1. **Frequent Review Points:** Do not perform more than 3-5 consecutive file modifications
   without pausing to summarize progress and request user verification.
2. **Milestone Stability:** Stop and ask for a review as soon as you achieve any stable
   milestone (e.g., code compiles after a refactor, a sub-module is renamed, or a
   complex regex operation is completed).
3. **Validation before Review:** Always run `./check.fish --check` or `cargo check`
   locally BEFORE asking the user for a review. Never present "broken" progress.
4. **Attention Signal:** When stopping for a mandatory review point, run `fish -c "beep"`
   to alert the user.
5. **Mandatory Manual Review:** A task, phase, or sub-phase is not complete until the
   user has performed a manual review. This is the final step in the verification
   lifecycle. Do not mark a task as done in the task file until this review is
   successfully completed.
   - **Automatic Requirement:** You MUST automatically add a "Mandatory manual review"
     step with a checkbox list of all modified files to the end of every task, phase,
     and sub-phase you create or update.
   - **Review Workflow:** When the user prompts for a manual review at the end of a
     task/phase/sub-phase:
     1. Use `codium <file_path>` to open the first file with a checkbox.
     2. Ask the user to manually review it.
     3. Once the user confirms ("good" or similar), check the box in the task file.
     4. Move to the next file and repeat until all checkboxes are checked.
6. **Strict Documentation Preservation:** Documentation is as critical as code. Any
   surgical edit that touches doc comments must be byte-perfect in its preservation of
   surrounding text.
8. **Post-Edit Rustdoc Verification:** After any file modification, especially when
   using `write_file`, you MUST verify that you have NOT clobbered pre-existing
   and valid rustdoc comments, diagrams, or module-level documentation. This
   is a high-priority check to maintain documentation integrity.
9. **Human-in-the-Loop:** When in doubt, or when a task involves global renames, stop and
   confirm the plan for the NEXT 3 files before touching them.

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
  research and propose a high-signal plan in chat before pausing. Always follow the
  **Standard Workflow** and do not skip the alignment or approval steps.
- **Milestone delivery:** Aim for one high-signal turn (e.g., a complete research summary or
  initial chat plan) rather than many low-signal turns.

## Skills, Agents & Commands Location

All skills, agents, and slash commands are in the `.agent/` directory (not `.claude/`).
When loading a skill, agent, or command, look in `.agent/skills/`, `.agent/agents/`,
and `.agent/commands/` respectively.

## Crate-Specific Instructions

Some crates have additional instructions in their own `AGENTS.md` files:

- **build-infra/**: Provides CLI tools (binaries). **After making code changes, you MUST run
  `cargo install --path build-infra --force`** to update the installed binaries in
  `~/.cargo/bin`. See `build-infra/AGENTS.md` for details.

- **tui/**: Main crate (`r3bl_tui`). For test directory taxonomy, PTY integration
  test conventions, and subprocess isolation patterns, use the `organize-tests` skill.

When working on a specific crate, always check for a local `AGENTS.md` file in that crate's
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

- **organize-tests** - Test directory taxonomy (why a test is isolated), PTY conventions (Run with section, deadlock prevention), and isolated process orchestration. Use when adding or refactoring tests.
  - Supporting files: `taxonomy.md` (directory guide), `pty-conventions.md` (PTY rules), `examples.md` (macro templates)

- **check-bounds-safety** - Type-safe Index/Length patterns for arrays, cursors, viewports, and
  terminal cursor movement. Includes `TermRowDelta`/`TermColDelta` for safe relative cursor
  movements that prevent CSI zero bugs. Use when working with bounds-sensitive code.
  - Supporting file: `decision-trees.md` (visual decision trees and flowcharts)

- **concurrency-safety** - Thread safety, Chain of Custody, Loud Lock Releases, and AtomicU8Ext patterns. Use when working with threads, locks, or atomics.
  - Supporting file: `patterns.md` (good/bad examples of lock management)

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
compatibility. This performs type checking and borrow checking for the Windows target
without full code generation or linking.

Note: While `--emit=metadata` skips the linking stage, the **mingw-w64 toolchain is still
required** because many core dependencies (like `windows-sys`, `parking_lot`, or `mimalloc`)
have build scripts that probe for `x86_64-w64-mingw32-gcc` and `x86_64-w64-mingw32-dlltool`.

```bash
cargo rustc -p <crate_name> --target x86_64-pc-windows-gnu -- --emit=metadata
```

Use this after modifying `DirectToAnsi` input handling or other Unix-specific code.

### Testing Interactive Terminal Applications

For testing interactive terminal applications, use (both are installed):

- `tmux`
- `screen`

## Git Workflow

- **No Destructive Resets**: NEVER use `git reset HEAD~n`, `git reset --hard`, or `git clean`
  unless explicitly and specifically commanded to do so by the user. These commands are
  destructive to the user's work and staging area.
- **Surgical Unstaging**: When asked to unstage specific files, ALWAYS use targeted
  commands like `git restore --staged <file>`. NEVER unstage the entire index or use
  blanket reset commands if a specific list of files is provided.
- **Respect the Index**: The staging area is the user's carefully curated state. Do not
  perform any action that clears or modifies the entire index (like blanket `git reset`)
  unless that is exactly what was requested.
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

### Git and GitHub CLI (gh) Usage

1. **GitHub CLI (`gh`)**: Works out of the box (uses session tokens).
2. **Git Read-only (`fetch`, `pull`)**: Works for public repositories.
3. **Git Push**: Requires a terminal (TTY) for credential prompts. In non-interactive
   agent environments (where prompts are disabled), Git may fail even if credentials
   are stored. To bypass this, **explicitly force** a credential helper to avoid
   the TTY check:

```bash
# Option A: Use GitHub CLI (recommended if gh is authenticated)
git -c credential.helper='!gh auth git-credential' push origin my-branch

# Option B: Use the local store (if ~/.git-credentials is set up)
git -c credential.helper=store push origin my-branch
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
task. Follow the **Standard Workflow** (Alignment -> Plan -> Execute) when using this system.
All tasks are stored in the `./task/` directory as individual Markdown files.

### Folder Structure

- `task/` - Active tasks (currently being worked on).
- `task/pending/` - Future tasks (not yet started).
- `task/done/` - Completed tasks.
- `task/archive/` - Abandoned tasks kept for historical reference.

See `task/AGENTS.md` for detailed rules on managing individual task files.

---

## Skill Details

For detailed information on any skill, see `.agent/skills/<skill-name>/SKILL.md`. Each skill
includes:

- **SKILL.md** - Main instructions and workflow
- **Supporting files** - Detailed examples, patterns, references, and decision trees

The skills contain all the detailed guidance that was previously in this file, now organized for
autonomous discovery and reuse.
