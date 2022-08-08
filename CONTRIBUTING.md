<p align="center">
  <img src="r3bl-term.svg" height="128px">
</p>

# Contributing
<a id="markdown-contributing" name="contributing"></a>


Table of contents

<!-- TOC -->

- [Feedback](#feedback)
- [Good starting points](#good-starting-points)
  - [🦜 New to terminals?](#%F0%9F%A6%9C-new-to-terminals)
  - [🐒 New to the R3BL codebase?](#-new-to-the-r3bl-codebase)
- [Developing](#developing)
  - [Set up](#set-up)
  - [Code style](#code-style)
  - [Best practices before submitting a PR](#best-practices-before-submitting-a-pr)

<!-- /TOC -->

## Feedback
<a id="markdown-feedback" name="feedback"></a>


This library crate is in service of the apps being built in the
[r3bl-cmdr](https://github.com/r3bl-org/r3bl-cmdr/) crate / project.

While the maintainers might currently prioritize working on features, we are open to ideas and
contributions by people and projects interested in using `r3bl_rs_utils` or `r3bl-cmdr` for other
projects. Please feel free to:

1. Open an [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/new/choose).
2. Chat with us on the [r3bl discord](https://discord.gg/pG4wjDnm) in the dedicated `#r3bl_rs_utils`
   channel.

## Good starting points
<a id="markdown-good-starting-points" name="good-starting-points"></a>


If you want to get started, check out the list of
[issues](https://github.com/r3bl-org/r3bl_rs_utils/issues) with the
["good first issue" label](https://github.com/r3bl-org/r3bl_rs_utils/issues?q=is%3Aissue+is%3Aopen+label%3A%22good+first+issue%22).

You can also browse the more information TODOs in [TODO.todo](TODO.todo) which haven't been turned
into issues yet.

### 🦜 New to terminals?
<a id="markdown-%F0%9F%A6%9C-new-to-terminals%3F" name="%F0%9F%A6%9C-new-to-terminals%3F"></a>


Here's a learning path to help you get started:

1. A really good first step is taking a look at `crossterm` crate - it is small and relatively
   straight forward to understand. This will give you good exposure to the underlying terminal
   stuff. Here's a link to the repo's
   [examples](https://github.com/crossterm-rs/crossterm/tree/master/examples).
2. Here's some
   [documentation](https://docs.rs/r3bl_rs_utils/0.7.41/r3bl_rs_utils/tui/crossterm_helpers/index.html)
   w/ lots of background information on terminals, PTY, TTY, etc.

### 🐒 New to the R3BL codebase?
<a id="markdown-%F0%9F%90%92-new-to-the-r3bl-codebase%3F" name="%F0%9F%90%92-new-to-the-r3bl-codebase%3F"></a>


1. A great starting point is this this [README](https://github.com/r3bl-org/r3bl_rs_utils). Here are
   some important sections:
   - [tui](https://github.com/r3bl-org/r3bl_rs_utils#tui)
   - [redux](https://github.com/r3bl-org/r3bl_rs_utils#redux)
2. Here's a [repo](https://github.com/r3bl-org/address-book-with-redux-tui/releases/tag/1.0) that is
   a good one to start working on first. This repo is for a simple address book CLI app that does
   NOT have TUI support. It is a good app to convert to using the TUI library to get a solid handle
   on how to build TUIs. This app was intended to be a pedagogical example to get a handle on this
   stuff.
3. Here are some resources to learn more about the project itself:
   - [r3bl_rs_utils repo README](https://github.com/r3bl-org/r3bl_rs_utils/blob/main/README.md).
   - [r3bl-cmdr repo README](https://github.com/r3bl-org/r3bl-cmdr/blob/main/README.md).
   - [Related content on developerlife.com](https://developerlife.com/category/Rust/).

## Developing
<a id="markdown-developing" name="developing"></a>


### Set up
<a id="markdown-set-up" name="set-up"></a>


This is no different than other Rust projects.

```bash
git clone https://github.com/r3bl-org/r3bl_rs_utils
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
