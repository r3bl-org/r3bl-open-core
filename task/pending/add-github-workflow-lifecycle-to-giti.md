# Task: Add GitHub Workflow Lifecycle to Giti

## 1. Overview
- **Goal:** Elevate `giti` from a standard git client to a full-fledged GitHub workflow orchestrator by porting our agentic `<verb>-<noun>` PR lifecycle.
- **Context:** We recently codified a highly successful, interconnected Git/GitHub lifecycle for our AI agents using slash commands (`/fix-issue`, `/review-pr`, `/create-pr`, `/merge-pr`). This workflow dramatically reduces cognitive load and ensures repository cleanliness. `giti` should formalize these journeys as first-class TUI features.

## 2. Problem Space & Constraints
- `giti` currently handles basic git operations (checkout, branch creation, deletion) but lacks GitHub orchestration.
- Developers currently have to drop out of `giti` and use `gh` CLI or a browser to create, review, and merge PRs.
- **Constraints:** Needs to shell out to the `gh` CLI (or use a GitHub API crate). Must fit naturally into `giti`'s existing interactive TUI design patterns (list selections, dialogs).

## 3. The Workflows to Formalize

We need to build interactive TUI journeys inside `giti` that perfectly mirror our agent slash commands:

### A. The Start: Issue & PR Creation
- **Mirroring `/fix-issue`:** A TUI flow that queries open GitHub issues, lets the user select one, automatically creates a new branch (e.g., `issue-<id>`), and immediately opens a Draft PR to track the work.
- **Mirroring `/create-pr`:** A shortcut to take an ad-hoc local branch the user just made, push it to origin, and instantly open a PR (`gh pr create --fill`).

### B. The Middle: Reviewing Community Code
- **Mirroring `/review-pr`:** A TUI flow that lists open community PRs, fetches the selected one, checks it out locally, and prepares the workspace for an audit and rewrite.

### C. The End: Clean Merging (The Capstone)
- **Mirroring `/merge-pr`:** The ultimate endpoint. When the user is done with a branch, `giti` should offer a one-button "Merge and Clean" action. This macro-action will:
  1. Force-push any final local commits.
  2. Merge the PR linearly via rebase, whilst deleting the remote branch (`gh pr merge --rebase --delete-branch`).
  3. Clean up the local workspace automatically (`git checkout main`, `git pull`, `git fetch --prune`, `git branch -D`).

## 4. Implementation Plan

- [ ] **Phase 1: Architecture & Auth**
  - Add `gh` CLI integration or a GitHub API client to `giti`.
  - Ensure authentication state is handled gracefully.
- [ ] **Phase 2: The "Start" Flows**
  - Implement the Issue Picker UI.
  - Implement automatic branch and Draft PR creation.
- [ ] **Phase 3: The "End" Flows**
  - Implement the "Merge & Clean" macro that flawlessly executes the 4-step `/merge-pr` cleanup pipeline.
- [ ] **Phase 4: Integration & Testing**
  - Ensure shell-outs to `gh` or `git` are non-blocking and display proper loading states in the TUI.
  - **Mandatory manual review:** Test all 4 flows on a dummy repository.
