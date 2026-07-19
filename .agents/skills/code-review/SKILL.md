---
name: code-review
description: Interactively review code changes chunk by chunk with explicit user approval
---

# Interactive Code Review Workflow

Use this skill when the user runs the `/code-review` slash command or uses any of these
natural language triggers:

- "show me the code changes you made here, one chunk at a time interactively. i will type 'good' if i approve"
- "review the code changes chunk by chunk"
- "interactive code review"
- "review changes interactively"

## Purpose

Provides a structured, low-cognitive-load, interactive code review experience in chat.
Instead of dumping large diffs or requiring the user to navigate full files in an external
editor, changes are broken into small, logical, self-contained diff chunks presented one
turn at a time.

## Workflow

### 1. Collect & Partition Diff Chunks

1. Run `git diff` on the current working tree (or inspect specific target files).
2. Partition the changes into small, logical, self-contained chunks:
   - Separate by file whenever possible.
   - For larger files, divide into logical functional blocks (e.g., imports, core logic,
     tests).
3. Determine total chunk count (N).

### 2. Present Chunks Iteratively (One Turn per Chunk)

For each chunk (e.g., Chunk X of N):

1. **Heading & File Link**:
   `### Chunk X of N: path/to/file.rs` (use clickable markdown file links).
2. **Context & Rationale**:
   Provide a concise 1-2 sentence explanation of what changed in this chunk and why.
3. **Diff Snippet**:
   Provide a clean `diff` code block containing only the relevant hunk.
4. **Approval Prompt**:
   Ask: `Please reply with **"good"** to approve and move to Chunk X+1.`
5. **STOP and WAIT**:
   Do NOT output subsequent chunks in the same turn. Yield control and wait for user input.

### 3. Handle User Feedback

- **Approved ("good", "ok", "lgtm")**:
  Advance to the next chunk and present it.
- **Requested Changes / Corrections**:
  1. Make surgical edits to the code using native file editing tools.
  2. Run `./check.fish --check` (or relevant test/clippy checks) to verify correctness.
  3. Re-present the revised chunk to the user with the updated diff.
  4. Wait for approval before proceeding.

### 4. Completion & Checklist Update

Once all N chunks have been explicitly approved:

1. If working on a task file (e.g., `task/<name>.md`), check off the corresponding files
   in the "Mandatory manual review" checklist.
2. Run `./check.fish --fmt` on changed files.
3. Notify the user that all chunks have been reviewed and approved, and ask how to proceed
   with next steps.
