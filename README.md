# r3bl-open-core

<img
src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg?raw=true"
height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<!-- prettier-ignore-start -->
```
██████╗ ██████╗ ██████╗ ██╗         ████████╗██╗   ██╗██╗
██╔══██╗╚════██╗██╔══██╗██║         ╚══██╔══╝██║   ██║██║
██████╔╝ █████╔╝██████╔╝██║            ██║   ██║   ██║██║
██╔══██╗ ╚═══██╗██╔══██╗██║            ██║   ██║   ██║██║
██║  ██║██████╔╝██████╔╝███████╗       ██║   ╚██████╔╝██║
╚═╝  ╚═╝╚═════╝ ╚═════╝ ╚══════╝       ╚═╝    ╚═════╝ ╚═╝
```
<!-- prettier-ignore-end -->

Table of contents:

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Why R3BL TUI?](#why-r3bl-tui)
  - [The Problem with Existing Solutions](#the-problem-with-existing-solutions)
  - [The R3BL Solution: Web and Desktop App Inspired Terminal Apps](#the-r3bl-solution-web-and-desktop-app-inspired-terminal-apps)
  - [Built-from-Scratch Primitives](#built-from-scratch-primitives)
  - [Advanced Rendering & Styling](#advanced-rendering--styling)
  - [Rich Component Ecosystem](#rich-component-ecosystem)
- [Welcome to the monorepo and workspace](#welcome-to-the-monorepo-and-workspace)
- [This workspace contains crates for building TUI, CLI, TTY apps](#this-workspace-contains-crates-for-building-tui-cli-tty-apps)
  - [Full TUI (async, raw mode, full screen) for immersive TUI apps](#full-tui-async-raw-mode-full-screen-for-immersive-tui-apps)
  - [Partial TUI (async, partial raw mode, async readline) for choice based user interaction](#partial-tui-async-partial-raw-mode-async-readline-for-choice-based-user-interaction)
  - [Partial TUI (async, partial raw mode, async readline) for async REPL](#partial-tui-async-partial-raw-mode-async-readline-for-async-repl)
- [Power via composition](#power-via-composition)
  - [Main library crate](#main-library-crate)
  - [Main binary crate](#main-binary-crate)
- [Project Task Organization](#project-task-organization)
  - [Task Management Files](#task-management-files)
  - [Workflow Connection](#workflow-connection)
- [Documentation and Planning](#documentation-and-planning)
  - [Release and Contribution Guides](#release-and-contribution-guides)
  - [Technical Design Documents](#technical-design-documents)
- [Learn how these crates are built, provide feedback](#learn-how-these-crates-are-built-provide-feedback)
- [Quick Start](#quick-start)
  - [Automated Setup (Recommended)](#automated-setup-recommended)
  - [Manual Setup](#manual-setup)
- [Build the workspace and run tests](#build-the-workspace-and-run-tests)
  - [Key Commands](#key-commands)
  - [Build Cache (using sccache) Verification](#build-cache-using-sccache-verification)
  - [Rust Toolchain Management](#rust-toolchain-management)
  - [Unified Script Architecture](#unified-script-architecture)
- [Star History](#star-history)
- [Archive](#archive)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

## Why R3BL TUI?

After leaving Google in 2021, I ([Nazmul Idris](https://developerlife.com/about-me/)) embarked on
creating infrastructure for modern, powerful CLI and TUI experiences built from the ground up in
Rust.

The core architectural innovation: a purely async, immediate mode reactive UI (every state change
triggers a render from scratch) where nothing blocks the main thread - unlike traditional approaches
using platform-specific blocking operations like POSIX
[`readline()`](https://man7.org/linux/man-pages/man3/readline.3.html) on Linux/macOS or Windows
[`ReadConsole()`](https://learn.microsoft.com/en-us/windows/console/readconsole).

R3BL TUI is fundamentally different from [`vim`](https://www.vim.org/),
[`neovim`](https://neovim.io/), and [`ratatui`](https://ratatui.rs/) through its immediate mode
reactive UI with clean separation between rendering and state mutation, and purely async nature.

This fully async, responsive framework works seamlessly across Linux, macOS, and Windows. It's
optimized for use over SSH connections by painting only diffs, and handles complex concurrent
operations with low latency while ensuring no thread blocking.

### The Problem with Existing Solutions

I initially tried [Node.js](https://nodejs.org/) with
[ink](https://developerlife.com/2021/11/25/ink-v3-advanced-ui-components/), but encountered
fundamental limitations:

- Module incompatibilities and dependency conflicts
- Limited control over keybindings and terminal behavior
- High resource consumption for simple tasks
- Screen flickering and poor rendering performance

### The R3BL Solution: Web and Desktop App Inspired Terminal Apps

Our framework supports the full spectrum from CLI to hybrid TUI to full TUI experiences with deep
system integration.

**Key Innovation: "Applets"** - A revolutionary state management system that allows processes to
persist state across their lifecycle and share it with other instances or processes.

### Built-from-Scratch Primitives

**Async Readline**: Unlike POSIX readline which is single-threaded and blocking, our implementation
is fully async, interruptable, and non-blocking.

**Choose API**: Single-shot user interactions that enter raw mode without taking over the screen or
disrupting the terminal's back buffer.

**Full TUI**: Complete raw mode with alternate screen support, fully async and non-destructive.

All components are end-to-end testable using our InputDevice and OutputDevice abstractions for
stdin, stdout, and stderr.

### Advanced Rendering & Styling

- **CSS-like styling** with JSX-inspired declarative layouts
- **Gradient color support** with automatic terminal capability detection
- **Double-buffered compositor** for efficient rendering
- **Comprehensive color support** that adapts to terminal capabilities (even handles macOS
  Terminal.app's lack of truecolor support)

### Rich Component Ecosystem

- Beautiful Markdown parser with syntax highlighting
- Rich text editor components
- Dialog box support
- Animation framework (in development)
- Process orchestration via the "script" module
- Async REPL infrastructure

R3BL TUI brings the power and ergonomics of modern web development to the terminal, creating a new
paradigm for command-line productivity tools.

We are working on building command line apps in Rust which have rich text user interfaces (TUI). We
want to lean into the terminal as a place of productivity, and build all kinds of awesome apps for
it.

1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
   development w/ a twist: taking concepts that work really well for the frontend mobile and web
   development world and re-imagining them for TUI & Rust.

- Taking inspiration from things like [React](https://react.dev/),
  [SolidJS](https://www.solidjs.com/), [Elm](https://guide.elm-lang.org/architecture/),
  [iced-rs](https://docs.rs/iced/latest/iced/),
  [Jetpack Compose](https://developer.android.com/compose),
  [JSX](https://ui.dev/imperative-vs-declarative-programming),
  [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can be run in parallel
  & concurrent via [Tokio](https://crates.io/crates/tokio)).
- Even the thread running the main event loop doesn't block since it is async.
- Using macros to create DSLs to implement something inspired by
  [CSS](https://www.w3.org/TR/CSS/#css) &
  [JSX](https://ui.dev/imperative-vs-declarative-programming).

2. 🌎 We are building apps to enhance developer productivity & workflows.

- The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a single terminal
  window). Rather it is to build a set of integrated "apps" (or "tasks") that run in the same
  process that renders to one terminal window.
- Inside of this terminal window, we can implement things like "applet" switching, routing, tiling
  layout, stacking layout, etc. so that we can manage a lot of TUI apps (which are tightly
  integrated) that are running in the same process, in the same window. So you can imagine that all
  these "applets" have shared application state. Each "applet" may also have its own local
  application state.
- You can mix and match "Full TUI" with "Partial TUI" to build for whatever use case you need.
  `r3bl_tui` allows you to create application state that can be moved between various "applets",
  where each "applet" can be "Full TUI" or "Partial TUI".
- Here are some examples of the types of "app"s we plan to build (for which this infrastructure acts
  as the open source engine):
  1. Multi user text editors w/ syntax highlighting.
  2. Integrations w/ github issues.
  3. Integrations w/ calendar, email, contacts APIs.

## Welcome to the monorepo and workspace

All the crates in the `r3bl-open-core` [monorepo](https://en.wikipedia.org/wiki/Monorepo) provide
lots of useful functionality to help you build TUI (text user interface) apps, along w/ general
niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉.

Any top-level folder in this repository that contains a `Cargo.toml` file is a Rust project, also
known as a [crate](https://doc.rust-lang.org/book/ch07-01-packages-and-crates.html). These crates
are likely published to [crates.io](https://crates.io/crates/r3bl_tui). Together, they form a
[Rust workspace](https://doc.rust-lang.org/book/ch14-03-cargo-workspaces.html).

Here's the [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md) for this
monorepo containing a Rust workspace. The changelog is a great place to start to get familiar with
what has changed recently in each of the crates in this Rust workspace.

## This workspace contains crates for building TUI, CLI, TTY apps

The [`r3bl_tui`](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui) crate is the main crate
that contains the core functionality for building TUI apps. It allows you to build apps that range
from "full" TUI to "partial" TUI, and everything in the middle.

Here are some videos that you can watch to get a better understanding of TTY programming.

- [Build with Naz: TTY playlist](https://www.youtube.com/playlist?list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3)
- [Build with Naz: async readline](https://www.youtube.com/playlist?list=PLofhE49PEwmwelPkhfiqdFQ9IXnmGdnSE)

### Full TUI (async, raw mode, full screen) for immersive TUI apps

[`tui`](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui/src/tui) gives you "raw mode",
"alternate screen" and "full screen" support, while being totally async. An example of this is the
"Full TUI" app `edi` in the [`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr)
crate. You can install & run this with the following command:

```sh
cargo install r3bl-cmdr
edi
```

### Partial TUI (async, partial raw mode, async readline) for choice based user interaction

[`choose`](https://github.com/r3bl-org/r3bl-open-core/blob/main/tui/src/readline_async/choose_api.rs)
allows you to build less interactive apps that ask a user user to make choices from a list of
options and then use a decision tree to perform actions.

An example of this is this "Partial TUI" app `giti` in the
[`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate. You can install &
run this with the following command:

```sh
cargo install r3bl-cmdr
giti
```

### Partial TUI (async, partial raw mode, async readline) for async REPL

[`readline_async`](https://github.com/r3bl-org/r3bl-open-core/blob/main/tui/src/readline_async/readline_async_api.rs)
gives you the ability to easily ask for user input in a line editor. You can customize the prompt,
and other behaviors, like input history.

Using this, you can build your own async shell programs using "async readline & stdout". Use
advanced features like showing indeterminate progress spinners, and even write to stdout in an async
manner, without clobbering the prompt / async readline, or the spinner. When the spinner is active,
it pauses output to stdout, and resumes it when the spinner is stopped.

An example of this is this "Partial TUI" app `giti` in the
[`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate. You can install &
run this with the following command:

```sh
cargo install r3bl-cmdr
giti
```

Here are other examples of this:

1. https://github.com/nazmulidris/rust-scratch/tree/main/tcp-api-server
2. https://github.com/r3bl-org/r3bl-open-core/tree/main/tui/examples

## Power via composition

You can mix and match "Full TUI" with "Partial TUI" to build for whatever use case you need.
`r3bl_tui` allows you to create application state that can be moved between various "applets", where
each "applet" can be "Full TUI" or "Partial TUI".

### Main library crate

There is just one main library crate in this workspace:
[`r3bl_tui`](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui).

### Main binary crate

There is just one main binary crate that contains user facing apps that are built using the library
crates: [`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr). This crate
contains these apps:

- `giti`: Interactive git workflows made easy.
- `edi`: Beautiful Markdown editor with advanced rendering and editing features.

You can install & run this with the following command:

```sh
cargo install r3bl-cmdr
# Interactive git workflows made easy.
giti --version
# Beautiful Markdown editor with advanced rendering and editing features.
edi --version
```

## Project Task Organization

This project uses three root-level Markdown files to organize day-to-day development work:

### Task Management Files

- **[`todo.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/todo.md)** - Active tasks and
  immediate priorities that need attention
- **[`done.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/done.md)** - Completed tasks
  and achievements, providing a historical record of progress
- **[`claude.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/claude.md)** - AI assistant
  interaction logs and collaborative planning sessions

### Workflow Connection

The task organization workflow connects with the documentation in `docs/` as follows:

- **Strategic to Tactical**: Items from `docs/` planning files (strategic goals, feature designs)
  are broken down into actionable tasks and copied into `todo.md`
- **Planning to Execution**: Complex features get documented in `docs/` first, then their
  implementation steps flow into the daily task management system
- **Documentation of Decisions**: AI-assisted development sessions and decision-making processes are
  logged in `claude.md` for future reference

This dual-level approach ensures both strategic planning (in `docs/`) and tactical execution (in
root-level `.md` files) are well-organized and connected.

## Documentation and Planning

The [`docs/`](https://github.com/r3bl-org/r3bl-open-core/tree/main/docs) folder contains
comprehensive documentation for this project, including:

### Release and Contribution Guides

- [`release-guide.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/docs/release-guide.md) -
  Step-by-step guide for releasing new versions
- [`contributing_guides/`](https://github.com/r3bl-org/r3bl-open-core/tree/main/docs/contributing_guides) -
  Detailed contribution guidelines including:
  - Branch naming conventions (`BRANCH.md`)
  - Commit message standards (`COMMIT_MESSAGE.md`)
  - Issue creation guidelines (`ISSUE.md`)
  - Pull request procedures (`PULL_REQUEST.md`)
  - Code style guide (`STYLE_GUIDE.md`)

### Technical Design Documents

- Parser strategy analysis and design decisions
- Performance optimization guides (`docs/task_tui_perf_optimize.md`)
- Architecture documentation for various components
- Feature-specific planning and design documents

The `docs/` folder serves as the central repository for:

- **Long-term planning**: Strategic goals and feature roadmaps
- **Technical decisions**: Architecture choices and implementation strategies
- **Process documentation**: How we work and contribute to the project
- **Design artifacts**: Detailed analysis of complex features before implementation

## Learn how these crates are built, provide feedback

To learn how we built this crate, please take a look at the following resources.

- If you like consuming video content, here's our
  [YT channel](https://www.youtube.com/@developerlifecom). Please consider
  [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).

## Quick Start

### Automated Setup (Recommended)

For Linux and macOS users, use the bootstrap script to automatically install all required tools:

```sh
# Clone the repository
git clone https://github.com/r3bl-org/r3bl-open-core.git
cd r3bl-open-core

# Run the bootstrap script
./bootstrap.sh
```

The [`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh) script will:

- Install Rust toolchain (rustup)
- Install Fish shell and fzf
- Install file watchers (inotifywait on Linux, fswatch on macOS)
- Install all required cargo development tools
- Detect your package manager automatically

### Manual Setup

If you prefer manual installation or are on Windows:

```sh
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Fish and fzf (via package manager)
# Ubuntu/Debian: sudo apt install fish fzf
# macOS: brew install fish fzf
# Or run ./bootstrap.sh for automatic detection

# Install development tools
fish run.fish install-cargo-tools
```

## Build the workspace and run tests

There's a unified [`fish`](https://fishshell.com/) script that you can use to run the build and
release pipeline for this workspace, and more (local only operations).

To get a list of available commands, you can review the `fish` script in the root of this repo
[`run.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/run.fish). To see all available
commands:

```sh
fish run.fish
```

You should see output that looks like this:

```text
Usage: fish run.fish <command> [args]

Workspace-wide commands:
    all                  Run all major checks
    build                Build entire workspace
    build-full           Full build with clean and update
    clean                Clean entire workspace
    test                 Test entire workspace
    check                Check all workspaces
    clippy               Run clippy on all workspaces
    clippy-pedantic      Run clippy with pedantic lints
    docs                 Generate docs for all
    serve-docs           Serve documentation
    rustfmt              Format all code
    install-cargo-tools  Install development tools
    upgrade-deps         Upgrade dependencies
    audit-deps           Security audit
    unmaintained         Check for unmaintained deps
    build-server         Remote build server - uses rsync

Watch commands:
    watch-all-tests      Watch files, run all tests
    watch-one-test [pattern]  Watch files, run specific test
    watch-clippy         Watch files, run clippy
    watch-check          Watch files, run cargo check

TUI-specific commands:
    run-examples [--release] [--no-log]  Run TUI examples
    run-examples-flamegraph-svg  Generate SVG flamegraph
    run-examples-flamegraph-fold Generate perf-folded format
    bench                Run benchmarks

cmdr-specific commands:
    run-binaries         Run edi, giti, or rc
    install-cmdr         Install cmdr binaries
    docker-build         Build release in Docker

Other commands:
    log                  Monitor log.txt in cmdr or tui directory
    help                 Show this help
```

### Key Commands

| Command                         | Description                                                     |
| ------------------------------- | --------------------------------------------------------------- |
| `fish run.fish all`             | Run all major checks (build, test, clippy, docs, audit, format) |
| `fish run.fish build`           | Build the entire workspace                                      |
| `fish run.fish test`            | Run all tests across the workspace                              |
| `fish run.fish watch-all-tests` | Watch for file changes and run tests automatically              |
| `fish run.fish run-examples`    | Run TUI examples interactively                                  |
| `fish run.fish run-binaries`    | Run cmdr binaries (edi, giti, rc) interactively                 |
| `fish run.fish update-toolchain` | Update Rust to month-old nightly toolchain with cleanup         |

### Bacon Development Tools

This project includes [bacon](https://dystroy.org/bacon/) configuration for background code checking and testing. Bacon provides real-time feedback on code changes with two distinct workflows:

**Interactive Workflow (Rich TUI with details):**
- Full terminal UI with detailed output
- Ctrl+click on errors and warnings to jump directly to source code (via OSC hyperlinks)
- Perfect for active debugging and development

**Background Workflow (Silent monitoring):**
- Minimal output - just success/failure status
- Answers simple yes/no questions like "do tests pass?" or "do docs build?"
- Ideal for background monitoring while focusing on other tasks

**Bacon commands:**

| Command | Workflow | Description |
| ------- | -------- | ----------- |
| `bacon nextest` | Interactive | Rich TUI test runner with clickable errors, detailed output, and real-time updates |
| `bacon nextest --headless --summary` | Background | Silent test runner providing only pass/fail status |
| `bacon doc` | Interactive | Documentation builder with detailed output and error navigation |
| `bacon doc --headless --summary` | Background | Silent doc builder answering "did docs generate?" with minimal output |

Choose the workflow that matches your current needs:
- Use **interactive** when actively debugging or wanting detailed feedback
- Use **background** for continuous monitoring, CI/CD pipelines, or when you just need to know if things work

**Note:** We use [`cargo-nextest`](https://nexte.st/) for running tests as it's significantly faster than `cargo test`. However, nextest does not run doctests (tests in documentation comments). To run doctests, use `cargo test --doc` separately.

### Build Cache (using sccache) Verification

This project uses [sccache](https://github.com/mozilla/sccache) to speed up Rust compilation by
caching build artifacts (configured in the `.cargo/config.toml` file). After running
`fish run.fish install-cargo-tools`, you can verify sccache is working:

```sh
sccache --show-stats
# Copy to clipboard for easy sharing
sccache --show-status 2>&1 | setclip
```

This will display cache hit rates and storage information. Higher cache hit percentages indicate
faster builds through cached compilation results.

To reset the cache, you can run:

```sh
# Complete reset
sccache --zero-stats
sccache --stop-server
rm -rf ~/.cache/sccache

# Server starts automatically on next use
cargo build  # or sccache --show-stats
```

There is no need to restart the server, as it is designed to be "lazy". And running `cargo build` or
`sccache --show-stats` will automatically start the server if it is stopped.

### Rust Toolchain Management

This project uses an intelligent Rust toolchain management strategy to balance access to recent language features with stability. The `update-toolchain` command maintains a month-old nightly toolchain, avoiding bleeding-edge instability while staying current with Rust development.

```sh
fish run.fish update-toolchain
```

**What it does:**
- Calculates a nightly toolchain date from exactly 1 month ago
- Updates `rust-toolchain.toml` to use this stable-but-recent nightly
- Installs the target toolchain if not already present
- Performs aggressive cleanup by removing all old nightly toolchains except:
  - All stable toolchains (`stable-*`)
  - The newly targeted month-old nightly
- Logs all operations to `/home/nazmul/Downloads/rust-toolchain-update.log`

**Benefits:**
- **Stability**: Month-old nightlies have proven stability while providing recent features
- **Disk space savings**: Aggressive cleanup removes accumulated old toolchains
- **Consistency**: Ensures all developers use the same Rust version
- **Automation ready**: Designed to run weekly via systemd timer for hands-off maintenance

**Example output:**
```text
Target toolchain: nightly-2024-08-13
✅ Successfully updated rust-toolchain.toml
✅ Successfully installed/verified nightly-2024-08-13
Removed 3 old toolchain(s)
Toolchains directory size before cleanup: 2.1G
Toolchains directory size after cleanup: 1.4G
```

### Unified Script Architecture

The root-level `run.fish` script consolidates functionality that was previously scattered across
multiple workspace-specific scripts. This unified approach provides:

- **Workspace-wide commands** that operate on the entire project
- **TUI-specific commands** for running examples and benchmarks
- **cmdr-specific commands** for binary management
- **Cross-platform file watching** using inotifywait (Linux) or fswatch (macOS)
- **Smart log monitoring** that detects and manages log files from different workspaces

All commands work from the root directory, eliminating the need to navigate between subdirectories.

## Star History

<a href="https://star-history.com/#r3bl-org/r3bl-open-core&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=r3bl-org/r3bl-open-core&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=r3bl-org/r3bl-open-core&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=r3bl-org/r3bl-open-core&type=Date" />
 </picture>
</a>

## Archive

As this repo grows, changes, and matures, pruning is necessary. The
[`r3bl-open-core-archive`](https://github.com/r3bl-org/r3bl-open-core-archive) is where all the code
and artifacts that are no longer needed are moved to.

This way nothing is "lost" and if you need to use some of the code that was removed, you can find it
there.

Also if you want to make changes to this code and maintain it yourself, please let us know.

1. You can submit PRs and we can also accept them, and publish them to crates.io if that makes
   sense.
2. Or we can even work out and arrangements to move ownership of the code & crate to you if you
   would like to commit to maintaining it.
