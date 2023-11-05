<p align="center">
  <img src="r3bl-term.svg" height="128px">
</p>

# Contributing to r3bl_rs_utils crate
<a id="markdown-contributing-to-r3bl_rs_utils-crate" name="contributing-to-r3bl_rs_utils-crate"></a>


<!-- TOC -->

- [Getting Started](#getting-started)
- [Discord server](#discord-server)
- [How to Contribute](#how-to-contribute)
  - [Bug Reports and Feature Requests](#bug-reports-and-feature-requests)
  - [Documentation Improvements](#documentation-improvements)
  - [Code Contributions](#code-contributions)
- [License](#license)
- [New to the entire codebase?](#new-to-the-entire-codebase)
- [Code of conduct and code style guide](#code-of-conduct-and-code-style-guide)
  - [Commit message guidelines](#commit-message-guidelines)
    - [Why are good commit messages important?](#why-are-good-commit-messages-important)
    - [What makes a good commit message?](#what-makes-a-good-commit-message)
    - [Examples of good commit messages](#examples-of-good-commit-messages)
- [Good starting points](#good-starting-points)
  - [🦜 New to terminals?](#%F0%9F%A6%9C-new-to-terminals)
  - [🐒 New to R3BL codebase?](#-new-to-r3bl-codebase)
    - [Redux background study](#redux-background-study)
    - [TUI background study](#tui-background-study)
    - [General background study](#general-background-study)
- [Developing](#developing)
  - [Set up](#set-up)
  - [Code style](#code-style)
  - [Best practices before submitting a PR](#best-practices-before-submitting-a-pr)

<!-- /TOC -->

# Contributing to r3bl_rs_utils
<a id="markdown-contributing-to-r3bl_rs_utils" name="contributing-to-r3bl_rs_utils"></a>


Thank you for considering contributing to r3bl_rs_utils! We welcome contributions from everyone,
regardless of your level of experience or expertise.

## Getting Started
<a id="markdown-getting-started" name="getting-started"></a>


Before you start contributing, please take a moment to read the
[Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct). We expect all contributors to
abide by this code of conduct in all interactions related to the project.

If you're new to the project, you may want to check out our [README](README.md) file for an overview
of the project and its goals.

## Discord server
<a id="markdown-discord-server" name="discord-server"></a>

[Discord server](https://discord.gg/8QhApTwqgA) to chat with us and discuss ideas, potential
contributions, and any questions you may have.

## How to Contribute
<a id="markdown-how-to-contribute" name="how-to-contribute"></a>


We welcome contributions in the form of bug reports, feature requests, documentation improvements,
and code contributions.

Here are some guidelines to help you get started:

### Bug Reports and Feature Requests
<a id="markdown-bug-reports-and-feature-requests" name="bug-reports-and-feature-requests"></a>


If you encounter a bug or have an idea for a new feature, please open an issue on our
[GitHub repository](https://github.com/r3bl-org/r3bl-open-core/issues). Please provide as much detail
as possible, including steps to reproduce the issue and any relevant error messages.

### Documentation Improvements
<a id="markdown-documentation-improvements" name="documentation-improvements"></a>


If you notice any errors or omissions in our documentation, please open an issue on our
[GitHub repository](https://github.com/r3bl-org/r3bl-open-core/issues) or submit a pull request with
your proposed changes.

### Code Contributions
<a id="markdown-code-contributions" name="code-contributions"></a>


If you're interested in contributing code to the project, please follow these steps:

1. Fork the repository on GitHub.
2. Clone your forked repository to your local machine.
3. Create a new branch for your changes.
4. Make your changes and commit them with a descriptive commit message.
5. Push your changes to your forked repository.
6. Submit a pull request to our repository.

> 📹 Videos available on YouTube that explain the GitHub pull request workflow.
>
> - [GitHub Pull Requests Tutorial](https://www.youtube.com/watch?v=rgbCcBNZcdQ)
> - [GitHub Pull Requests: How to Create a Pull Request](https://www.youtube.com/watch?v=For9VtrQx58)
> - [GitHub Pull Requests: How to Review Code](https://www.youtube.com/watch?v=HW0RPaJqm4g)
>
> These videos provide step-by-step instructions on how to create a pull request, review code, and
> merge changes into a repository. They also cover best practices and common pitfalls to avoid.

Please make sure your code adheres to our coding standards and passes all tests before submitting a
pull request. We also recommend that you open an issue to discuss your proposed changes before
submitting a pull request.

## License
<a id="markdown-license" name="license"></a>


r3bl_rs_utils is released under the [Apache 2.0](LICENSE).

## New to the entire codebase?
<a id="markdown-new-to-the-entire-codebase%3F" name="new-to-the-entire-codebase%3F"></a>


Here's an onboarding guide to help you get started:

- <https://github.com/r3bl-org/onboarding>

If you are new to Rust and terminals, this is a good place to begin!

## Code of conduct and code style guide
<a id="markdown-code-of-conduct-and-code-style-guide" name="code-of-conduct-and-code-style-guide"></a>


1. Please follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct) all
   the way through!
2. [R3BL code style guide](https://github.com/r3bl-org/guidelines/blob/main/STYLE_GUIDE.md).

> Also follow the guidelines provided in this [repo](https://github.com/r3bl-org/guidelines).

### Commit message guidelines
<a id="markdown-commit-message-guidelines" name="commit-message-guidelines"></a>

#### Why are good commit messages important?
<a id="markdown-why-are-good-commit-messages-important%3F" name="why-are-good-commit-messages-important%3F"></a>

Good commit messages are important because they help other developers understand the changes you've
made to the codebase. They also help you keep track of what you've done and why you did it. A good
commit message should be clear, concise, and informative.

#### What makes a good commit message?
<a id="markdown-what-makes-a-good-commit-message%3F" name="what-makes-a-good-commit-message%3F"></a>

A good commit message should follow these guidelines:

1. **Start with a summary**: The first line of the commit message should be a short summary of the
   changes you've made. It should be no more than 50 characters and should describe what the commit
   does.

2. **Provide more detail**: After the summary, provide more detail about the changes you've made.
   This can include why you made the changes, how you made them, and any other relevant information.

3. **Use the imperative mood**: Use the imperative mood (e.g. "Fix bug" instead of "Fixed bug") to
   describe what the commit does. This makes the commit message more clear and concise.

4. **Separate subject from body**: Separate the subject (the first line) from the body of the commit
   message with a blank line. This makes the commit message easier to read.

5. **Keep it short**: Keep the commit message short and to the point. Aim for no more than 72
   characters per line.

6. **Use present tense**: Use present tense to describe what the commit does. For example, "Add
   feature" instead of "Added feature".

7. **Reference issues**: If the commit is related to an issue or pull request, reference it in the
   commit message. For example, "Fix bug #123" or "Implement feature #456".

#### Examples of good commit messages
<a id="markdown-examples-of-good-commit-messages" name="examples-of-good-commit-messages"></a>

Here are some examples of good commit messages:

```
Add new feature to dashboard

This commit adds a new feature to the dashboard that allows users to
filter by date range. It also includes tests for the new feature.
```

```
Fix bug in login form

The login form was not properly validating user input, which could
lead to security vulnerabilities. This commit fixes the bug and adds
additional validation checks.
```

```
Refactor code for readability

This commit refactors the code to make it more readable and easier
to maintain. It includes changes to variable names, function names,
and code structure.
```

```
Implement feature #123

This commit implements feature #123, which allows users to upload
images to their profile. It includes changes to the database schema,
API endpoints, and front-end code.
```

I hope this guide helps you write better commit messages!

## Good starting points
<a id="markdown-good-starting-points" name="good-starting-points"></a>


If you want to get started, check out the list of
[issues](https://github.com/r3bl-org/r3bl-open-core/issues) with the
["good first issue" label](https://github.com/r3bl-org/r3bl-open-core/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22).

You can also browse the more information TODOs in [TODO.todo](TODO.todo) which haven't been turned
into issues yet.

The learning path below will help you get started. These emoji below will give you a sense how
important the related information is to using the R3BL codebase.

| Command  | Description                                        |
| -------- | -------------------------------------------------- |
| 🍌       | Nice to know it exists                             |
| 🍌🍌     | Have a high level understanding of                 |
| 🍌🍌🍌   | Working knowledge                                  |
| 🍌🍌🍌🍌 | Critical - deep understanding & hands on exercises |

### 🦜 New to terminals?
<a id="markdown-%F0%9F%A6%9C-new-to-terminals%3F" name="%F0%9F%A6%9C-new-to-terminals%3F"></a>


1. 🍌🍌🍌🍌 A really good first step is taking a look at `crossterm` crate.
   - It is small and relatively straight forward to understand. This will give you good exposure to
     the underlying terminal stuff.
   - Here's a link to the repo's
     [examples](https://github.com/crossterm-rs/crossterm/tree/master/examples). Clone it, and play
     w/ some of these examples to make some changes and run them in your favorite terminal.
2. 🍌🍌 Here's some
   [documentation](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/tui/terminal_lib_backends/index.html)
   w/ lots of background information on terminals, PTY, TTY, etc.

### 🐒 New to R3BL codebase?
<a id="markdown-%F0%9F%90%92-new-to-r3bl-codebase%3F" name="%F0%9F%90%92-new-to-r3bl-codebase%3F"></a>


#### Redux background study
<a id="markdown-redux-background-study" name="redux-background-study"></a>


1. 🍌🍌🍌 A great starting point is the [redux](https://github.com/r3bl-org/r3bl-open-core#redux)
   section.
2. 🍌🍌🍌🍌 This [repo](https://github.com/r3bl-org/address-book-with-redux-tui/releases/tag/1.0) is
   a good one to start working on first.
   - This app was intended to be a pedagogical example.
   - This repo is for a simple address book CLI app that does _NOT_ have TUI support. But it does
     have _Redux_ support. So you don't have to learn both at the same time.
   - Check out how Redux functions here. How things work in an async manner (middlewares, etc). Run
     the code using `cargo run`, and make some changes and run it again.

#### TUI background study
<a id="markdown-tui-background-study" name="tui-background-study"></a>


1. 🍌🍌🍌🍌 A great starting point is this [tui](https://github.com/r3bl-org/r3bl-open-core#tui)
   section.

   - [Example of TUI only w/out layout](https://github.com/r3bl-org/r3bl-open-core/tree/main/src/ex_app_no_layout)
   - [Example of TUI only w/ layout](https://github.com/r3bl-org/r3bl-open-core/tree/main/src/ex_app_with_layout)

2. 🍌🍌🍌🍌 Here's a
   [repo](https://github.com/r3bl-org/address-book-with-redux-tui/releases/tag/1.0) that is a good
   one to start working on first.
   - The mission is to convert it to have support for the TUI library. This will give you a solid on
     how to build TUIs.

#### General background study
<a id="markdown-general-background-study" name="general-background-study"></a>


Here are some resources to learn more about the project itself:

- [r3bl_rs_utils repo README](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md).
- [r3bl-cmdr repo README](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md).
- [Related content on developerlife.com](https://developerlife.com/category/Rust/).

## Developing
<a id="markdown-developing" name="developing"></a>


### Set up
<a id="markdown-set-up" name="set-up"></a>


This is no different than other Rust projects.

```bash
git clone https://github.com/r3bl-org/r3bl-open-core
cd r3bl_rs_utils
# To run the tests
cargo test
```

### Code style
<a id="markdown-code-style" name="code-style"></a>


We follow the standard Rust formatting style and conventions suggested by
[clippy](https://github.com/rust-lang/rust-clippy).

### Best practices before submitting a PR
<a id="markdown-best-practices-before-submitting-a-pr" name="best-practices-before-submitting-a-pr"></a>


Before submitting a PR make sure to run:

1. for formatting (a `rustfmt.toml` file is provided):

   ```shell
   cargo fmt --all
   ```

2. the clippy lints

   ```shell
   cargo clippy
   ```

3. the test suite

   ```shell
   cargo test
   ```
