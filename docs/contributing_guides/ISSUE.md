# Guide to writing issues

<a id="markdown-guide-to-writing-issues" name="guide-to-writing-issues"></a>

<!-- TOC -->

- [🎈 Why is it important to write good issues?](#-why-is-it-important-to-write-good-issues)
- [✏️ How to write an issue](#-how-to-write-an-issue)
  - [Title](#title)
  - [Description](#description)
- [🌺 Examples](#-examples)
  - [Example 1: new feature](#example-1-new-feature)
  - [Example 2: bug fix](#example-2-bug-fix)

<!-- /TOC -->

## 🎈 Why is it important to write good issues?

<a id="markdown-%F0%9F%8E%88-why-is-it-important-to-write-good-issues%3F" name="%F0%9F%8E%88-why-is-it-important-to-write-good-issues%3F"></a>

- **Collaboration**: Good issues facilitate collaboration. They allow others to understand the
  problem, join the discussion, and contribute to the solution.
- **Prioritization**: Detailed issues help maintainers prioritize tasks based on their complexity,
  impact, and urgency.
- **Onboarding**: For new contributors, well-documented issues are a great way to understand the
  project and start contributing.

## ✏️ How to write an issue

<a id="markdown-%E2%9C%8F%EF%B8%8F-how-to-write-an-issue" name="%E2%9C%8F%EF%B8%8F-how-to-write-an-issue"></a>

### Title

<a id="markdown-title" name="title"></a>

- The title should start with the category name in square brackets (e.g. `[tui][docs]`), if relevant,
  followed by a **short description** of the issue. Also associate a label with the issue that
  matches this prefix category to the appropriate label (e.g. `tui` and `component: tui`).
- The title should be in the **imperative mood**. Using the imperative mood means to phrase your
  commit description as an order or instruction. For example, instead of writing "Fixed bug in user
  login", you would write **"Fix a bug in user login"**. This makes it clear and concise, describing
  what applying the commit will do rather than what has been done.
- The title should not end with a period.
- The title should start with a capital letter.

### Description

<a id="markdown-description" name="description"></a>

- The description should be a detailed description of the issue. Add as much detail as possible.
- If relevant, add sections called "Current behavior" and "Desired behavior" to the description.
  - If you are creating an issue for a bug, add reproduction steps (see below examples) to the
    description.
- Add screenshots, gifs, and videos if relevant. Visuals are always helpful.
- Add links to relevant issues, PRs, and other resources.
- Add labels to the issue.

## 🌺 Examples

<a id="markdown-%F0%9F%8C%BA-examples" name="%F0%9F%8C%BA-examples"></a>

### Example 1: new feature

<a id="markdown-example-1%3A-new-feature" name="example-1%3A-new-feature"></a>

```
Title: [tui][edi] Implement feature to add tags to tasks

# Description:

Currently, tasks can only be categorized by assigning them to a specific project. This can be limiting when managing tasks across multiple projects or when tasks need to be further organized.

## Proposed solution:

Implement a feature to add tags to tasks. Tags would be user-defined keywords that can be associated with tasks to provide additional context and organization.

## Benefits:

- Enhanced task organization and categorization
- Improved task filtering and search capabilities
- Increased flexibility in managing tasks

## Implementation details:

- Tags should be user-creatable and editable.
- Tags should be associated with tasks through a many-to-many relationship.
- Tags should be searchable and filterable within task lists.

## Labels:

feature
tui
edi

```

### Example 2: bug fix

<a id="markdown-example-2%3A-bug-fix" name="example-2%3A-bug-fix"></a>

```
Title: [tui][edi] Fix bug in task creation

# Description:

## Current behavior:

When creating a new task, the task is not saved to the database.

## Desired behavior:

When creating a new task, the task should be saved to the database.

## Steps to reproduce:

1. Open the task creation dialog.
2. Enter a task name.
3. Click the "Create" button.
4. Open the task list.
5. The task should be visible in the task list.

## Labels:

bug
tui
edi

```
