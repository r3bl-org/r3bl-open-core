# Claude Code Instructions for r3bl-open-core

Ask for clarification immediately on important choices or ambiguities. Take your time with
changes—slow, steady, and careful work beats fast and careless.

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

### Code Quality & Style

- **check-code-quality** - Comprehensive quality checklist (check → build → docs → clippy → tests). Use after completing code changes and before creating commits.
  - Supporting file: `reference.md` (detailed cargo command reference)

- **run-clippy** - Clippy linting, comment punctuation, cargo fmt. Use after code changes and before creating commits.
  - Supporting file: `patterns.md` (code style patterns and examples)

### Documentation

- **write-documentation** - Rustdoc formatting with inverted pyramid principle and cargo rustdoc-fmt. Use when writing or improving documentation.
  - Supporting files: `rustdoc-formatting.md` (cargo rustdoc-fmt guide), `examples.md` (5 production-quality examples)

- **fix-intradoc-links** - Fix and create rustdoc intra-doc links for IDE navigation. Use when cargo doc shows link warnings.
  - Supporting file: `patterns.md` (14 detailed link patterns)

### Architecture & Patterns

- **organize-modules** - Private modules with public re-exports, conditional visibility for docs/tests. Use when creating or organizing modules.
  - Supporting file: `examples.md` (6 complete module organization examples)

- **check-bounds-safety** - Type-safe Index/Length patterns for arrays, cursors, viewports. Use when working with bounds-sensitive code.
  - Supporting file: `decision-trees.md` (visual decision trees and flowcharts)

### Performance

- **analyze-performance** - Flamegraph-based performance regression detection. Use when optimizing or investigating performance.
  - Supporting file: `baseline-management.md` (when and how to update baselines)

## Slash Commands for Skills

You can explicitly invoke skills using slash commands:

- `/check` → check-code-quality
- `/docs` → write-documentation
- `/clippy` → run-clippy
- `/fix-intradoc-links` → fix-intradoc-links
- `/check-regression` → analyze-performance
- `/r3bl-task` → Task management (see below)

## Rust Code Guidelines

### MCP Tools

Use these MCP tools to navigate and modify Rust code effectively:

- **serena**: definition, diagnostics, edit_file, hover, references, rename symbol, etc.

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
