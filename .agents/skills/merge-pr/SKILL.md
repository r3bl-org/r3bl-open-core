---
name: merge-pr
description:
  Workflow for rebasing and merging a local branch to main via a GitHub Pull Request.
---

# Merge PR Workflow

This skill codifies the workflow for taking a completed local task branch, pushing it to GitHub, creating a Pull Request, and merging it via rebase.

## When to Use

Use this skill when the user asks to "close the pr by rebasing and merging to main" or explicitly invokes `/merge-pr`.

## The Workflow

### Step 1: Ensure Clean State & Push

- Verify the git working tree is clean. If not, ask the user to commit or stash changes.
- Push the current branch to origin using `git push -f origin HEAD`. We use `-f` because local commits are frequently amended during code reviews.

### Step 2: Merge via Rebase

- Merge the existing PR into main using the rebase strategy and delete the remote branch: `gh pr merge --rebase --delete-branch`.
- This ensures a linear commit history on the main branch without ugly merge commits.

### Step 3: Cleanup

- Switch back to the main branch: `git checkout main`.
- Pull the latest changes to sync up with origin: `git pull`.
- Prune stale remote tracking branches: `git fetch --prune`.
- Delete the local task branch you just merged: `git branch -D <branch-name>`.
