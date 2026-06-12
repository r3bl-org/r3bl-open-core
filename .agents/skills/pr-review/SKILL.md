---
name: pr-review
description: Create a structured integration and review plan for a Pull Request
---

# PR Review and Integration Workflow

Use this skill when the user runs the `/pr-review <number>` slash command or uses any of these natural language triggers:
- "lets review pr <number>"
- "lets work on pr <number>"
- "lets take a look at pr <number>"

## Workflow

When triggered, you MUST follow these exact steps:

1. **Information Gathering**
   - Run `gh pr view <number>` to read the PR description.
   - Run `gh pr diff <number>` to see the exact code changes and files touched.
   
2. **Task File Generation**
   - Create a new markdown task file at `task/pr-<number>-fix.md`.
   - The file MUST follow the template structure below, organizing the PR into distinct, actionable chunks (phases/headings).
   
3. **Format & Present**
   - Automatically run `npx prettier --write task/pr-<number>-fix.md` to format the new file.
   - Run `antigravity-ide task/pr-<number>-fix.md` to open the file for the user.
   - Ask the user to **manually review and explicitly approve** the plan before any implementation begins.

## Task File Template

When creating `task/pr-<number>-fix.md`, structure it exactly like this:

```md
# Task: PR <number> Integration & Fixes

## Overview

[Brief summary of the PR, author, and what problem it solves.]

## Execution Workflow

We will process each of the action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and, where applicable, test functionality.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically touched files before the heading is marked as checked `[x]`.

_(Once all headings are successfully implemented and checked off, we will proceed to final verification and cleanup.)_

### Core Fixes from PR #<number>

#### [ ] [Name of first distinct fix/feature]

[Brief context of the problem and the specific fix.]

- _Context:_ [Why this change is needed.]
- _The Fix:_ [What the code actually does.]
- _File(s) Touched:_ [List the files.]

#### [ ] [Name of next fix/feature...]
...

### Final Verification & Cleanup

- [ ] Verify full test suite coverage using `./check.fish --full`.
- [ ] Hijack the PR branch to apply our rewritten code while preserving their authorship credit:
  - `git checkout main` and identify our rewrite commit (`FIX_COMMIT`) and the commit before it (`MAIN_COMMIT`).
  - `git reset --hard <MAIN_COMMIT>` and `git push --force origin main` to cleanly separate our fix from main.
  - `gh pr checkout <number>` to pull down their branch.
  - `AUTHOR=$(git log -1 --format="%an <%ae>")` to extract their authorship.
  - `git reset --hard <FIX_COMMIT>` to wipe their changes and plonk our code into their branch.
  - `git commit --amend --author="$AUTHOR" --no-edit` to give them credit for our rewrite.
  - `git push --force` to update the PR on GitHub.
- [ ] Merge the PR into `main` (`gh pr merge <number>`).
- [ ] Update the current meta-task (e.g. `task/prepare-vX.Y.Z-meta-task.md`) to check off PR #<number>.
- [ ] **Mandatory manual review:** Verify every file modified in this task for correct implementation and ensure no regressions.
  - [ ] `path/to/modified_file_1.rs`
  - [ ] `path/to/modified_file_2.rs`
  - [ ] `task/prepare-vX.Y.Z-meta-task.md`
```

## Critical Rules
- Do NOT merge or apply the PR automatically. The purpose of this workflow is to rewrite or cherry-pick the fixes systematically while validating them against the current `main` branch.
- Break down the PR diff into logical, self-contained action items rather than just one giant block.
