---
name: batch-refactor-with-sub-agents
description: Use a sub-agent (like `generalist`) to perform repetitive code transformations across multiple files in a single turn.
---

## When to Use Sub-Agents
- Renaming symbols or updating function signatures across many files (but not the entire repository).
- Migrating code from one pattern to another (e.g., manual error handling to a common helper).
- Replacing literals with centralized constants across the codebase.
- Performing repetitive tasks that require *semantic understanding* and cannot be solved with a simple string replace.

## When to Use Custom Rust Scripts (Priority for Massive Refactors)
For massive, repository-wide bulk refactoring (typically **6+ files**, e.g., renaming a string or trait across 100+ files), you MUST NOT use `sed`, `awk`, `perl`, `python`, `bash`, or `fish` (which are strictly prohibited). Furthermore, spawning sub-agents for a simple find-and-replace across 6+ files is inefficient.
Instead:
1. Write a custom, disposable Rust script. Save this script in `tmpfs` (e.g., `/tmp/`).
2. Implement a `--dry-run` CLI argument to test the script before making destructive changes.
3. Use `std::fs` to read files, apply string replacements (`replace()`), and write back (only if not a dry-run).
4. Compile it natively with `rustc` and execute it.
5. Run `cargo check` to verify.

## Procedure for Sub-Agents
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
