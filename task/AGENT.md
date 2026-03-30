# Rules for managing tasks

Tasks are instructions to guide claude code to implement things. One task per .md file.

## Structure of a task file

```md
# Task: [Short Name]

## Overview

Why this task exists and what it accomplishes. Include architectural context only if
non-obvious from the code.

## Implementation plan

### Phase 1: [Phase name]

- [x] Completed step
- [ ] Pending step
- [ ] Another pending step

### Phase 2: [Phase name]

- [ ] Step with brief technical note if needed
- [ ] Another step
```

Keep it concise:
- Use `[x]`/`[ ]` checkboxes, not status codes on headings.
- Use `###` phase headers to group related steps. Only add phases when steps naturally cluster.
- Prefer one line per step. A brief technical note or sub-bullet is fine for clarity.
- Do NOT use doctoc, numbered step headings (`Step 0.0`), or per-step detail blocks.
- Avoid large code blocks. Use descriptive text for intent, but a brief snippet is
  encouraged if it's the highest-signal way to define the goal.
- Technical notes and context belong in the Overview, not repeated per step.

## Folder structure

- `task/` - Active tasks (currently being worked on).
- `task/pending/` - Future tasks (not yet started).
- `task/done/` - Completed tasks.
- `task/archive/` - Abandoned tasks kept for historical reference.
