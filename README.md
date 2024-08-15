# r3bl-open-core

Welcome to this [monorepo](https://en.wikipedia.org/wiki/Monorepo). All the folders in
this repo are separate Rust projects (crates) that are probably published to crates.io.
And this constitutes a Rust workspace.

Here's the [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md)
for this monorepo or Rust workspace. This is a great place to start to get familiar with
what has changed recently in each of the projects in this Rust workspace.

# The workspace contains many TUI, CLI, TTY crates

The following is a high level overview of each of the crates that constitute this Rust
[workspace](https://github.com/r3bl-org/r3bl-open-core).

There are crates that range from "full" TUI to "partial" TUI, and everything in the middle.

## Full TUI (async, raw mode, full screen) for immersive TUI apps

[`r3bl_tui`](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui) gives you raw mode
"alternate screen" and "full screen" support, while being totally async. An example of
this is the "Full TUI" app `edi` in the `r3bl-cmdr` crate. You can install & run this with
the following command:

```sh
cargo install r3bl-cmdr
edi
```

## Partial TUI (async, partial raw mode) for async REPL and shell programs

[`r3bl_terminal_async`](https://github.com/r3bl-org/r3bl-open-core/tree/main/terminal_async)
gives you the ability to easily build your own async shell programs using "async readline
& stdout".

Here are examples of this:
1. https://github.com/nazmulidris/rust-scratch/tree/main/tcp-api-server
2. https://github.com/r3bl-org/r3bl-open-core/tree/main/terminal_async/examples

## Minimum TUI (sync, blocking, partial raw mode) for simple CLI programs with blocking interaction

[`r3bl_tuify`](https://github.com/r3bl-org/r3bl-open-core/tree/main/tuify) gives you the
ability to easily build your own CLI programs with blocking interaction. This is a great
to get user input, while blocking the main thread, and using raw mode while the main thread is blocked.
An example app of this is the `giti` app in the `r3bl-cmdr` crate. You can install & run this with
the following command:

```sh
cargo install r3bl-cmdr
giti
```

## Underlying crates

There are many other underlying crates that are used to build these top level crates.
Here's a short list of them:

- [`r3bl_rs_utils_core`](https://github.com/r3bl-org/r3bl-open-core/tree/main/core)
  contains lots of low level utilities that are used in the other crates. This includes
  things like declarative macros, colors, styles, unicode support, etc. Over time, if some
  code is created in a "higher level" crate, and it's useful in other crates, it's moved
  to this crate. And this is documented in the
  [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md).

- [`r3bl_test_fixtures`](https://github.com/r3bl-org/r3bl-open-core/tree/main/test_fixtures)
  contains lots of test fixtures that are used in the other crates. This includes things
  like mocks for stdio, and event streams (input events that are generated by user
  interaction).

- [`r3bl_ansi_color`](https://github.com/r3bl-org/r3bl-open-core/tree/main/ansi_color) is
  a somewhat unrelated crate to the others in this workspace. It provides a clean API that
  allows you to easily use ANSI colors in your terminal programs. If you don't want to use
  the more complex crates and you just need to output some styled text to the terminal,
  then this is the crate for you.

## Top level user facing crate

There's even a crate that only contains user facing apps that are built using these
underlying crates. This is the
[`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate, which
gives you the `giti` and `edi` apps (described above). You can install & run this with the
following command:

```sh
cargo install r3bl-cmdr
```

# Building the workspace, CI/CD, and testing

There's a `nushell` script that you can use to run the CI/CD pipeline for this workspace,
and more (local only operations). To get a list of these, you can view the `nushell`
script in the root of this repo
[`run`](https://github.com/r3bl-org/r3bl-open-core/blob/main/run). To get an idea of the
commands that you can run, try running the following command:

```sh
cargo install nu
nu run
```

You should see output that looks like this:

```text
Usage: run <command> [args]
<command> can be:
    all
    all-cicd
    build
    build-full
    clean
    install-cargo-tools
    test
    docs
    check
    check-watch
    clippy
    clippy-watch
    serve-docs
    upgrade-deps
    rustfmt
    help
```

For example:
- The `nu run all-cicd` command will run the CI/CD pipeline for this workspace.
- However, you can run the `nu run all` command to run the CI/CD pipeline, and more (local
  only operations).

Each crate that's contained in this workspace may also have its own `nushell` script that
is also named `run`. This is a convention that is used in this workspace. You can run the
`run` script in each of the crates to get a list of commands that are specific to that
crate.