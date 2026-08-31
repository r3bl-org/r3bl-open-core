# Agent Instructions for r3bl-open-core

## AI Agent Security & System Integrity Mandates

To prevent catastrophic system failures, all AI agents (Gemini, Claude, etc.) MUST adhere
to these strict guardrails. These mandates take absolute precedence over any "YOLO" mode
or perceived "fixes."

### 1. Critical Directory Protection

Recursive operations (`chown -R`, `chmod -R`, `rm -rf`) are STRICTLY PROHIBITED on the
following top-level system directories and their contents:

- `/` (Root)
- `/usr` (System binaries and libraries)
- `/etc` (System configuration)
- `/bin`, `/sbin`, `/lib`, `/lib64` (Essential system paths)
- `/boot` (Bootloader and kernels)
- `/var` (Variable data, including system logs and databases)

### 2. Ownership & Integrity

- **Root Ownership:** System directories and binaries MUST remain owned by `root`. The
  agent must NEVER suggest or execute a change of ownership for system-managed paths to a
  non-root user.
- **Privilege Escalation:** Do not modify the `setuid` or `setgid` bits of any system
  binary (e.g., `sudo`, `pkexec`, `mount`) unless specifically instructed by the user to
  fix a verified corruption.

### 3. Execution Safety

- **Explicit Paths Only:** All `sudo` commands involving recursive changes or deletions
  MUST use absolute paths. The use of wildcards (`*`) or relative paths (`.`) with
  `sudo chown/chmod/rm` is forbidden.
- **Verification First:** Before suggesting a permissions fix, the agent must first verify
  the current state using `ls -ld` or `stat`.
- **Destructive Warning:** Any command that modifies system-wide permissions or ownership
  must be explicitly flagged to the user with a explanation of the risks, even in YOLO
  mode.

---

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes: slow, steady, and careful work beats fast and careless.

## Standard Workflow (Alignment -> Plan -> Execute)

To ensure safety and alignment, always start by clarifying the scope of work. Ask the
user: "Are we starting:

1. a **new task**,
2. continuing an **existing task**, or
3. doing **one-off work**? (Please respond with 1, 2, or 3)"

### 1. New Task (Plan -> Task File -> Execute)

Follow this "slow and steady" workflow for all non-trivial changes:

- **In-Chat Planning:** Research the problem and present a comprehensive plan in chat for
  refinement. Use code examples and specifics.
- **Task File Creation:** Once approved, formalize it via `/r3bl-task create <name>`.
- **Manual Review:** Wait for the user to manually review and **explicitly approve** the
  task file before starting implementation.
- **Iterative Implementation:** Implement step-by-step, using `/r3bl-task update <name>`.

### 2. Existing Task

- **Load Task:** Identify the active task in `task/` and use `/r3bl-task load <name>`.
- **Resume:** Resume work from the next unchecked step after confirming with the user.

### 3. One-off Work

- For simple, isolated changes that do not require formal planning or task tracking,
  proceed directly to research and implementation.

## Progress & Review Guardrails (Anti-Hallucination)

To prevent large-scale destructive errors, "hallucinations," or accidental deletions
during complex or long-running tasks, you MUST follow these loop-in-the-user rules:

1. **Frequent Review Points:** Do not perform more than 3-5 consecutive file modifications
   without pausing to summarize progress and request user verification.
2. **Milestone Stability:** Stop and ask for a review as soon as you achieve any stable
   milestone (e.g., code compiles after a refactor, a sub-module is renamed, or a complex
   regex operation is completed).
3. **Validation before Review:** Always run `./check.fish --check` or `cargo check`
   locally BEFORE asking the user for a review. Never present "broken" progress.
4. **Attention Signal:** When stopping for a mandatory review point, run `fish -c "beep"`
   to alert the user.
5. **Mandatory Manual Review:** A task, phase, or sub-phase is not complete until the user
   has performed a manual review. This is the final step in the verification lifecycle. Do
   not mark a task as done in the task file until this review is successfully completed.
    - **Automatic Requirement:** You MUST automatically add a "Mandatory manual review"
      step with a checkbox list of all modified files to the end of every task, phase, and
      sub-phase you create or update.
    - **Review Workflow:** When the user prompts for a manual review at the end of a
      task/phase/sub-phase:
        - **Option A (Interactive Chat Review):** If the user triggers `/code-review` or
          asks for an interactive chunk-by-chunk review, follow the `code-review` skill to
          audit test coverage (`check-test-coverage`), present diffs one chunk at a time,
          and wait for approval.
        - **Option B (IDE Review):**
            1. Ask the user: "choose your ide: 1: code, 2: antigravity-ide, 3: codium, 4:
               code-insiders, 5: codium-insiders, if you press enter we will default to 1".
               (Note: if the user types "agy-ide" or similar, map it to "antigravity-ide").
            2. Then use `<IDE> <file_path>` to open the first file with a checkbox.
            3. Ask the user to manually review it.
            4. Once the user confirms ("good" or similar), check the box in the task file.
            5. Move to the next file and repeat until all checkboxes are checked.
6. **Strict Documentation Preservation:** Documentation is as critical as code. Any
   surgical edit that touches doc comments must be byte-perfect in its preservation of
   surrounding text.
7. **Post-Edit Rustdoc Verification:** After any file modification, especially when using
   `write_file`, you MUST verify that you have NOT clobbered pre-existing and valid
   rustdoc comments, diagrams, or module-level documentation. This is a high-priority
   check to maintain documentation integrity.
8. **Human-in-the-Loop:** When in doubt, or when a task involves global renames, stop and
   confirm the plan for the NEXT 3 files before touching them.
9. **No `git checkout` for mistakes:** If a mistake is made in a file during a refactor or
   edit, do NOT run `git checkout` to revert it. Instead, systematically fix the mistake
   using the built-in native file-editing tools accurately, deliberately, and slowly.
10. **Git Diff Audit Before Review:** Before asking the user for review or declaring a file
    edit complete, you MUST run `git diff <file_path>` (or `git diff`) on all modified
    files. Audit the diff line-by-line to verify that changes are strictly surgical and
    contain zero unintended collateral modifications (such as altered doctests, lost doc
    comments, or modified function signatures).

## Design Philosophy

Prioritize low cognitive load, progressive disclosure, and type-safe design. Make illegal
states unrepresentable. See `design-philosophy` skill for principles and patterns.

## Tooling & Capabilities

**A. Semantic Rust Tools (AST-Aware MCP):** Priority #1. When connected to the Rust MCP
server named `rust-analyzer` (running the `rust-analyzer-mcp-server` binary), aggressively use its
tools for code navigation (`rust_analyzer_definition`, `rust_analyzer_references`,
`rust_analyzer_hover`, `rust_analyzer_symbols`), quick-fixes
(`rust_analyzer_code_actions`), and diagnostics (`rust_analyzer_diagnostics`). These
AST-level operations are the safest way to inspect and modify code.

**B. Native File Replacements:** Priority #2. For structural changes that fall outside the
MCP's capabilities, use native file-editing tools like `multi_replace_file_content`.
Combine these with semantic tools (like `find_references`) to ensure you are modifying the
correct call sites. Do NOT write Python scripts.

**C. Bulk Code Modifications:**

1. **Scripts of ANY Kind are STRICTLY PROHIBITED**: You are forbidden from using scripts
   of any kind (whether shell-based like `perl`, `sed`, `awk`, `python`, `bash`, `fish`, or
   custom/disposable Rust scripts compiled via `rustc`) to perform file content modifications
   or bulk find-and-replaces.
    - **NEVER use `sed`** for blind type/variable renaming (it causes massive collateral
      damage to unstaged code and unrelated logic).
    - **NEVER use `awk`** (e.g., `awk '!seen[$0]++'`) for "cleaning up" files or
      deduplicating imports; it will destroy the file by removing all structural duplicate
      lines like `}` or empty lines.
    - **NEVER use disposable Rust or Python scripts** for bulk refactoring; subtle grammar
      and lexical edge cases (comments, doc tests, strings, shadowing) inevitably cause bugs.
2. **Native Tooling**: All code refactoring and modifications MUST be performed file-by-file
   using native file-editing tools (`replace_file_content` and `multi_replace_file_content`)
   combined with AST-level semantic tools (`rust-analyzer`).
3. **Optional Disposable Staging**: For extensive or multi-file changes, it is permissible
   to create a disposable copy (e.g., via BTRFS reflink `cp --reflink=auto`) where manual
   changes are made and everything is checked for correctness (`./check.fish --check`,
   `./check.fish --test`, `./check.fish --clippy`, `./check.fish --quick-doc`) before the
   changes are finalized in the real codebase.

**D. Safe Numeric Casting (No `as`):** Never use raw primitive `as` casts (e.g.,
`value as u8`). Instead, use the type-safe casting traits defined in
`tui/src/core/common/primitive_casting.rs` (e.g., `WideningCastTo...`,
`NarrowingCastTo...`). If a raw `as` cast is absolutely unavoidable (e.g., in `const`
contexts or dealing with FFI), you MUST annotate it perfectly with both a comment and an
allow attribute:

```rust
// XMARK: Intentional numeric casting using as.
#[allow(clippy::as_conversions)]
```

**E. Local Workflows (.agents/):** For repo-specific workflows (clippy, formatting, log
analysis), capabilities are defined in the `.agents/` directory. When a task matches a
skill, agent, or command:

1. Look inside the `.agents/` directory.
2. Read the markdown instructions inside that folder.
3. Execute the underlying shell/scripts exactly as instructed.

## Context Guardrail

You do not have the full codebase in memory. Actively use search and file-reading tools to
gather local context. If a request requires system-wide knowledge, global refactoring, or
sweeping architectural changes, **DO NOT GUESS**. Stop and ask the user to provide broader
context.

## Research Efficiency

- **Batch tool calls:** Execute research and file-reading tools in parallel to build
  context rapidly.
- **Deep investigation:** When mapping unfamiliar layers, proactively use multiple search
  and read calls in a single turn.
- **Autonomous progress:** In autonomous mode, do not stop for minor clarifications.
  Complete research and propose a high-signal plan in chat before pausing. Always follow
  the **Standard Workflow** and do not skip the alignment or approval steps.
- **Milestone delivery:** Aim for one high-signal turn (e.g., a complete research summary
  or initial chat plan) rather than many low-signal turns.

## Skills, Agents & Commands Location

All skills, agents, and slash commands are in the `.agents/` directory (not `.claude/`).
When loading a skill, agent, or command, look in `.agents/skills/`, `.agents/agents/`, and
`.agents/commands/` respectively.

## Crate-Specific Instructions

Some crates have additional instructions in their own `AGENTS.md` files:

- **build-infra/**: Provides CLI tools (binaries). **After making code changes, you MUST
  run `cargo install --path build-infra --force`** to update the installed binaries in
  `~/.cargo/bin`. See `build-infra/AGENTS.md` for details.

- **rust-analyzer-mcp-server/**: Provides the `rust-analyzer-mcp-server` MCP binary. **After making code changes, you MUST
  run `cargo install --path rust-analyzer-mcp-server --force`** to update the installed binary in
  `~/.cargo/bin`. See `rust-analyzer-mcp-server/AGENTS.md` for details.

- **tui/**: Main crate (`r3bl_tui`). For test directory taxonomy, PTY integration test
  conventions, and subprocess isolation patterns, use the `organize-tests` skill.

When working on a specific crate, always check for a local `AGENTS.md` file in that
crate's directory for additional workflow requirements.

## Available Skills

This project uses skills to organize coding patterns and workflows. All skills are in
`.agents/skills/`. When loading a skill, also check for and read any supporting `.md`
files in that skill's directory (e.g., `patterns.md`, `reference.md`, `examples.md`).

### Design

- **design-philosophy** - Core principles: cognitive load, progressive disclosure, type
  safety, abstraction worth. Use when designing APIs, modules, or data structures.
    - Supporting file: `patterns.md` (good/bad examples and quick reference)

### Code Quality & Style

- **check-code-quality** - Comprehensive quality checklist (check -> build -> docs ->
  clippy -> tests). Use after completing code changes and before creating commits.
    - Supporting file: `reference.md` (detailed cargo command reference)

- **check-test-coverage** - Audit and verify branch-targeted test coverage for a specific file or module, ensuring all custom logic paths and error branches are covered while strictly eliminating dependency test bloat.
    - Supporting file: `examples.md` (audit patterns and good vs bad coverage examples)

- **run-clippy** - Clippy linting, comment punctuation, cargo fmt. Use after code changes
  and before creating commits.
    - Supporting file: `patterns.md` (code style patterns and examples)

- **remove-crate-prefix** - Clean up generated code by removing unnecessary `crate::<T>`
  prefixes and instead importing `use crate::<T>` at the top of the file. Use this to
  comply with the "Clean Imports over Inline Absolute Paths" mandatory rule.

- **code-review** - Interactive chunk-by-chunk in-chat code review with explicit approval
  steps. Use when the user requests an interactive code review or runs `/code-review`.

### Documentation

- **write-documentation** - Consolidated documentation skill covering structure (inverted
  pyramid), intra-doc links, constant conventions, and formatting. Use proactively when
  writing code with rustdoc comments, or retroactively via `/fix-intradoc-links`,
  `/fix-comments`, `/fix-md-tables`.
    - Supporting files: `link-patterns.md`, `constant-conventions.md`, `examples.md`,
      `rustdoc-formatting.md`

### Architecture & Patterns

- **organize-modules** - Private modules with public re-exports (barrel export pattern),
  conditional visibility for docs/tests. Use when creating or organizing modules.
    - Supporting file: `examples.md` (6 complete module organization examples)

- **organize-tests** - Test directory taxonomy (why a test is isolated), PTY conventions
  (Run with section, deadlock prevention), zero test-bloat directive (test our code, not
  dependencies), and isolated process orchestration. Use when adding or refactoring tests.
    - Supporting files: `taxonomy.md` (directory guide), `pty-conventions.md` (PTY rules),
      `examples.md` (macro templates)

- **check-bounds-safety** - Type-safe Index/Length patterns for arrays, cursors,
  viewports, and terminal cursor movement. Includes `TermRowDelta`/`TermColDelta` for safe
  relative cursor movements that prevent CSI zero bugs. Use when working with
  bounds-sensitive code.
    - Supporting file: `decision-trees.md` (visual decision trees and flowcharts)

- **concurrency-safety** - Thread safety, Chain of Custody, Loud Lock Releases, and
  AtomicU8Ext patterns. Use when working with threads, locks, or atomics.
    - Supporting file: `patterns.md` (good/bad examples of lock management)

- **fast-string-allocations** - Zero-allocation string building strategies. Use when
  formatting strings, generating ANSI codes, or writing hot loops to avoid heap
  allocations and Formatter overhead.

### Performance

- **analyze-performance** - Flamegraph-based performance regression detection. Use when
  optimizing or investigating performance.
    - Supporting file: `baseline-management.md` (when and how to update baselines)

### Release

- **release-crate** - Full crate release workflow: version bump, changelog, publish to
  crates.io, git tag, GitHub release. Use when releasing a new version of any workspace
  crate.

- **review-pr** - Create a structured integration and review plan for a Pull Request. Use
  when the user wants to systematically integrate a community PR.

- **create-pr** - Push local changes and create a GitHub Pull Request. Use when you have
  local changes that need a PR but didn't start with `/fix-issue`.

- **merge-pr** - Workflow for pushing a completed task branch, creating a Pull Request,
  and merging it to main via rebase.

### Log Analysis

- **analyze-log-files** - Strip ANSI escape sequences from log files before analysis. Use
  when asked to process, read, or analyze log files that may contain terminal escape
  codes.

## Available Agents (`.agents/agents/`)

| Agent              | Purpose                                     |
| :----------------- | :------------------------------------------ |
| **test-runner**    | Expert in running tests and fixing failures |
| **clippy-runner**  | Expert in linting and fixing style issues   |
| **code-formatter** | Expert in bulk code formatting              |
| **perf-checker**   | Expert in performance regression analysis   |

## Slash Commands

**Rule:** When adding a new skill to `.agents/skills/`, you MUST add a corresponding slash
command entry for that new skill in the table below. This ensures the command is available
via autocomplete in the Antigravity CLI.

| Command                     | Skill                                      |
| :-------------------------- | :----------------------------------------- |
| `/analyze-logs`             | analyze-log-files                          |
| `/check-regression`         | analyze-performance                        |
| `/batch-refactor`           | batch-refactor-with-sub-agents             |
| `/check-bounds-safety`      | check-bounds-safety                        |
| `/check`                    | check-code-quality                         |
| `/check-test-coverage`      | check-test-coverage                        |
| `/code-review`              | code-review                                |
| `/concurrency-safety`       | concurrency-safety                         |
| `/create-commit-message`    | create-commit-message                      |
| `/create-pr`                | create-pr                                  |
| `/design-philosophy`        | design-philosophy                          |
| `/fast-string-allocations`  | fast-string-allocations                    |
| `/fix-issue`                | fix-issue                                  |
| `/merge-pr`                 | merge-pr                                   |
| `/organize-modules`         | organize-modules                           |
| `/organize-tests`           | organize-tests                             |
| `/release`                  | release-crate                              |
| `/remove-crate-prefix`      | remove-crate-prefix                        |
| `/review-pr`                | review-pr                                  |
| `/clippy`                   | run-clippy                                 |
| `/docs`                     | write-documentation                        |
| `/fix-intradoc-links`       | write-documentation (focused on links)     |
| `/fix-comments`             | write-documentation (constant conventions) |
| `/fix-md-tables`            | write-documentation (table formatting)     |
| `/write-structured-tracing` | write-structured-tracing                   |
| `/r3bl-task`                | Task management (see below)                |
| `/boxes`                    | Unicode box-drawing character set          |

## Running Checks

**Always use `check.fish`** instead of running cargo commands directly. `check.fish`
provides ICE recovery, stale artifact cleanup, config change detection, toolchain
validation, and tmpfs/ionice optimizations, all of which are lost with direct cargo calls.

| Command                    | What it runs                                                               |
| :------------------------- | :------------------------------------------------------------------------- |
| `./check.fish --check`     | `cargo check` (fast typecheck)                                             |
| `./check.fish --build`     | `cargo build` (compile production)                                         |
| `./check.fish --clippy`    | `cargo clippy --all-targets` (linting)                                     |
| `./check.fish --fmt`       | `cargo fmt` + `cargo rustdoc-fmt` on git-changed files                     |
| `./check.fish --test`      | `cargo test` + doctests                                                    |
| `./check.fish --doc`       | `cargo doc --workspace` (full, with dep-doc caching)                       |
| `./check.fish --quick-doc` | `cargo doc --workspace --no-deps` (fastest, no staging/sync)               |
| `./check.fish --full`      | All of the above + Windows cross-compilation check + lychee link rot check |

Commands with **no check.fish equivalent** (run directly):

- `cargo rustdoc-fmt`: format rustdoc comments
- `cargo clippy --all-targets --fix --allow-dirty`: auto-fix lints
- `cargo fmt --all`: format code

### Non-Blocking Verification via Subagents

Long-running verification commands (such as `./check.fish --test`, `./check.fish --doc`,
`./check.fish --quick-doc`, and `./check.fish --full`) can take 1 to 2+ minutes. To prevent
blocking the active conversation loop:

- **Fast Checks**: Lightweight commands like `./check.fish --check` and
  `./check.fish --fmt` take only a few seconds and may run directly.
- **Long-Running Checks**: Delegate time-consuming checks to a background subagent (e.g.,
  using `invoke_subagent` with `TypeName: "self"`).
- **Unblocked Collaboration**: While the subagent verifies build, test, or documentation
  status in the background, the primary agent remains interactive to discuss code,
  perform reviews, and plan next steps.

## Rust Code Guidelines

### Writing Documentation & Markdown

When writing or modifying rustdoc comments in code, task files, or standalone `.md` files,
**proactively apply** these conventions (all documented in `write-documentation` skill):

1. **Intra-doc links**: Prefer `crate::` paths (shorter). Use `super::` when `crate::`
   paths get too long and symbols are co-located. Reference-style links at bottom of doc
   blocks. See `link-patterns.md` for patterns.

2. **Human-readable constants**: Use binary for bitmasks (`0b0110_0000`), byte literals
   for printable chars (`b'['`), decimal for non-printables (`27`). Show hex in comments
   for cross-reference. See `constant-conventions.md`.

3. **Inverted pyramid**: High-level concepts at module/trait level, simple syntax examples
   at method level. See `examples.md`.

4. **Sidebar headings**: Only `#` and `##` headings appear in the rustdoc sidebar
   navigation. Use `**bold**` text instead of `###` for sub-sections within doc comments.

5. **No connecting dashes, en dashes, or em dashes (Global Rule)**: The ASCII **hyphen** /
   **hyphen-minus** (`-`, `U+002D`) is the ONLY dash character permitted anywhere in the
   codebase (including `.md` task/plan files, rustdoc, and chat responses). Non-ASCII en
   dashes (`–`) and em dashes (`—`, endash/emdash) are strictly forbidden. Furthermore, do
   NOT use hyphens (`-`) to connect clauses or sentences in documentation: write separate
   sentences or use colons, semicolons, commas, or parentheses instead.

6. **No `$` or LaTeX Math Delimiters (Global Rule)**: Do NOT use `$` or `$$` or `\(...\)`
   LaTeX math delimiters in doc comments, task `.md` files, or chat responses. They do not
   render correctly in standard markdown viewers. Use standard Markdown text, backticks
   (e.g., `[start, start+len)`), or code blocks instead.

7. **Cross-Platform Doctests for Platform-Specific APIs**: When writing doctests that
   reference platform-specific types (such as Linux-only `DirectToAnsiInputDevice` or
   `MioPollWorker`), do NOT use `ignore` and do NOT downgrade them to noisy generic mocks.
   Instead, use the `# #[cfg(not(target_os = "linux"))] # fn main() {}` and
   `# #[cfg(target_os = "linux")] # mod linux_only { ... }` pattern documented in the
   `write-documentation` skill. This keeps rendered docs clean, validates real types on
   Linux, and compiles safely on macOS and Windows.

Don't wait for `check-code-quality` to catch issues - write docs correctly the first time.

### Constructor Conventions: `Default` over No-Arg `new()`

When designing Rust types in this codebase:

1. **No-Argument Constructors**: If a type requires no arguments for default construction, derive or implement `Default` (`#[derive(Default)]` or `impl Default for MyType`) and do **NOT** create a redundant `pub fn new() -> Self` method. Callers instantiate it via `MyType::default()` or `Default::default()`.
2. **Parameterized Constructors**: If a constructor requires arguments (e.g., `with_capacity(capacity: usize)` or `new(arg: Type)`), use explicit constructor methods with parameters.

### Mutator Return Conventions: Avoid Boolean Blindness (`Option<T>` / `Option<()>`)

When designing mutator methods taking `&mut self` that can fail due to invalid indices or out-of-bounds conditions:

1. **Why `&mut self` Mutators Must Heap-Allocate Text Payloads**:
   In Rust, a `&mut self` mutator that deletes or replaces text cannot return `Option<&str>` or zero-allocation byte offsets (`DocSeg`), because:
   - The borrow checker forbids returning a `&str` reference borrowing from `self` across a `&mut self` mutation.
   - Once deleted from `self`, the text bytes no longer exist in memory, invalidating any borrowed slices or offset metadata.
   - Therefore, returning deleted/replaced text payload *unavoidably requires* an owned heap allocation (`Option<String>`).
2. **Returning Removed/Replaced Payloads (`Option<T>`)**:
   Return a payload `Option<T>` (e.g., `Option<LineMetadata>` for zero-allocation structural metrics, or owned `Option<String>` for text) ONLY when callers genuinely consume the deleted payload.
3. **Zero-Allocation Success/Failure (`Option<()>` over `bool`)**:
   If callers do not consume the deleted text payload, do NOT incur speculative heap allocations (`Option<String>`). Instead, return `Option<()>` (`Some(())` for success, `None` for failure/out-of-bounds). `Option<()>` provides the exact same zero-allocation CPU-register performance as a raw `bool` while eliminating boolean blindness and supporting the `?` try operator.

### Clean Imports over Inline Absolute Paths (Mandatory)

Do NOT write absolute inline paths like `crate::Type` or `crate::Size` inside function
signatures or bodies; instead, import them cleanly via `use` statements at the top of the
file, then reference the type directly. This keeps code highly readable and reduces
cognitive clutter. When writing generated code, make sure to use the `remove-crate-prefix`
skill to automate this clean up so generated code is not littered with `crate::<T>`.

**✅ Good:**

```rust
use crate::{Size, Pos};

pub fn render(size: Size) -> Pos { ... }
```

**❌ Bad:**

```rust
pub fn render(size: crate::Size) -> crate::Pos { ... }
```

### Macro Imports

Do NOT use `#[macro_use]` on module declarations. For `#[macro_export]` macros, use
explicit imports: `use crate::macro_name;`. Each `mod` block that uses a macro needs its
own import - parent scope imports don't propagate into child modules.

### Cross-Platform Verification

When working with platform-specific code (`#[cfg(unix)]`, `#[cfg(not(unix))]`), verify
Windows compatibility. This performs type checking and borrow checking for the Windows
target without full code generation or linking.

Note: While `--emit=metadata` skips the linking stage, the **mingw-w64 toolchain is still
required** because many core dependencies (like `windows-sys`, `parking_lot`, or
`mimalloc`) have build scripts that probe for `x86_64-w64-mingw32-gcc` and
`x86_64-w64-mingw32-dlltool`.

```bash
cargo rustc -p <crate_name> --target x86_64-pc-windows-gnu -- --emit=metadata
```

Use this after modifying `DirectToAnsi` input handling or other Unix-specific code.

### Testing Interactive Terminal Applications

For testing interactive terminal applications, use (both are installed):

- `tmux`
- `screen`

### Prefer `?` Operator over `.and_then()` (Mandatory)

Do NOT use `.and_then()` for chaining `Option` or `Result` operations in code. Instead,
write idiomatic Rust using early returns with the `?` operator (or `if let` / `let-else`
statements). The only acceptable exception is for boolean constant gating in
tracing/logging calls if necessary, though even then, canonical Rust is preferred.

**✅ Good:**

```rust
fn get_char(&self, pos: Pos) -> Option<PixelChar> {
    let row = self.get_row(pos.row_index)?;
    let cell = row.get(pos.col_index.as_usize())?;
    Some(*cell)
}
```

**❌ Bad:**

```rust
fn get_char(&self, pos: Pos) -> Option<PixelChar> {
    self.get_row(pos.row_index)
        .and_then(|row| row.get(pos.col_index.as_usize()))
        .copied()
}
```

### Prefer `FxHashMap` and `FxHashSet` over Standard `HashMap` and `HashSet` (Mandatory)

Do NOT use standard `std::collections::HashMap` or `std::collections::HashSet` in this
codebase. Standard library maps use `SipHash 1-3` by default to protect against network
Hash-DoS collision attacks, which is completely unnecessary for local TUI and CLI
applications.

Instead, always import and use `rustc_hash::FxHashMap` and `rustc_hash::FxHashSet`
(from the `rustc-hash = "2.1.0"` crate). `FxHasher` runs in ~1 CPU cycle using simple
arithmetic shifts and multiplications, drastically cutting hashing overhead (up to 87%
reduction in flamegraphs).

**✅ Good:**

```rust
use rustc_hash::FxHashMap;

let mut map = FxHashMap::default();
map.insert(key, value);
```

**❌ Bad:**

```rust
use std::collections::HashMap;

let mut map = HashMap::new();
map.insert(key, value);
```

## Git Workflow

### PR Lifecycle & Commands

We have a cohesive, interconnected lifecycle for Pull Requests codified in
`.agents/skills/`:

1. **Start a new task:** `/fix-issue`
    - Creates the branch, pushes it, and opens a Draft PR (`gh pr create --draft`) to
      track the work.
2. **Review community work:** `/review-pr`
    - Fetches an existing PR to systematically audit, test, and rewrite locally.
3. **Manual PR creation:** `/create-pr`
    - For when you have local changes on a branch and just want to push and open a PR
      (`gh pr create --fill`) without going through the full `/fix-issue` design process.
4. **Merge and complete:** `/merge-pr`
    - The endpoint for all of the above. Pushes the finalized local branch and linearly
      merges the existing PR (`gh pr merge --rebase`), then cleans up the local workspace.

### General Rules

- **No Destructive Resets**: NEVER use `git reset HEAD~n`, `git reset --hard`, or
  `git clean` unless explicitly and specifically commanded to do so by the user. These
  commands are destructive to the user's work and staging area.
- **No Clobbering Existing Edits**: NEVER use `git checkout` or `git restore` to overwrite
  existing files in the working tree unless explicitly and specifically commanded to do so
  by the user. This can destroy uncommitted work.
- **Surgical Unstaging**: When asked to unstage specific files, ALWAYS use targeted
  commands like `git restore --staged <file>`. NEVER unstage the entire index or use
  blanket reset commands if a specific list of files is provided.
- **Respect the Index**: The staging area is the user's carefully curated state. Do not
  perform any action that clears or modifies the entire index (like blanket `git reset`)
  unless that is exactly what was requested.
- Never use `git stash` / `git stash pop` to test against clean state - it destroys the
  staging area (index). Use the Task tool with `isolation: "worktree"` to run tests in a
  separate git worktree without touching the main working tree.
- Use `git mv` instead of `mv` when moving or renaming files to preserve move history in
  git.
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
   agent environments (where prompts are disabled), Git may fail even if credentials are
   stored. To bypass this, **explicitly force** a credential helper to avoid the TTY
   check:

```bash
# Option A: Use GitHub CLI (recommended if gh is authenticated)
git -c credential.helper='!gh auth git-credential' push origin my-branch

# Option B: Use the local store (if ~/.git-credentials is set up)
git -c credential.helper=store push origin my-branch
```

### Commit Message Format

When creating or formatting a commit message, you MUST invoke and follow the
`create-commit-message` skill. It contains all the detailed rules for formatting (72-char
limits, trailers, scope prefixes).

## Task Tracking System

The `/r3bl-task` slash command is available to manage all the details of a long-running
task. Follow the **Standard Workflow** (Alignment -> Plan -> Execute) when using this
system. All tasks are stored in the `./task/` directory as individual Markdown files.

### Task File Formatting

Always run `prettier --write <file>` on any `task/*.md` files after creating or updating
them. This ensures markdown is correctly formatted and easy to review in the user's IDE.

### Folder Structure

- `task/` - Active tasks (currently being worked on).
- `task/pending/` - Future tasks (not yet started).
- `task/done/` - Completed tasks.
- `task/archive/` - Abandoned tasks kept for historical reference.

See `task/AGENTS.md` for detailed rules on managing individual task files.

---

## Skill Details

For detailed information on any skill, see `.agents/skills/<skill-name>/SKILL.md`. Each
skill includes:

- **SKILL.md** - Main instructions and workflow
- **Supporting files** - Detailed examples, patterns, references, and decision trees

The skills contain all the detailed guidance that was previously in this file, now
organized for autonomous discovery and reuse.

---

## Large Refactoring Scenarios

When executing large-scale, tree-wide refactoring tasks (such as mass renaming variables,
fields, or types across many files), agents MUST activate the `safe-renaming` skill and
follow these safety protocols:

1. **Use Built-in Tools First:** Always attempt to use the built-in, precise file
   modification tools (`multi_replace_file_content` or `replace_file_content`) if the
   scope of the refactor is small enough to be handled safely file-by-file.
2. **BTRFS Staging Copy (`cp --reflink=auto`):** For tree-wide refactoring, create a BTRFS
   reflink clone of the workspace at `~/Downloads/rename-staging/` before modifying any
   files. Never use `tmpfs` as build artifacts can exhaust RAM.
3. **Custom Rust Script with `--dry-run`:** Create a disposable Rust (`rustc`) script with
   a mandatory `--dry-run` CLI argument and strict word boundary matching (`\b` or
   `!is_alphanumeric() && c != '_'`).
4. **Staging Execution & Verification:** Run `--dry-run` on `~/Downloads/rename-staging/`,
   apply the changes there, and run the full test suite (`cargo check`, `cargo clippy`,
   `cargo test`, `cargo doc`) inside the staging directory.
5. **Apply to Live Repo:** IF AND ONLY IF all tests pass 100% in
   `~/Downloads/rename-staging/`, remove the staging directory and run the refactoring
   binary on the live workspace (`~/github/roc`).
6. **Reference Skill:** See `.agents/skills/safe-renaming/SKILL.md` for full instructions.
