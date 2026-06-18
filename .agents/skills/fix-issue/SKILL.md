---
name: fix-issue
description:
  Guidelines for deeply exploring the problem space first and designing the best solution
  before implementing.
---

# Issue Fix Workflow: Deep Exploration and Design

This skill codifies the "slow and steady" approach to solving non-trivial issues or adding
features. It enforces exploring the problem space, considering alternative designs, and
formalizing the design before writing any implementation code.

## When to Use

Use this skill whenever you are:

- Creating a new task file for a non-trivial issue or feature.
- Asked by the user to "explore first", "design first", or "write a design doc".
- Working on complex or architectural changes that span multiple components.

## The Design-First Workflow

The workflow consists of four distinct steps:

```
┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐      ┌───────────────────┐
│ 1. Deep Research  │ ───► │  2. Design Doc    │ ───► │  3. User Review   │ ───► │ 4. Implementation │
│ (No Code Written) │      │  (Task MD File)   │      │   & Approval      │      │ (Iterative Step)  │
└───────────────────┘      └───────────────────┘      └───────────────────┘      └───────────────────┘
```

### Step 1: Deep Research (No Code Written)

Before proposing any changes or writing a task file, gather context on the problem space:

1. Read the issue description (e.g., via `gh issue view <id>`).
2. Search the codebase to locate all relevant files, traits, types, and usages.
3. Understand the control flow, thread boundaries, and lifecycle constraints.
4. Do NOT write or modify any code during this step.

### Step 2: Design Doc (Task File Creation)

Create the task file under `task/issue-<id>-fix.md` or `task/<name>.md`. 
Create a new git branch for this task, push it, and open a Draft PR using `gh pr create --draft --fill` to track the work.

Instead of a simple todo list, format this task file as a **Design Document** with the following sections:

#### 1. Overview

- Clear statement of the goal.
- Context and reference to GitHub issues or PRs.

#### 2. Problem Space & Constraints

- Detailed analysis of the current code.
- Specific limitations of the existing design.
- Architectural and safety constraints (e.g., thread safety, locks, lifetimes, platform
  compatibility).

#### 3. Design Alternatives

Present at least two (ideally three) alternative approaches. For each alternative, detail:

- **How it works**: High-level explanation of the API and structural changes.
- **Pros**: Benefits of this approach.
- **Cons**: Drawbacks, complexity, or limitations.
- **Assessment**: Feasibility, effort, and alignment with the project's design philosophy.

#### 4. Proposed Design

- Which alternative was chosen and why.
- Specific API signatures, type changes, and struct definitions.
- Visual diagram (if applicable, using ASCII or Mermaid).
- Threading, lock, and lifecycle impact analysis.

#### 5. Implementation Plan

The very last section containing flat checklists grouped under phase headers.

- Phase 1: Preparation / API Refactoring
- Phase 2: Core Implementation
- Phase 3: Integration & Testing
- Every phase MUST end with a **Mandatory manual review** checkbox list for modified
  files.

### Step 3: User Review & Approval

Present the newly created task file to the user.

- Highlight the design alternatives and the proposed design.
- Ask for feedback and wait for explicit approval before proceeding to implementation.

### Step 4: Iterative Implementation

Only after the design doc is approved, load the task and begin implementation
step-by-step.

- Run quality checks (`./check.fish`) and pause for manual reviews frequently.

### Step 5: Merge & Complete

Once the implementation is fully tested and manually reviewed:
- Remind the user to run the `/merge-pr` slash command to push the final code and merge the existing Pull Request into `main`.
