---
name: audit-task
description: Launch a subagent to critically and exhaustively audit a task plan markdown file. Looks for bugs, inconsistencies, missing features, and inaccurate, wrong, or dangerous steps.
---

# Audit Task

This skill is invoked when the user runs the `/audit-task <file>` slash command.

## Procedure

1. Read the provided `<file>` (which is typically a Markdown file in the `task/` folder).
2. Launch a subagent using the `invoke_subagent` tool. You MUST set the `Model` argument to `pro` (which maps to gemini-3.1-pro) and explicitly instruct it to use high reasoning effort. You can use an existing subagent like `research`, or define a new one for this specific purpose.
3. The prompt for the subagent MUST instruct it to:
   - Read the plan in the provided `<file>`.
   - Audit the plan critically and exhaustively.
   - Look for bugs, inconsistencies, and missing features.
   - Identify any inaccurate, wrong, or dangerous steps/instructions.
   - Report its findings clearly and systematically.
4. Wait for the subagent to complete its analysis.
5. Present the subagent's findings to the user in a clear, structured report.
