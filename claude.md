# Claude Code Instructions for r3bl-open-core

## Task Tracking System

This project uses a two-file task tracking system to maintain project state and history:

### 1. todo.md - Active Work File

- **Check this file at the start of each session** to understand current project state
- Update task checkboxes `[x]` immediately when completing tasks
- Keep partially completed sections here (with mixed `[x]` and `[ ]` items)
- Add newly discovered tasks to appropriate sections
- Maintain task hierarchy with proper indentation for subtasks

### 2. done.md - Archive File

- Contains completed feature sets and milestones
- Move **entire sections** here only when ALL subtasks are complete
- Include the section header and all its subtasks when moving
- This serves as the project's historical record

### When to Move Tasks from todo.md to done.md

- Only move complete sections where ALL subtasks show `[x]`
- Never move individual tasks - preserve context by keeping related tasks together
- Example: The "fix md parser" section should only move after all 200+ subtasks are complete

### Task Format Guidelines

- Use `- [x]` for completed tasks
- Use `- [ ]` for pending tasks
- Group related tasks under descriptive headers with `#` or plain text
- Include GitHub issue links where relevant (e.g.,
  `https://github.com/r3bl-org/r3bl-open-core/issues/397`)
- Add technical notes, code snippets, or implementation details for complex tasks
- Use consistent indentation (2 spaces) for subtasks

### Benefits of This System

- todo.md remains focused and manageable (<300 lines)
- Historical progress is preserved in done.md
- Active work stays visible with full context
- Easy to track what's been accomplished vs what remains

## Additional Project Guidelines

### Code Quality

- Run typecheck, test, and lint commands after completing tasks:
  - Fast compiler typecheck command is `cargo check`
  - Detailed lint check is `cargo clippy`
  - Test check is `cargo nextest run`
  - Ask user for the correct commands if unable to find them
  - Suggest adding these commands to this file for future reference

### Git Workflow

- Never commit changes unless explicitly asked by the user
- This prevents overly proactive behavior that might disrupt the user's workflow
