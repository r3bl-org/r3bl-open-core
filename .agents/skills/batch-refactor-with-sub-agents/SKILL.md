---
name: batch-refactor-with-sub-agents
description: Use a sub-agent (like `generalist`) to perform repetitive code transformations across multiple files in a single turn.
---

## When to Use
- Renaming symbols or updating function signatures across many files.
- Migrating code from one pattern to another (e.g., manual error handling to a common helper).
- Replacing literals with centralized constants across the codebase.
- Performing any repetitive "find-and-replace" task that would otherwise require many sequential `replace` calls.

## Procedure
1.  **Define the transformation**: Identify the exact "before" and "after" patterns. Create a representative code snippet for the sub-agent to follow.
2.  **Locate targets**: Use `grep_search` to find all absolute file paths and line numbers that need modification.
3.  **Draft a precise prompt**: Call the `generalist` (or similar) tool with a prompt that includes:
    - **Goal**: Clear statement of the refactoring objective.
    - **Scope**: A bulleted list of absolute file paths to modify.
    - **Example**: A code block showing the `old_string` vs `new_string` transformation.
    - **Constraints**: Instructions to preserve specific logic (e.g., "preserve original closure logic while changing the call site").
    - **Verification**: Instructions to run `cargo check --all-targets` or specific tests after finishing.
4.  **Delegate**: Execute the sub-agent call.
5.  **Review and Cleanup**: Sub-agents may introduce minor issues like `unused_import` warnings. Perform a final sweep with `cargo clippy` and fix manually or via a second batch call.

## Pitfalls and Fixes
- **symptom**: Sub-agent misses some files or applies incorrect logic.
- **likely cause**: Prompt was too vague or the transformation was too complex for a single turn.
- **fix**: Break the task into smaller, more homogeneous batches (e.g., "refactor all files in directory A first").
- **symptom**: `cargo check` fails after the sub-agent finishes.
- **likely cause**: Sub-agent clobbered a symbol or messed up indentation/syntax.
- **fix**: Read the modified files and use targeted `replace` calls to fix the syntax errors.

## Verification
- Run `cargo check --all-targets` to confirm zero compilation errors.
- Run `cargo clippy --all-targets` to find and fix any `unused_import` or style warnings introduced by the refactor.
- Run relevant integration tests for the modified files.
