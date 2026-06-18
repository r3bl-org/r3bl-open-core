---
name: create-pr
description:
  Push local changes and create a GitHub Pull Request.
---

# Create PR Workflow

Use this skill when the user explicitly invokes `/create-pr` or asks to create a PR from their local branch.

## The Workflow

### Step 1: Ensure Clean State & Push

- Verify the git working tree is clean. If not, ask the user to commit or stash changes.
- Push the current branch to origin using `git push -u origin HEAD`.

### Step 2: Create the Pull Request

- Create a PR using the GitHub CLI: `gh pr create --fill`.
- This automatically uses the commit messages to populate the PR title and body.

### Step 3: Next Steps

- Tell the user the PR has been created and provide the link if available.
- Remind the user they can use the `/merge-pr` slash command when they are ready to merge it into `main`.
