# Guide to pull requests (PR)

<a id="markdown-guide-to-pull-requests-pr" name="guide-to-pull-requests-pr"></a>

<!-- TOC -->

- [🎈 Why is it important to write good PRs?](#-why-is-it-important-to-write-good-prs)
- [🐒 Workflow on how to create a PR](#-workflow-on-how-to-create-a-pr)
- [✏️ How to structure a PR](#-how-to-structure-a-pr)
  - [Title](#title)
  - [Description](#description)
  - [Checklist](#checklist)
  - [One commit per PR](#one-commit-per-pr)

<!-- /TOC -->

## 🎈 Why is it important to write good PRs?

<a id="markdown-%F0%9F%8E%88-why-is-it-important-to-write-good-prs%3F" name="%F0%9F%8E%88-why-is-it-important-to-write-good-prs%3F"></a>

Good PRs are important because they help maintainers understand the changes you've made to the
codebase. They also help you keep track of what you've done.

> 🔧🔩 Feel free to create a PR and mark it as a `draft` if you want to get feedback from the
> maintainers. We are supporters of early feedback and iteration over completion.

## 🐒 Workflow on how to create a PR

<a id="markdown-%F0%9F%90%92-workflow-on-how-to-create-a-pr" name="%F0%9F%90%92-workflow-on-how-to-create-a-pr"></a>

If you're interested in contributing code, updating docs, etc., please follow these steps:

1. Fork the repository on GitHub.
2. Clone your forked repository to your local machine.
3. Create a new branch for your changes. Please read our
   [guide to naming a 🌳 branch](BRANCH.md).
4. Make your changes and commit them with a descriptive commit message. Please read our
   [guide to writing a 📝 commit message](COMMIT_MESSAGE.md).
5. Write tests when relevant.
6. Run tests: `fish run.fish test` (works on all OSs) or simply `./run.nu test` (works on MacOS and
   Linux). Take a look at
   [other scripts you can run](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui#run-the-demo-locally).
7. Run the code to make sure everything works `fish run.fish run` or `./run.nu run` if you are using
   MacOS or Linux.
8. Squash your commits into **one commit**.
9. Push your changes to your forked repository.
10. Submit a pull request to our repository.

> 📹 Videos available on YouTube that explain the GitHub pull request workflow:
>
> - [What is a pull request?](https://www.youtube.com/watch?v=For9VtrQx58)
> - [Creating a Simple Github Pull Request](https://www.youtube.com/watch?v=rgbCcBNZcdQ)
>
> These videos provide step-by-step instructions on how to create a pull request, review code, and
> merge changes into a repository.

## ✏️ How to structure a PR

<a id="markdown-%E2%9C%8F%EF%B8%8F-how-to-structure-a-pr" name="%E2%9C%8F%EF%B8%8F-how-to-structure-a-pr"></a>

Feel free to add/remove sections as needed. The sections below are just a suggestion.

```
Title: A clear and concise title that summarizes the changes (please see below for more details).

# Description:
- If your PR closes an issue, add `Closes #<issue number>` (e.g. Closes #224) to the description of the PR. This will automatically close the issue when the PR is merged.
- A detailed description of the changes that were made.

## Checklist:
A checklist to ensure that all of the necessary steps have been completed.

- [ ] I have added tests that prove my fix is effective or that my feature works.
- [ ] I have manually tested my changes and they work as intended.
- [ ] I have read the [guide to writing a PR](https://github.com/r3bl-org/r3bl-open-core/blob/main/docs/contributing_guides/PULL_REQUEST.md).

## Steps to reproduce the new behavior (if relevant):
1. ...
2. ...

Screenshots or other relevant media if applicable.

Commit message:
I have squashed all my commits to one commit that follows the [guide on how to write a commit message](https://github.com/r3bl-org/r3bl-open-core/blob/main/docs/contributing_guides/COMMIT_MESSAGE.md).
```

### Title

<a id="markdown-title" name="title"></a>

- The title should start with the category name in square brackets (e.g. [tui][docs]), if relevant,
  followed by a **short description** of the issue.
- The title should be in the **imperative mood**. Using the imperative mood means to phrase your
  commit description as an order or instruction. For example, instead of writing "Fixed bug in user
  login", you would write **"Fix a bug in user login"**. This makes it clear and concise, describing
  what applying the commit will do rather than what has been done.
- The title should not end with a period.
- The title should start with a capital letter.

### Description

<a id="markdown-description" name="description"></a>

If your PR closes an issue, add `Closes #<issue number>` (e.g. Closes #224) to the description of
the PR. This will automatically close the issue when the PR is merged.

Add a description of the PR.

### Checklist

<a id="markdown-checklist" name="checklist"></a>

Take a look at the checklist items above in the sample PR snippet. Feel free to add more checklist
items if relevant.

### One commit per PR

<a id="markdown-one-commit-per-pr" name="one-commit-per-pr"></a>

- Make sure your PR has only one commit.
- If you have multiple commits, squash them into one commit before submitting the PR. See
  [guide to writing a commit message](COMMIT_MESSAGE.md).
