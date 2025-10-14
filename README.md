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
- [IDE Setup and Extensions](#ide-setup-and-extensions)
  - [R3BL VSCode Extensions](#r3bl-vscode-extensions)
- [Build the workspace and run tests](#build-the-workspace-and-run-tests)
  - [Key Commands](#key-commands)
  - [Bacon Development Tools](#bacon-development-tools)
  - [Status Monitoring Scripts](#status-monitoring-scripts)
  - [Build Cache (using sccache) Verification](#build-cache-using-sccache-verification)
  - [Wild Linker (Linux)](#wild-linker-linux)
  - [Rust Toolchain Management](#rust-toolchain-management)
    - [Testing Toolchain Installation Progress](#testing-toolchain-installation-progress)
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

The [`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh) script handles **OS-level setup** with a clean main function structure and will:

- **System Package Manager Detection**: Automatically detects apt, dnf, pacman, zypper, or brew
- **Core Rust Installation**: Install Rust toolchain (rustup) and ensure cargo is in PATH
- **Compiler Setup**: Install clang compiler (required by Wild linker)
- **Development Shell**: Install Fish shell and fzf for interactive development
- **File Watching**: Install file watchers (inotifywait on Linux, fswatch on macOS)
- **Development Utilities**: Install htop, screen, tmux for system monitoring
- **Node.js Ecosystem**: Install Node.js and npm for web tooling
- **AI Integration**: Install Claude Code CLI with MCP server configuration
- **Rust Development Tools Setup**: Call `fish run.fish install-cargo-tools` for all Rust-specific tooling

**Architecture**: Uses clear function separation with main() orchestrator and dedicated functions for each concern (install_rustup, install_clang, install_shell_tools, etc.)

### Manual Setup

If you prefer manual installation or are on Windows:

```sh
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Fish and fzf (via package manager)
# Ubuntu/Debian: sudo apt install fish fzf
# macOS: brew install fish fzf
# Or run ./bootstrap.sh for automatic detection

# Install Rust development tools (after OS dependencies)
fish run.fish install-cargo-tools
```

**Note**: The manual approach requires you to install OS-level dependencies yourself. The `install-cargo-tools` command focuses specifically on **Rust development tools**:

- **cargo-binstall**: Fast binary installer (installed first as foundation)
- **uv**: Modern Python package manager (required for Serena semantic code MCP server)
- **Core Development Tools**: bacon, cargo-nextest, flamegraph, inferno, sccache
- **Workspace Management**: cargo-workspaces, cargo-cache, cargo-update
- **Code Quality**: cargo-deny, cargo-unmaintained, cargo-expand, cargo-readme
- **Wild Linker**: Fast linker with optimized .cargo/config.toml generation
- **Language Server**: rust-analyzer component
- **Smart Installation**: Uses cargo-binstall for speed with fallback to cargo install --locked
- **Shared Utilities**: Leverages utility functions from script_lib.fish for consistency

## IDE Setup and Extensions

### R3BL VSCode Extensions

For an optimal development experience with r3bl-open-core, we provide a custom VSCode extension pack
specifically designed for Rust development. This extension pack is not available on the VSCode
marketplace and must be installed manually.

**What's included:**

- **R3BL Theme** - A carefully crafted dark theme optimized for Rust and Markdown development
- **Auto Insert Copyright** - Automatically inserts copyright headers in new files
- **Semantic Configuration** - Enhanced Rust syntax highlighting with additional semantic tokens
- **Extension Pack** - Bundles all R3BL extensions for easy installation

**Benefits for r3bl-open-core development:**

- Zero manual configuration required
- Enhanced semantic highlighting for better code readability
- Automatic copyright header insertion following project standards
- Seamless integration with rust-analyzer
- Optimized color scheme for the r3bl codebase

**Installation:**

```sh
# Clone the extension repository
git clone https://github.com/r3bl-org/r3bl-vscode-extensions.git
cd r3bl-vscode-extensions

# Install extensions (works with both VSCode and VSCode Insiders)
./install.sh
```

**Prerequisites:**

- VSCode or VSCode Insiders installed
- Bash shell (for running install.sh)

**Post-installation:**

1. Restart VSCode
2. Select the R3BL Theme: `Ctrl+Shift+P` → "Preferences: Color Theme" → "R3BL Theme"
3. Configure copyright settings if needed

The extensions work seamlessly with the existing development tools mentioned in this guide,
including rust-analyzer and bacon.

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
    install-cargo-tools  Install Rust development tools
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

| Command                           | Description                                                     |
| --------------------------------- | --------------------------------------------------------------- |
| `fish run.fish all`               | Run all major checks (build, test, clippy, docs, audit, format) |
| `fish run.fish build`             | Build the entire workspace                                      |
| `fish run.fish test`              | Run all tests across the workspace                              |
| `fish run.fish install-cargo-tools` | Install Rust development tools (cargo-binstall, uv, bacon, nextest, Wild linker, sccache, etc.) |
| `fish run.fish watch-all-tests`   | Watch for file changes and run tests automatically              |
| `fish run.fish run-examples`      | Run TUI examples interactively                                  |
| `fish run.fish run-binaries`      | Run cmdr binaries (edi, giti, rc) interactively                 |
| `fish run.fish update-toolchain`  | Update Rust to month-old nightly toolchain with cleanup         |
| `fish run.fish sync-toolchain`    | Sync Rust environment to match rust-toolchain.toml              |
| `fish run.fish remove-toolchains` | Remove ALL toolchains (⚠️ destructive testing utility)          |

### Bacon Development Tools

This project includes [bacon](https://dystroy.org/bacon/) configuration for background code checking
and testing. Bacon provides real-time feedback on code changes with two distinct workflows:

**Interactive Workflow (Rich TUI with details):**

- Full terminal UI with detailed output
- Ctrl+click on errors and warnings to jump directly to source code (via OSC hyperlinks)
- Perfect for active debugging and development

**Background Workflow (Silent monitoring):**

- Minimal output - just success/failure status
- Answers simple yes/no questions like "do tests pass?" or "do docs build?"
- Ideal for background monitoring while focusing on other tasks

**Available Bacon Commands:**

**Code Quality & Checking:**

| Command            | Description                                                 |
| ------------------ | ----------------------------------------------------------- |
| `bacon check`      | Fast typecheck of default target                            |
| `bacon check-all`  | Typecheck all targets (lib, bins, tests, benches, examples) |
| `bacon clippy`     | Run clippy lints on default target                          |
| `bacon clippy-all` | Run clippy lints on all targets (keybinding: `c`)           |

**Testing:**

| Command                              | Workflow    | Description                                                              |
| ------------------------------------ | ----------- | ------------------------------------------------------------------------ |
| `bacon test`                         | Interactive | Run all tests with cargo test (includes unit, integration, and doctests) |
| `bacon test -- <pattern>`            | Interactive | Run specific test matching pattern                                       |
| `bacon doctests`                     | Interactive | Run only documentation tests (`cargo test --doc`)                        |
| `bacon nextest`                      | Interactive | Rich TUI test runner using cargo-nextest (faster, no doctests)           |
| `bacon nextest --headless --summary` | Background  | Silent test runner providing only pass/fail status                       |

**Documentation:**

| Command                          | Workflow    | Description                                       |
| -------------------------------- | ----------- | ------------------------------------------------- |
| `bacon doc`                      | Interactive | Generate documentation with detailed output       |
| `bacon doc --headless --summary` | Background  | Silent doc builder answering "did docs generate?" |
| `bacon doc-open`                 | Interactive | Generate docs and open in browser                 |

**Running & Benchmarking:**

| Command                      | Description                                                             |
| ---------------------------- | ----------------------------------------------------------------------- |
| `bacon run`                  | Build and run the project in background                                 |
| `bacon run-long`             | Run long-running processes (e.g., servers) with auto-restart on changes |
| `bacon ex -- <example_name>` | Run specific example (e.g., `bacon ex -- my-example`)                   |
| `bacon bench`                | Run performance benchmarks                                              |

Choose the workflow that matches your current needs:

- Use **interactive** when actively debugging or wanting detailed feedback
- Use **background** for continuous monitoring, CI/CD pipelines, or when you just need to know if
  things work

**Testing Notes:**

- We use [`cargo-nextest`](https://nexte.st/) for running tests as it's significantly faster than
  `cargo test`
- However, nextest does **not** run doctests (tests in documentation comments)
- Use `bacon doctests` or `bacon test` to run documentation tests
- Use `bacon test --doc` is equivalent to `bacon doctests`

### Status Monitoring Scripts

For developers who want ultra-minimal status monitoring, this project includes two bash scripts
designed for integration with the
[GNOME Executor extension](https://extensions.gnome.org/extension/2932/executor/). These scripts
provide at-a-glance status indicators in your GNOME top bar.

**Quick Status Scripts:**

| Script                      | Purpose                          | Success Output | Failure Output |
| --------------------------- | -------------------------------- | -------------- | -------------- |
| `test-status-one-line.bash` | Run tests and show emoji status  | ` 🧪✔️`        | ` 🧪❌`        |
| `doc-status-one-line.bash`  | Build docs and show emoji status | ` 📚✔️`        | ` 📚❌`        |

**Key Features:**

- **Single-line output**: Perfect for status bars and monitoring systems
- **Emoji-only status**: Universal visual language requiring no text parsing
- **Silent operation**: All cargo output is suppressed, only status emoji appears
- **Directory-independent**: Scripts work from anywhere by changing to project directory
- **Fast execution**: Optimized for quick status checks without verbose output

**Usage Examples:**

```sh
# Quick test status check
./test-status-one-line.bash
# Output: " 🧪✔️"

# Quick documentation build check
./doc-status-one-line.bash
# Output: " 📚✔️"
```

**Integration with Development Workflow:**

- **Complements Bacon**: While bacon provides rich interactive feedback, these scripts offer minimal
  monitoring
- **CI/CD friendly**: Perfect for automated pipelines requiring simple pass/fail status
- **GNOME integration**: Designed specifically for desktop environment status bar integration
- **Background monitoring**: Ideal for continuous status monitoring without interrupting workflow

These scripts provide the same underlying functionality as the bacon workflows but with radically
different output designed for external consumption rather than developer interaction.

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

### Wild Linker (Linux)

This project uses the [Wild linker](https://github.com/davidlattimore/wild) as a fast alternative to
the default linker on Linux systems. Wild can significantly reduce link times during iterative
development, making builds faster and more responsive.

**Automatic Configuration**: The build system automatically detects and configures Wild when both
`clang` and `wild` are installed. If either tool is missing, the configuration gracefully falls back
to standard parallel compilation without Wild.

**Installation**: The setup process automatically installs both prerequisites:

- `clang`: Installed by [`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh) as a system dependency
- `wild-linker`: Installed by `fish run.fish install-cargo-tools` via `cargo-binstall` (with fallback to `cargo install`)

**Configuration**: When available, Wild is configured in `.cargo/config.toml` for Linux targets:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = [
    "-Z", "threads=8",  # Parallel compilation
    "-C", "link-arg=--ld-path=wild"  # Wild linker
]
```

**Verification**: Check if Wild is active by looking for the configuration in `.cargo/config.toml`
or by observing faster link times during development builds.

**Platform Support**: Wild linker is Linux-only. On other platforms, the build system uses standard
parallel compilation without Wild.

### Rust Toolchain Management

This project includes three complementary scripts for comprehensive Rust toolchain management, each
serving a specific purpose in the development workflow:

#### 1. rust-toolchain-update.fish - Smart Validated Toolchain Updates

Intelligently finds and validates a stable nightly toolchain, preferring older versions for
stability while ensuring they don't have ICE (Internal Compiler Error) bugs.

```sh
# Via run.fish command
fish run.fish update-toolchain

# Or directly
./rust-toolchain-update.fish
```

**What it does:**

- **Smart search**: Tests nightly toolchains starting from 45 days ago, moving forward day-by-day
  until finding a stable one (up to today)
- **ICE validation**: Runs comprehensive validation suite on each candidate:
  - `cargo clippy --all-targets`
  - `cargo build`
  - `cargo nextest run`
  - `cargo test --doc`
  - `cargo doc --no-deps`
- **Toolchain vs code errors**: Distinguishes between:
  - ❌ **ICE errors** (compiler crashes) → rejects toolchain, tries next day
  - ✅ **Code errors** (compilation/test failures) → accepts toolchain (validates compiler works,
    not your code)
- **First stable wins**: Stops at the first toolchain without ICE errors (usually finds stable
  toolchain in first attempt)
- **Updates** `rust-toolchain.toml` to use the validated stable nightly
- Installs the target toolchain with rust-analyzer component (required by IDEs, cargo, and serena
  MCP server)
- **Desktop notifications** (via notify-send):
  - 🎉 Success notification when stable toolchain found (normal urgency)
  - 🚨 Critical alert if no stable toolchain found in entire 45-day window (extremely rare)
- Performs aggressive cleanup by removing all old nightly toolchains except:
  - All stable toolchains (`stable-*`)
  - The newly validated nightly
- **Final verification with fresh build**:
  - Removes ICE failure files (`rustc-ice-*.txt`) generated during validation
  - Cleans all caches: cargo cache, build artifacts, sccache
  - Runs full verification: tests, doctests, and documentation build
  - Ensures new toolchain works perfectly from scratch
- Logs all operations to `/home/nazmul/Downloads/rust-toolchain-update.log`

**When to use:**

- Weekly maintenance (can be automated via systemd timer)
- When you want to update to a validated stable nightly
- When you want to clean up old toolchains
- After encountering ICE errors with current toolchain

**Example output:**

```text
═══════════════════════════════════════════════════════
Starting search for stable toolchain
Strategy: Start 45 days ago, try progressively newer up to today
Search window: 2025-08-29 to 2025-10-13
═══════════════════════════════════════════════════════

Attempt 1/46
Trying toolchain: nightly-2025-08-29 (45 days ago)

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Validating toolchain: nightly-2025-08-29
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Running validation step: clippy
  ⚠️  Command exited with code 101 (this is OK if not ICE)
  ✅ No ICE detected - continuing validation
...
✅ Toolchain nightly-2025-08-29 is STABLE (no ICE detected)

🎉 FOUND STABLE TOOLCHAIN: nightly-2025-08-29
Success notification sent

✅ Successfully updated rust-toolchain.toml
✅ Successfully installed rust-analyzer component
Removed 2 old toolchain(s)
Toolchains directory size before cleanup: 5.3G
Toolchains directory size after cleanup: 2.6G
```

#### 2. rust-toolchain-sync-to-toml.fish - Sync to Existing Config

Syncs your Rust environment to match whatever is specified in `rust-toolchain.toml`.

```sh
./rust-toolchain-sync-to-toml.fish
```

**What it does:**

- **Reads** the channel value from `rust-toolchain.toml` (doesn't modify it)
- Installs the exact toolchain specified in the TOML
- Installs rust-analyzer and rust-src components automatically (required by IDEs, cargo, and serena
  MCP server)
- Performs aggressive cleanup by removing all old nightly toolchains except:
  - All stable toolchains (`stable-*`)
  - The target toolchain from the TOML
- Logs all operations to `/home/nazmul/Downloads/rust-toolchain-sync-to-toml.log`

**When to use:**

- After `git checkout/reset/pull` changes `rust-toolchain.toml`
- When rust-analyzer is missing for the current toolchain
- When your IDE shows "rust-analyzer failed to start"
- When Claude Code's serena MCP server crashes with LSP initialization errors
- After manually editing `rust-toolchain.toml`
- When you need to stay on a specific nightly version

**Key difference from update script:**

- **This script (sync)**: Respects TOML → Installs what's specified
- **Update script**: Modifies TOML → Installs "1 month ago" nightly

**Example workflow:**

```sh
# Weekly script updates TOML to nightly-2025-09-11
# But you need to stay on nightly-2025-09-05 for testing a specific feature
git checkout rust-toolchain.toml  # Revert to 09-05
./rust-toolchain-sync-to-toml.fish  # Install components for 09-05
# Now rust-analyzer works for 09-05
```

#### 3. remove_toolchains.sh - Testing Utility

Removes ALL Rust toolchains for testing upgrade progress display (⚠️ DESTRUCTIVE).

```sh
./remove_toolchains.sh
```

**What it does:**

- Removes ALL Rust toolchains from your system
- Cleans up toolchain directories completely
- Creates a clean slate for testing rustup installation progress

**When to use:**

- When developing/testing the upgrade progress display in `edi` and `giti`
- To see full rustup download and installation progress
- For testing `cmdr/src/analytics_client/upgrade_check.rs` functionality

**Recovery after testing:**

```sh
rustup toolchain install stable && rustup default stable
# Or
fish run.fish update-toolchain
```

**⚠️ Warning:** This is a destructive testing utility. Use only when you understand the implications
and are prepared to reinstall toolchains.

#### Toolchain Management Benefits

**Stability**: Month-old nightlies have proven stability while providing recent features **Disk
space savings**: Aggressive cleanup removes accumulated old toolchains **Consistency**: All
developers use the same Rust version via `rust-toolchain.toml` **Automation ready**: `update` script
designed to run weekly via systemd timer **Recovery ready**: `sync` script fixes environment after
git operations **Testing support**: `remove` script enables testing upgrade workflows

### Unified Script Architecture

The project uses a clean separation of concerns across three main scripts:

**[`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh)** - **OS-Level Setup**
- System package manager detection and OS dependencies
- Rust toolchain installation via rustup
- Development environment setup (Fish shell, fzf, file watchers)
- Cross-platform compatibility (Linux, macOS)
- Calls run.fish for Rust-specific tooling

**[`run.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/run.fish)** - **Rust Development Commands**
- **Workspace-wide commands** that operate on the entire project
- **Cargo tool installation** (install-cargo-tools with cargo-binstall, uv, bacon, nextest, etc.)
- **TUI-specific commands** for running examples and benchmarks
- **cmdr-specific commands** for binary management
- **Cross-platform file watching** using inotifywait (Linux) or fswatch (macOS)
- **Smart log monitoring** that detects and manages log files from different workspaces

**[`script_lib.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/script_lib.fish)** - **Shared Utilities**
- Common functions used by both bootstrap.sh and run.fish
- Utility functions: install_if_missing, generate_cargo_config, install_cargo_tool
- Cross-platform package manager detection

All commands work from the root directory, eliminating the need to navigate between subdirectories. This architecture ensures no redundancy - each tool is installed in exactly one place with clear ownership.

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
