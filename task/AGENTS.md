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
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `path/to/modified_file_1.rs`
    - [ ] `path/to/modified_file_2.rs`
```

Keep it concise:

- Use `[x]`/`[ ]` checkboxes, not status codes on headings.
- Use `###` phase headers to group related steps. Only add phases when steps naturally
  cluster.
- **Short, Meaningful Headings**: Keep headings concise (typically 2-5 words, e.g.,
  `### Phase 1: RRT Broadcast Channel` instead of
  `### Phase 1: Resilient Reactor Thread (RRT) Synchronous Channel Layer (std::sync::mpsc + Waker)`).
  This keeps both the document and the `mktoc` table of contents easy to scan. Move any
  extra detail or type lists into the first sentence immediately following the heading.
- **Mandatory Manual Review**: Every task, phase, and sub-phase MUST end with a "Mandatory
  manual review" step containing a checkbox for each modified file. **CRITICAL:** These
  checkboxes MUST be left as `[ ]` by the agent. Only the user may check them after their
  manual review is complete.
- Prefer one line per step. A brief technical note or sub-bullet is fine for clarity.
- Do NOT use `doctoc` (use `mktoc` with `<!-- BEGIN mktoc -->` / `<!-- END mktoc -->` if a
  TOC is needed), numbered step headings (`Step 0.0`), or per-step detail blocks.
- **Exact Signatures & File Paths for Implementation**: Include exact Rust type
  signatures, trait bounds, and target file paths in the plan so implementation steps can
  be executed deterministically without guessing architecture. Avoid large boilerplate
  implementation blocks.
- Technical notes and context belong in the Overview, not repeated per step.
- **Formatting**: If a markdown file contains `<!-- BEGIN mktoc -->` /
  `<!-- END mktoc -->` markers, run `mktoc <file>`. Always run `prettier --write <file>`
  on any markdown files after creating or editing them to ensure they are properly
  formatted and easy to review in the user's IDE.

## Folder structure

- `task/` - Active tasks (currently being worked on).
- `task/pending/` - Future tasks (not yet started).
- `task/done/` - Completed tasks.
- `task/archive/` - Abandoned tasks kept for historical reference.
