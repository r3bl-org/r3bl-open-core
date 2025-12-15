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
  - [Task File Format](#task-file-format)
  - [Task Workflow Commands](#task-workflow-commands)
  - [Workflow Connection](#workflow-connection)
  - [Development Tools Integration](#development-tools-integration)
- [Documentation and Planning](#documentation-and-planning)
  - [Release and Contribution Guides](#release-and-contribution-guides)
  - [Technical Design Documents](#technical-design-documents)
- [Learn how these crates are built, provide feedback](#learn-how-these-crates-are-built-provide-feedback)
- [Quick Start](#quick-start)
  - [Automated Setup (Recommended)](#automated-setup-recommended)
  - [Manual Setup](#manual-setup)
- [IDE Setup and Extensions](#ide-setup-and-extensions)
  - [R3BL IntelliJ Plugins](#r3bl-intellij-plugins)
  - [R3BL VSCode Extensions](#r3bl-vscode-extensions)
- [Build the workspace and run tests](#build-the-workspace-and-run-tests)
  - [Key Commands](#key-commands)
  - [Cargo Target Directory Isolation for IDE/Tool Performance](#cargo-target-directory-isolation-for-idetool-performance)
    - [The Problem: Cargo Lock Contention](#the-problem-cargo-lock-contention)
    - [The Solution: Separate Build Artifacts](#the-solution-separate-build-artifacts)
    - [Configuration by Tool](#configuration-by-tool)
    - [Benefits](#benefits)
    - [Example Workflow Setup](#example-workflow-setup)
    - [Disk Space Management](#disk-space-management)
    - [Troubleshooting](#troubleshooting)
    - [Incremental Compilation Management](#incremental-compilation-management)
  - [Bacon Development Tools](#bacon-development-tools)
  - [Automated Development Monitoring](#automated-development-monitoring)
    - [Option 1: Lightweight Watch Mode (Recommended for Most Users)](#option-1-lightweight-watch-mode-recommended-for-most-users)
    - [Option 2: Comprehensive Tmux Dashboard](#option-2-comprehensive-tmux-dashboard)
  - [Tmux Development Dashboard](#tmux-development-dashboard)
  - [Status Monitoring Scripts](#status-monitoring-scripts)
  - [Wild Linker (Linux)](#wild-linker-linux)
  - [Cross-Platform Verification (Windows)](#cross-platform-verification-windows)
  - [Rust Toolchain Management](#rust-toolchain-management)
    - [Why mkdir for Locking?](#why-mkdir-for-locking)
    - [1. rust-toolchain-update.fish - Smart Validated Toolchain Updates](#1-rust-toolchain-updatefish---smart-validated-toolchain-updates)
    - [2. rust-toolchain-sync-to-toml.fish - Sync to Existing Config](#2-rust-toolchain-sync-to-tomlfish---sync-to-existing-config)
    - [3. rust-toolchain-validate.fish - Unified Toolchain Validation](#3-rust-toolchain-validatefish---unified-toolchain-validation)
    - [4. remove_toolchains.sh - Testing Utility](#4-remove_toolchainssh---testing-utility)
    - [Log File Output](#log-file-output)
    - [Comprehensive Toolchain Management System](#comprehensive-toolchain-management-system)
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

This project uses a two-tier task management system for organizing day-to-day development work:
lightweight pointers with simple tasks in root-level files, and detailed task files with
implementation plans in the `./task/` directory.

### Task Management Files

- **[`todo.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/todo.md)** - Active tasks,
  immediate priorities, and pointers to detailed task files. Latest changes at top. Uses status
  markers: `[ ]` (pending), `[⌛]` (in progress), `[x]` (completed)
- **[`done.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/done.md)** - Completed tasks
  and achievements, providing a historical record of progress. Links to archived task files in
  `./task/done/`
- **[`./task/`](https://github.com/r3bl-org/r3bl-open-core/tree/main/task)** - Directory containing
  detailed task management files:
  - **Active tasks**: `task_*.md` files in root of `./task/` - Complex tasks currently in progress
  - **`pending/`**: Tasks queued for later work
  - **`done/`**: Completed task files moved from root after all steps are marked `[COMPLETE]`
  - **`archive/`**: Abandoned tasks retained for historical reference
  - **`CLAUDE.md`**: Rules and format specifications for creating and maintaining task files

### Task File Format

Detailed task files follow a structured format defined in
[`./task/CLAUDE.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/task/CLAUDE.md):

**Structure:**

```markdown
# Task Overview

High-level description, architecture, context, and the "why"

# Implementation Plan

## Step 0: Do Something [STATUS]

Detailed instructions for this step

### Step 0.0: Do Subtask [STATUS]

Details about subtask

### Step 0.1: Do Another Subtask [STATUS]

Details about another subtask

## Step 1: Do Something Else [STATUS]

More detailed steps...
```

**Hierarchical organization:**

- Steps are numbered (Step 0, Step 1, Step 2, etc.)
- Substeps use decimal notation (Step 0.0, Step 0.1, etc.)
- Table of contents automatically generated and maintained using `doctoc`
- Formatting standardized with `prettier`

**Status markers:**

- `[COMPLETE]` - Step finished and verified
- `[WORK_IN_PROGRESS]` - Currently working on this step
- `[BLOCKED]` - Cannot proceed (waiting for dependency)
- `[DEFERRED]` - Postponed to later

### Task Workflow Commands

The `/task` slash command (defined in
[`.claude/commands/task.md`](https://github.com/r3bl-org/r3bl-open-core/blob/main/.claude/commands/task.md))
manages the task lifecycle:

**Create a new task:**

```sh
/task create my_feature_name
```

- Creates `./task/task_my_feature_name.md` from your detailed plan
- Use after you have a comprehensive plan in your todo list
- Initializes structure with steps and status markers

**Update an existing task:**

```sh
/task update my_feature_name
```

- Updates progress markers in `./task/task_my_feature_name.md`
- Moves completed task files to `./task/done/` when all steps are `[COMPLETE]`
- Updates `todo.md` and `done.md` cross-references as needed

**Resume working on a task:**

```sh
/task load my_feature_name
```

- Loads `./task/task_my_feature_name.md` for continued work
- Resumes from the last step marked `[WORK_IN_PROGRESS]`
- If none found, asks which incomplete step to start with

### Workflow Connection

The task organization workflow connects strategic planning with tactical execution:

- **Strategic Planning** (`docs/` folder): Feature roadmaps, architectural decisions, design
  documents
- **Planning to Active Work**: Complex features are documented in `docs/` first, then planned into
  `todo.md`
- **Tactical Execution**:
  1. Simple tasks stay in `todo.md` as checklist items
  2. Complex tasks get detailed planning → `/task create` → `./task/task_*.md`
  3. Work progresses through hierarchical steps with `/task update` marking progress
  4. Completion → Task moved to `./task/done/` via `/task update`
  5. `done.md` maintains archive links for historical reference
- **Continuous Sync**: `todo.md` is synchronized with the GitHub project dashboard for visibility
  across team members

This three-level approach (docs → todo.md → ./task/) ensures strategic planning, tactical planning,
and detailed execution are well-organized and connected.

### Development Tools Integration

R3BL provides IDE extensions and plugins to enhance your development workflow, regardless of your
editor choice:

**For VSCode Users**

R3BL provides custom VSCode extensions including Task Spaces (organize editor tabs by context),
theme, and enhanced syntax highlighting. See the [R3BL VSCode Extensions](#r3bl-vscode-extensions)
section below for installation and detailed feature descriptions.

**For IntelliJ IDEA Users**

R3BL provides theme and productivity plugins for IntelliJ IDEA and other JetBrains IDEs. See the
[R3BL IntelliJ Plugins](#r3bl-intellij-plugins) section below for installation from the JetBrains
Marketplace and detailed feature descriptions.

**Workflow Integration:**

Both IDE environments complement the `./task/` file management system in this project:

- **VSCode**: The R3BL Task Spaces extension helps you organize editor tabs by context (e.g., one
  space for features, one for docs, one for debugging) while the `./task/` files track your
  implementation progress
- **RustRover**: Use the built-in Task Management plugin alongside `./task/` files for seamless
  workflow integration

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

The [`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh) script
handles **OS-level setup** with a clean main function structure and will:

- **System Package Manager Detection**: Automatically detects apt, dnf, pacman, zypper, or brew
- **Core Rust Installation**: Install Rust toolchain (rustup) and ensure cargo is in PATH
- **Compiler Setup**: Install clang compiler (required by Wild linker)
- **Development Shell**: Install Fish shell and fzf for interactive development
- **File Watching**: Install file watchers (inotifywait on Linux, fswatch on macOS)
- **Development Utilities**: Install htop, screen, tmux for system monitoring
- **Node.js Ecosystem**: Install Node.js and npm for web tooling
- **AI Integration**: Install Claude Code CLI with MCP server configuration
- **Rust Development Tools Setup**: Call `fish run.fish install-cargo-tools` for all Rust-specific
  tooling

**Architecture**: Uses clear function separation with main() orchestrator and dedicated functions
for each concern (install_rustup, install_clang, install_shell_tools, etc.)

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

**Note**: The manual approach requires you to install OS-level dependencies yourself. The
`install-cargo-tools` command focuses specifically on **Rust development tools**:

**From crates.io (via cargo-binstall with fallback to cargo install):**

- **cargo-binstall**: Fast binary installer (installed first as foundation)
- **uv**: Modern Python package manager (required for Serena semantic code MCP server)
- **Core Development Tools**: bacon, flamegraph, inferno
- **Workspace Management**: cargo-workspaces, cargo-cache, cargo-update
- **Code Quality**: cargo-deny, cargo-unmaintained, cargo-expand, cargo-readme
- **Wild Linker**: Fast linker with optimized .cargo/config.toml generation
- **Language Server**: rust-analyzer component

**From local source (via cargo install --path):**

- **cmdr**: edi, giti, rc binaries (calls `install-cmdr`)
- **build-infra**: cargo-rustdoc-fmt (calls `install-build-infra`)

**Features:**

- **Smart Installation**: Uses cargo-binstall for speed with fallback to cargo install --locked
- **Local Source Rebuild**: Always rebuilds cmdr and build-infra from source with current toolchain
- **Shared Utilities**: Leverages utility functions from script_lib.fish for consistency

## IDE Setup and Extensions

Choose the development environment that works best for you. R3BL provides extensions and plugins for
both VSCode and IntelliJ IDEA.

### R3BL IntelliJ Plugins

For developers using IntelliJ IDEA, RustRover, or other JetBrains IDEs, install the R3BL plugins
directly from the JetBrains Marketplace:

**Available Plugins:**

- **[R3BL Theme](https://plugins.jetbrains.com/plugin/28943-r3bl-theme/)** - Vibrant dark theme with
  carefully chosen colors for visual clarity and reduced eye strain. Optimized for Rust, Markdown,
  and 30+ languages.
- **[R3BL Copy Selection Path](https://plugins.jetbrains.com/plugin/28944-r3bl-copy-selection-path-and-range/)** -
  Copy file paths with selected line ranges in Claude Code compatible format. Press `Alt+O` to copy
  the current file path with line numbers.

**Installation from JetBrains Marketplace:**

1. Open IntelliJ IDEA / RustRover
2. Go to `Settings` → `Plugins` → `Marketplace`
3. Search for "R3BL Theme" and "R3BL Copy Selection Path"
4. Click `Install` on each plugin
5. Restart the IDE

**Or install from disk (for latest development builds):**

```sh
# Clone the plugins repository
git clone https://github.com/r3bl-org/r3bl-intellij-plugins.git
cd r3bl-intellij-plugins

# Build the plugins
./gradlew buildPlugin

# In IntelliJ: Settings → Plugins → ⚙️ → Install Plugin from Disk
# Select the .zip files from:
# - plugins/r3bl-theme/build/distributions/r3bl-theme-*.zip
# - plugins/r3bl-copy-selection-path/build/distributions/r3bl-copy-selection-path-*.zip
```

**Benefits for r3bl-open-core development:**

- **Vibrant Color Scheme**: Enhanced syntax highlighting makes Rust code more readable
- **Claude Code Integration**: Quickly copy file paths with line ranges using `Alt+O` to share code
  references with Claude Code
- **Reduced Eye Strain**: Carefully balanced colors optimized for long coding sessions
- **Multi-Language Support**: Works great with Rust, Markdown, TOML, and all file types in this
  project

**Post-installation:**

1. Restart IntelliJ IDEA / RustRover
2. Go to `Settings` → `Appearance & Behavior` → `Appearance` → `Theme` → Select "R3BL"
3. Use `Alt+O` to copy file paths with line ranges (great for Claude Code interactions!)

**Task Workflow Integration:**

IntelliJ IDEA and RustRover include a built-in Task Management plugin that works seamlessly
alongside the `./task/` file management system in this project. Use it to organize your work
contexts while the `./task/` files track your implementation progress.

### R3BL VSCode Extensions

For an optimal development experience with r3bl-open-core in VSCode, we provide a custom extension
pack specifically designed for Rust development. This extension pack is not available on the VSCode
marketplace and must be installed manually.

**What's included:**

- **Task Spaces** - Organize and switch between collections of editor tabs for different work
  contexts (e.g., one space for editing features, one for writing documentation, one for debugging).
  Complements the `./task/` file management system by helping you organize your editor sessions.
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

Both the IntelliJ plugins and VSCode extensions work seamlessly with the existing development tools
mentioned in this guide, including rust-analyzer, bacon, and the comprehensive development workflow.

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
    rustdoc-fmt          Format rustdoc comments
    install-cargo-tools  Install all dev tools (crates.io + local source)
    upgrade-deps         Upgrade dependencies
    update-cargo-tools   Update all tools (crates.io + rebuild local source)
    audit-deps           Security audit
    unmaintained-deps    Check for unmaintained deps
    toolchain-update     Update Rust to month-old nightly
    toolchain-sync       Sync environment to rust-toolchain.toml
    toolchain-validate   Quick toolchain validation (components only)
    toolchain-validate-complete  Complete toolchain validation (full build+test)
    toolchain-remove     Remove ALL toolchains (testing)
    build-server         Remote build server - uses rsync

Watch commands:
    test-watch [pattern]  Watch files, run specific test
    clippy-watch          Watch files, run clippy
    check-watch           Watch files, run cargo check
    check-full-watch      Watch files, run all checks (tests, doctests, docs)
    check-full-watch-test Watch files, run tests and doctests
    check-full-watch-doc  Watch files, run doc build only

TUI-specific commands:
    run-examples [--release] [--no-log]  Run TUI examples
    run-examples-flamegraph-svg  Generate SVG flamegraph
    run-examples-flamegraph-fold [--benchmark]  Generate perf-folded (use --benchmark for reproducible profiling)
    bench                Run benchmarks

Local source package commands:
    install-cmdr         Install cmdr binaries (edi, giti, rc) from source
    install-build-infra  Install build-infra tools (cargo-rustdoc-fmt) from source

cmdr-specific commands:
    run-binaries         Run edi, giti, or rc
    docker-build         Build release in Docker

Development Session Commands:
    dev-dashboard        Start 2-pane tmux development dashboard

Other commands:
    log                  Monitor log.txt in cmdr or tui directory
    help                 Show this help
```

### Key Commands

| Command                                                    | Description                                                                             |
| ---------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| `fish run.fish all`                                        | Run all major checks (build, test, clippy, docs, audit, format)                         |
| `fish run.fish build`                                      | Build the entire workspace                                                              |
| `fish run.fish test`                                       | Run all tests across the workspace                                                      |
| `fish run.fish install-cargo-tools`                        | Install all dev tools (crates.io + local source packages)                               |
| `fish run.fish update-cargo-tools`                         | Update all tools (crates.io + rebuild local source packages)                            |
| `fish run.fish install-cmdr`                               | Install cmdr binaries (edi, giti, rc) from source                                       |
| `fish run.fish install-build-infra`                        | Install build-infra tools (cargo-rustdoc-fmt) from source                               |
| `fish run.fish test-watch [pattern]`                       | Watch for file changes and run specific test                                            |
| `fish run.fish run-examples`                               | Run TUI examples interactively                                                          |
| `fish run.fish run-examples-flamegraph-svg`                | Generate SVG flamegraph for performance analysis                                        |
| `fish run.fish run-examples-flamegraph-fold [--benchmark]` | Generate perf-folded format for analysis (use `--benchmark` for reproducible profiling) |
| `fish run.fish bench`                                      | Run benchmarks                                                                          |
| `fish run.fish run-binaries`                               | Run cmdr binaries (edi, giti, rc) interactively                                         |
| `fish run.fish dev-dashboard`                              | Start 2-pane tmux development dashboard (tests, docs, checks)                           |
| `fish run.fish check-full`                                 | Run comprehensive checks (tests, doctests, docs, toolchain validation)                  |
| `fish run.fish check-windows-build`                        | Verify Windows cross-compilation (platform cfg gates)                                   |
| `fish run.fish toolchain-validate`                         | Quick toolchain validation (components only, ~1-2 seconds)                              |
| `fish run.fish toolchain-validate-complete`                | Complete toolchain validation (full build+test, ~5-10 minutes)                          |
| `fish run.fish toolchain-update`                           | Update Rust to month-old nightly toolchain with cleanup                                 |
| `fish run.fish toolchain-sync`                             | Sync Rust environment to match rust-toolchain.toml                                      |
| `fish run.fish toolchain-remove`                           | Remove ALL toolchains (⚠️ destructive testing utility)                                  |

> **TUI Testing**: The `r3bl_tui` crate uses PTY-based testing for accurate terminal I/O
> verification. See the [PTY Testing Infrastructure](./tui/README.md#pty-testing-infrastructure)
> section in the TUI README for details on writing and running TUI tests.

### Cargo Target Directory Isolation for IDE/Tool Performance

**Critical Optimization**: When multiple development tools run cargo simultaneously (IDE, terminal,
file watcher, CI), they compete for locks on the shared `target/` directory. This causes severe
responsiveness issues as each tool waits for others to complete. Isolating build artifacts by tool
eliminates this bottleneck completely.

#### The Problem: Cargo Lock Contention

When you have multiple `cargo` instances running:

- **VSCode rust-analyzer**: Runs `cargo check` continuously in background
- **RustRover**: Runs `cargo check` continuously in background
- **File watcher** (`check.fish`, `bacon`): Triggers cargo tests, doc builds, etc. on every file
  save
- **Terminal**: You run manual `cargo` commands, and `Claude Code` is running commands

All these access the same `target/` directory:

```
target/
├── debug/
├── release/
└── .rustc_info.json  # ← Lock contention here
```

When one tool locks `target/`, all others wait. This cascades into a "traffic jam" where everything
becomes unresponsive.

#### The Solution: Separate Build Artifacts

Configure each tool to use its own target directory. Rust supports this via the `CARGO_TARGET_DIR`
environment variable:

```
target/
├── vscode/      # VSCode rust-analyzer builds
├── rustrover/   # JetBrains IDE builds
├── claude/      # Claude Code builds
├── check/       # check.fish file watcher builds
└── cli/         # Terminal manual builds (optional)
```

Now tools build in parallel without interfering with each other.

#### Configuration by Tool

Generally speaking you can just add `CARGO_TARGET_DIR=target/XYZ` in the command, for eg you can run
the following in your terminal to run `claude` with the `CARGO_TARGET_DIR` env var set, and all the
`cargo` commands spawned by `claude` will have their own taret directory to work with:

```bash
CARGO_TARGET_DIR=target/claude $argv
```

You can add this to an alias, add it to scripts (like `check.fish` does via
`set -gx CARGO_TARGET_DIR target/check`) or you can configure settings in your tool of choice.

In VSCode, you can add the following to `.vscode/settings.json`:

```json
{
  "rust-analyzer.cargo.targetDir": true
}
```

In RustRover, you can go to "Settings -> Rust -> Environment Variables" and add this
`CARGO_TARGET_DIR=target/rustrover`

#### Benefits

| Benefit              | Impact                                                                      |
| -------------------- | --------------------------------------------------------------------------- |
| **Zero Contention**  | Tools run in parallel without waiting on locks                              |
| **Responsive IDE**   | rust-analyzer completes checks while you code (not blocked by file watcher) |
| **Faster Feedback**  | Terminal cargo commands complete instantly (not queued behind IDE checks)   |
| **Parallel Testing** | bacon + check.fish both run, providing redundant test feedback              |
| **Disk Space**       | ~2-3GB per tool (manageable with cleanup)                                   |

#### Example Workflow Setup

Here's a typical productive development workflow setup:

```bash
# Terminal 1: Running your IDE (VSCode with rust-analyzer)
CARGO_TARGET_DIR=target/vscode code .

# Terminal 2: File watcher with automatic tests
check.fish --watch-tests # Runs with: CARGO_TARGET_DIR=target/check

# Terminal 3: Run claude code
CARGO_TARGET_DIR=target/claude claude

# Terminal 4: Run bacon
CARGO_TARGET_DIR=target/bacon bacon doc --headless

# Result: All three run in parallel, zero blocking
```

Before this optimization, Terminal 3 would hang waiting for Terminals 1-2 to release the `target/`
lock.

#### Disk Space Management

Each tool caches ~2-3GB of build artifacts. With 4 tools, expect ~10-12GB total. To manage:

```bash
# View size of each target directory
du -sh target/*/

# Clean individual tool builds
rm -rf target/vscode
rm -rf target/rustrover
rm -rf target/claude
rm -rf target/check

# Full cleanup (nuclear option)
rm -rf target/
```

#### Troubleshooting

**Syntax errors still appear in IDE but code works in terminal?**

Your IDE and terminal are using different target directories. Verify `CARGO_TARGET_DIR`
configuration:

```bash
# Check what each tool sees
echo $CARGO_TARGET_DIR  # Terminal value
# VSCode: Check .vscode/settings.json
# RustRover: Check IDE settings
```

**Build artifacts aren't being reused across tools?**

Each tool has its own `target/` directory by design. This is correct - the slight disk space
overhead is worth the responsiveness gain. If you need to share builds, unset `CARGO_TARGET_DIR`
(not recommended for development).

**"Target directory not found" error?**

Cargo automatically creates the directory. If you see this error, verify the path is writable and
the environment variable is set correctly:

```bash
# Verify the variable is actually set
env | grep CARGO_TARGET_DIR

# Test with explicit path
CARGO_TARGET_DIR=/tmp/test cargo build
```

#### Incremental Compilation Management

Incremental compilation is disabled globally (`incremental = false` in `.cargo/config.toml`) to
avoid issues with the rustc dependency graph on nightly builds:

```toml
# .cargo/config.toml
[build]
incremental = false  # Disable to avoid rustc dep graph ICE on nightly
```

**Why disable incremental compilation?**

- The nightly compiler has occasional bugs with the dependency graph in incremental mode
- These bugs can cause Internal Compiler Errors (ICE) like "mir_drops_elaborated_and_const_checked"
- Disabling it globally ensures stable builds across all cargo invocations
- The performance impact is acceptable for development workflows

**If you encounter ICE errors anyway:**

```bash
# Clear any corrupted incremental artifacts
rm -rf target/check target/debug target/release

# Rebuild cleanly
cargo check  # or cargo build, cargo test, etc.
```

The `check.fish` script also explicitly sets `CARGO_INCREMENTAL=0` as a redundant safeguard.

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

| Command                           | Workflow    | Description                                                              |
| --------------------------------- | ----------- | ------------------------------------------------------------------------ |
| `bacon test`                      | Interactive | Run all tests with cargo test (includes unit, integration, and doctests) |
| `bacon test -- <pattern>`         | Interactive | Run specific test matching pattern                                       |
| `bacon doctests`                  | Interactive | Run only documentation tests (`cargo test --doc`)                        |
| `bacon test --headless --summary` | Background  | Silent test runner providing only pass/fail status                       |

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

- Use `bacon test` to run all tests (includes unit, integration, and doctests)
- Use `bacon doctests` or `bacon test --doc` to run only documentation tests

### Automated Development Monitoring

The project provides two complementary approaches for continuous monitoring during development -
choose based on your workflow preferences:

#### Option 1: Lightweight Watch Mode (Recommended for Most Users)

For developers who want automated monitoring without the overhead of tmux, use the standalone check
script:

```sh
./check.fish --watch
```

**What it does:**

- **Monitors source directories**: Watches `cmdr/src/`, `analytics_schema/src/`, and `tui/src/` for
  changes
- **Event-driven execution**: Triggers immediately on file changes (no polling delay)
- **Intelligent debouncing**: 5-second delay prevents rapid re-runs during saves
- **Comprehensive checks**: Runs tests, doctests, and doc builds automatically
- **Clean progress output**: Shows stage-by-stage progress without verbose cargo logs
- **Automatic toolchain validation**: Validates and repairs Rust toolchain before checks
- **ICE recovery**: Detects and recovers from Internal Compiler Errors automatically
- **Continuous operation**: Keeps watching even if checks fail (perfect for iterative development)

**Example output:**

```
👀 Watch mode activated
Monitoring: cmdr/src, analytics_schema/src, tui/src
Press Ctrl+C to stop

🔄 Changes detected, running checks...

▶️  Running tests...
✅ Tests passed

▶️  Running doctests...
✅ Doctests passed

▶️  Building docs...
✅ Docs built

✅ All checks passed!

👀 Watching for changes...
```

**Benefits:**

- **Single window**: No tmux complexity - just one terminal
- **Immediate feedback**: 2-second response time after file saves
- **Low overhead**: Minimal resource usage compared to running multiple monitors
- **Perfect for focus**: Clean output doesn't distract from your editor

**Event handling:** While checks run (30+ seconds), the Linux kernel buffers new file change events.
When checks complete, buffered events trigger immediately if debounce allows. This ensures no
changes are lost but may cause cascading re-runs if you save multiple times during test execution.
Adjust `DEBOUNCE_SECONDS` in the script if needed.

**Usage:**

```sh
# Show available options
./check.fish --help

# Start watch mode
./check.fish --watch

# Or run checks once (manual mode)
./check.fish
```

#### Option 2: Comprehensive Tmux Dashboard

### Tmux Development Dashboard

For developers who prefer a multi-pane visual environment, the tmux dashboard combines multiple
bacon monitors with the `check.fish --watch` script for comprehensive coverage.

**When to choose tmux dashboard over standalone watch mode:**

- You want to see **all** checks running simultaneously in different panes
- You prefer visual separation between tests, doctests, docs, and comprehensive checks
- You're comfortable with tmux keybindings and pane navigation
- You have screen space for a 2x2 grid layout

**Comprehensive 4-Pane Development Dashboard:**

```
┌─────────────────────────────────────────────────────────────┐
│ Tmux Session: r3bl-dev (2x2 grid layout)                    │
├──────────────────────┬──────────────────────────────────────┤
│ Top-left:            │ Top-right:                           │
│ bacon test           │ bacon doc                            │
│ (Unit & Integration  │ (Documentation generation            │
│  Tests)              │  with live feedback)                 │
├──────────────────────┼──────────────────────────────────────┤
│ Bottom-left:         │ Bottom-right:                        │
│ bacon doctests       │ ./check.fish --watch                 │
│ (Documentation       │ (Event-driven comprehensive checks:  │
│  Tests)              │  tests + doctests + docs + ICE)      │
└──────────────────────┴──────────────────────────────────────┘
```

**Key Features:**

- **Persistent Session**: Session name "r3bl-dev" - reconnect from other terminals with
  `tmux attach-session -t r3bl-dev`
- **Multiple Monitors**: Combines three bacon monitors (tests, doctests, docs) with one
  comprehensive check monitor (`check.fish --watch`)
- **Event-Driven Checks**: The bottom-right pane runs `./check.fish --watch` which triggers
  immediately on file changes (not periodic polling)
- **Comprehensive Coverage**: The `check.fish --watch` monitor provides:
  - All unit and integration tests (`cargo test --all-targets`)
  - Documentation tests (`cargo test --doc`)
  - Documentation building (`cargo doc --no-deps`)
  - Automatic ICE (Internal Compiler Error) detection and recovery
  - Automatic toolchain validation and repair if needed
  - 5-second intelligent debouncing to prevent rapid re-runs
- **Interactive Multiplexing**: Full tmux keybindings for pane switching and layout customization
- **Redundant Coverage**: Tests run in two panes (bacon test + check.fish) - if one fails, the other
  shows details

**Usage:**

```sh
# Start the development dashboard
fish run.fish dev-dashboard

# Reconnect to existing session from another terminal
tmux attach-session -t r3bl-dev

# Kill the session when done
tmux kill-session -t r3bl-dev
```

**Comparison: Standalone vs Tmux Dashboard:**

| Aspect                 | `./check.fish --watch`              | Tmux Dashboard                             |
| ---------------------- | ----------------------------------- | ------------------------------------------ |
| **Setup Complexity**   | Single command, one window          | tmux session with 4 panes                  |
| **Screen Real Estate** | Minimal (one terminal)              | Large (2x2 grid)                           |
| **Monitoring Scope**   | Comprehensive (tests+docs+doctests) | Granular (separate panes for each)         |
| **Visual Separation**  | Sequential output in one stream     | Parallel output in dedicated panes         |
| **Ideal For**          | Focused development, laptop screens | Multi-monitor setups, visual dashboards    |
| **Tmux Knowledge**     | Not required                        | Helpful for navigation                     |
| **Resource Usage**     | Lower (one monitor)                 | Higher (4 monitors)                        |
| **Event-Driven**       | Yes (file system events)            | Yes (check.fish pane) + bacon auto-rebuild |

**When to use each:**

- **Use standalone watch**: When you want simple, focused monitoring in a single terminal
- **Use tmux dashboard**: When you want comprehensive visibility with separate panes for each
  concern

Both approaches use the same `check.fish --watch` script in different contexts - standalone for
simplicity, integrated for comprehensive dashboards.

**Typical Development Session:**

1. Start session: `fish run.fish dev-dashboard`
2. Monitor panes to catch issues while coding
3. Switch to specific pane for detailed investigation if needed
4. All four monitors provide continuous feedback on code quality

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

### Wild Linker (Linux)

This project uses the [Wild linker](https://github.com/davidlattimore/wild) as a fast alternative to
the default linker on Linux systems. Wild can significantly reduce link times during iterative
development, making builds faster and more responsive.

**Automatic Configuration**: The build system automatically detects and configures Wild when both
`clang` and `wild` are installed. If either tool is missing, the configuration gracefully falls back
to standard parallel compilation without Wild.

**Installation**: The setup process automatically installs both prerequisites:

- `clang`: Installed by
  [`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh) as a system
  dependency
- `wild-linker`: Installed by `fish run.fish install-cargo-tools` via `cargo-binstall` (with
  fallback to `cargo install`)

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

### Cross-Platform Verification (Windows)

This project uses platform-specific code gates (`#[cfg(unix)]`, `#[cfg(not(unix))]`) for
Unix-specific functionality like terminal I/O. To verify these gates compile correctly on Windows
without needing a full Windows cross-compiler (mingw-w64), we use Rust's metadata-only compilation.

**How It Works:**

The `--emit=metadata` flag tells rustc to stop after type checking and MIR generation, skipping code
generation and linking entirely. This validates all platform-specific cfg gates without needing a
linker for the target platform.

```sh
# Verify Windows cross-compilation
fish run.fish check-windows-build

# Or run directly:
cargo rustc -p r3bl_tui --target x86_64-pc-windows-gnu -- --emit=metadata
```

**Prerequisites:**

The Windows target is automatically installed by `fish run.fish install-cargo-tools`. To install
manually:

```sh
rustup target add x86_64-pc-windows-gnu
```

**When to Use:**

- After modifying `#[cfg(unix)]` or `#[cfg(not(unix))]` conditional compilation gates
- Before committing platform-specific code changes
- As part of CI/CD for cross-platform verification
- When adding new platform-specific modules or functions

**Example Output:**

```text
Verifying Windows cross-compilation for r3bl_tui...
Target: x86_64-pc-windows-gnu
Mode: metadata only (no linking required)

✅ Windows cross-compilation check passed
Platform-specific cfg gates compile correctly for Windows.
```

**Technical Details:**

| Aspect              | Description                                                      |
| ------------------- | ---------------------------------------------------------------- |
| **Target**          | `x86_64-pc-windows-gnu` (Windows with GNU toolchain ABI)         |
| **Compilation**     | Stops at MIR stage (`--emit=metadata`), no object code generated |
| **Linking**         | Not required - no mingw-w64 or Windows SDK needed                |
| **What's Verified** | Syntax, types, trait bounds, cfg gate correctness                |
| **What's NOT**      | Runtime behavior, Windows-specific API calls, linking errors     |

This approach catches the most common cross-platform issues (missing cfg gates, type mismatches in
platform-specific code) with minimal setup overhead.

> **Platform Backends**: The TUI crate supports multiple backends: `Crossterm` (cross-platform,
> default on macOS/Windows) and `DirectToAnsi` (provided by `r3bl_tui` itself, Linux-native, ~18%
> better performance). We use cfg gates to ensure the selection of the correct backend for supported
> platforms. See [Platform-Specific Backends](./tui/README.md#platform-specific-backends) for
> details.

### Rust Toolchain Management

This project includes three complementary scripts for comprehensive Rust toolchain management, each
serving a specific purpose in the development workflow.

**Concurrency Safety:** Toolchain **modification** scripts (`rust-toolchain-update.fish` and
`rust-toolchain-sync-to-toml.fish`) use `mkdir` (atomic directory creation) to ensure only one
toolchain modification runs at a time. **Validation** scripts (`rust-toolchain-validate.fish` and
`check.fish`) are lock-free since they only read toolchain state - multiple validations can run
concurrently without conflict.

#### Why mkdir for Locking?

The key insight is understanding **atomicity** - when a system operation must check-and-act in a way
that's guaranteed to be indivisible:

**The Problem with File Existence Checks:**

Traditional approaches try to check if a lock exists, then create it:

```bash
# UNSAFE - Race condition!
if [ ! -f lock ]; then
    echo "timestamp" > temp
    mv temp lock  # TOCTOU race between check and move
fi
```

Between the check (`[ ! -f lock ]`) and the move (`mv temp lock`), another process can slip in and
also acquire the lock. This is called a **Time-Of-Check-Time-Of-Use (TOCTOU) race condition**.

**How mkdir Works - Atomic Check-and-Create:**

`mkdir` is different. It combines the check and create into ONE indivisible kernel operation:

```bash
# SAFE - Atomic operation
mkdir lock_dir  # Check AND create in ONE kernel operation
# Only ONE process succeeds; all others fail
```

When `mkdir` runs, the kernel does:

1. **Check**: Does the directory exist?
2. **Create**: If not, create it
3. **Return**: With ONE atomic operation - not two separate steps

Even with perfect timing and multiple processes starting simultaneously, only ONE can create the
directory.

**Technical Implementation:**

```fish
# In script_lib.fish
if mkdir ./rust-toolchain-script.lock 2>/dev/null
    # Lock acquired - this process has exclusive access
else
    # Lock held by another process
fi
```

**Key Advantages:**

- **Atomic**: Check-and-create in ONE kernel operation (impossible to race)
- **Simple**: No file descriptors or special handling needed
- **Reliable**: Works on all Unix systems (standard POSIX behavior)
- **Stale lock detection**: Automatically removes locks older than 10 minutes (crashed processes)
- **Crash-safe**: Abandoned locks are auto-cleaned after 10 minutes, or manually via
  `rm -rf rust-toolchain-script.lock`

The locking mechanism uses:

- **mkdir (atomic directory creation)**: Creates lock directory atomically - succeeds for one
  process, fails for all others
- **Atomic kernel operation**: Check-and-create happens as ONE indivisible operation - the
  definition of mutual exclusion
- **Timestamp tracking**: Stores creation time in `rust-toolchain-script.lock/timestamp` for age
  tracking
- **Stale lock detection**: Checks lock age on collision - auto-removes if older than 10 minutes
  (600 seconds)
- **Lock holder cleanup**: Process that acquired lock removes directory (including timestamp) when
  done
- **Conflict detection**: Failed mkdir indicates lock is held - shows age for transparency
- **Standard Unix pattern**: Used by systemd, init systems, and most Unix tools

#### 1. rust-toolchain-update.fish - Smart Validated Toolchain Updates

Intelligently finds and validates a stable nightly toolchain, preferring older versions for
stability while ensuring they don't have ICE (Internal Compiler Error) bugs.

```sh
# Via run.fish command
fish run.fish toolchain-update

# Or directly
./rust-toolchain-update.fish
```

**What it does:**

- **Smart search**: Tests nightly toolchains starting from 45 days ago, moving forward day-by-day
  until finding a stable one (up to today)
- **ICE validation**: Runs comprehensive validation suite on each candidate:
  - `cargo clippy --all-targets`
  - `cargo build`
  - `cargo test --all-targets`
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
  - Cleans all caches: cargo cache, build artifacts
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
# Via run.fish command
fish run.fish toolchain-sync

# Or directly
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
fish run.fish toolchain-sync  # Install components for 09-05
# Now rust-analyzer works for 09-05
```

#### 3. rust-toolchain-validate.fish - Unified Toolchain Validation

Consolidated validation script providing two modes: quick component check or comprehensive
build+test validation.

```sh
# Quick mode: Fast component check (~1-2 seconds)
fish run.fish toolchain-validate
./rust-toolchain-validate.fish quick

# Complete mode: Full build+test validation (~5-10 minutes)
fish run.fish toolchain-validate-complete
./rust-toolchain-validate.fish complete

# View detailed help
./rust-toolchain-validate.fish
```

**Mode Comparison:**

| Aspect            | Quick Mode                              | Complete Mode                        |
| ----------------- | --------------------------------------- | ------------------------------------ |
| **Time**          | ~1-2 seconds                            | ~5-10 minutes                        |
| **Purpose**       | Component verification                  | Stability verification               |
| **Use Case**      | Fast health checks                      | Pre-nightly validation               |
| **Checks**        | Installation + components + rustc works | Full build + clippy + tests + docs   |
| **ICE Detection** | No                                      | Yes (critical for nightly selection) |

**Quick Mode Validation:**

- ✅ Toolchain is installed via rustup
- ✅ rust-analyzer component is present
- ✅ rust-src component is present
- ✅ rustc --version works (not corrupted)

**Complete Mode Validation:**

- ✅ All quick mode checks
- ✅ cargo clippy --all-targets (no ICE)
- ✅ cargo build (no ICE)
- ✅ cargo test --all-targets (no ICE)
- ✅ cargo test --doc (no ICE)
- ✅ cargo doc --no-deps (no ICE)

**Return Codes:**

- `0`: ✅ Valid (quick) or Stable (complete)
- `1`: ❌ Not installed (quick) or ICE detected (complete)
- `2`: ⚠️ Missing components (quick only)
- `3`: 🔥 Corrupted - rustc fails (quick only)
- `4`: ❌ Failed to read rust-toolchain.toml

**When to use Quick Mode:**

- After installing/repairing toolchain with `sync-toolchain`
- Troubleshooting IDE issues (rust-analyzer not working?)
- Pre-flight check before running tests
- Regular health monitoring
- Part of automated CI/CD pipelines

**When to use Complete Mode:**

- Verifying nightly toolchain stability before using it
- Detecting Internal Compiler Errors (ICE) in compiler
- Before committing code with new toolchain
- During `toolchain-update` search (finding stable nightly)
- After major Rust version updates

**Integration with other toolchain scripts:**

- **check.fish**: Uses quick mode to check toolchain before running tests; calls `toolchain-sync` if
  invalid
- **rust-toolchain-sync-to-toml.fish**: Performs quick validation after installing components
- **rust-toolchain-update.fish**: Uses complete mode to find stable nightly

#### 4. remove_toolchains.sh - Testing Utility

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
fish run.fish toolchain-update
```

**⚠️ Warning:** This is a destructive testing utility. Use only when you understand the implications
and are prepared to reinstall toolchains.

#### Log File Output

All toolchain management scripts display detailed log file locations to stdout at startup:

```
📋 Detailed log: /home/nazmul/Downloads/rust-toolchain-sync-to-toml.log
```

This makes it easy to monitor progress and check detailed logs after operations complete:

```sh
# Watch logs in real-time
tail -f /home/nazmul/Downloads/rust-toolchain-update.log

# Or review after completion
cat /home/nazmul/Downloads/rust-toolchain-sync-to-toml.log
```

#### Comprehensive Toolchain Management System

The four scripts work together to provide a complete toolchain management solution:

**Four complementary scripts:**

- **validate** (`rust-toolchain-install-validate.fish`): Non-destructive validation of current
  toolchain
- **update** (`rust-toolchain-update.fish`): Smart search for stable nightly with comprehensive
  validation
- **sync** (`rust-toolchain-sync-to-toml.fish`): Install toolchain matching rust-toolchain.toml
- **remove** (`remove_toolchains.sh`): Testing utility to clean all toolchains (destructive)

**Key benefits:**

- **Stability**: Month-old nightlies have proven stability while providing recent features
- **Disk space savings**: Aggressive cleanup removes accumulated old toolchains
- **Consistency**: All developers use the same Rust version via `rust-toolchain.toml`
- **Automation ready**: `update` script designed to run weekly via systemd timer
- **Recovery ready**: `sync` script fixes environment after git operations
- **Validation ready**: `validate` script enables automated health checks in CI/CD pipelines
- **Testing support**: `remove` script enables testing upgrade workflows
- **Integrated monitoring**: `check.fish` automatically validates and repairs toolchain before
  running tests

### Unified Script Architecture

The project uses a clean separation of concerns across three main scripts with shared utilities:

```
┌──────────────────────────────────────────────────────────────────────────┐
│                           Bootstrap Flow                                  │
└──────────────────────────────────────────────────────────────────────────┘

    ┌─────────────────┐     calls     ┌──────────────────────────────────┐
    │  bootstrap.sh   │──────────────▶│  fish run.fish install-cargo-tools│
    │  (OS-level)     │               │  (Rust development tools)         │
    └─────────────────┘               └──────────────────────────────────┘
            │                                       │
            │ installs                              │ uses
            ▼                                       ▼
    ┌─────────────────┐               ┌──────────────────────────────────┐
    │ rustup, clang,  │               │        script_lib.fish           │
    │ fish, fzf,      │               │   (shared utility functions)     │
    │ inotify-tools   │               │                                  │
    └─────────────────┘               │  • install_windows_target        │
                                      │  • install_if_missing            │
                                      │  • install_cargo_tool            │
                                      │  • generate_cargo_config         │
                                      │  • read_toolchain_from_toml      │
                                      │  • acquire_toolchain_lock        │
                                      │  • ... 25+ shared functions      │
                                      └──────────────────────────────────┘
                                                    ▲
                    ┌───────────────────────────────┼───────────────────────┐
                    │                               │                       │
                    │ sources                       │ sources               │ sources
                    │                               │                       │
    ┌───────────────────────┐  ┌─────────────────────────────┐  ┌──────────────────────┐
    │       run.fish        │  │ rust-toolchain-update.fish  │  │ rust-toolchain-sync- │
    │  (dev commands)       │  │ (smart toolchain updater)   │  │ to-toml.fish         │
    │                       │  │                             │  │ (sync to TOML)       │
    │  • build, test, docs  │  │  • install_windows_target   │  │                      │
    │  • clippy, rustfmt    │  │  • acquire_toolchain_lock   │  │  • install_windows_  │
    │  • install-cargo-tools│  │  • read_toolchain_from_toml │  │    target            │
    │    (calls install_    │  │  • set_toolchain_in_toml    │  │  • acquire_toolchain_│
    │     windows_target)   │  │  • ...                      │  │    lock              │
    └───────────────────────┘  └─────────────────────────────┘  └──────────────────────┘
```

**Key DRY Principle**: All shared functionality lives in `script_lib.fish`. Individual scripts
source this library and call shared functions, ensuring consistent behavior and eliminating code
duplication. When a function like `install_windows_target` needs updating, it only needs to be
changed in one place.

**[`bootstrap.sh`](https://github.com/r3bl-org/r3bl-open-core/blob/main/bootstrap.sh)** - **OS-Level
Setup**

- System package manager detection and OS dependencies
- Rust toolchain installation via rustup
- Development environment setup (Fish shell, fzf, file watchers)
- Cross-platform compatibility (Linux, macOS)
- Calls run.fish for Rust-specific tooling

**[`run.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/run.fish)** - **Rust Development
Commands**

- **Workspace-wide commands** that operate on the entire project
- **Cargo tool installation** (install-cargo-tools with cargo-binstall, uv, bacon, etc.)
- **TUI-specific commands** for running examples and benchmarks
- **cmdr-specific commands** for binary management
- **Cross-platform file watching** using inotifywait (Linux) or fswatch (macOS)
- **Smart log monitoring** that detects and manages log files from different workspaces

**[`script_lib.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/script_lib.fish)** -
**Shared Utilities**

- Common functions used by both bootstrap.sh and run.fish
- Utility functions: install_if_missing, generate_cargo_config, install_cargo_tool
- Cross-platform package manager detection

All commands work from the root directory, eliminating the need to navigate between subdirectories.
This architecture ensures no redundancy - each tool is installed in exactly one place with clear
ownership.

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
