// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// You can get the unicode symbols for the drawings here:
// - <https://symbl.cc/en/unicode/blocks/miscellaneous-symbols-and-arrows/>
// - <https://symbl.cc/en/unicode/blocks/box-drawing/>
// - <https://symbl.cc/en/collections/brackets/>
// - <https://symbl.cc/en/collections/crosses/>

//! # Why R3BL?
//!
//! <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/tui/r3bl-tui.svg?raw=true" height="256px">
//!
//! <!-- R3BL TUI library & suite of apps focused on developer productivity -->
//!
//! <span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span
//! style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span
//! style="color:#F92363"> </span><span style="color:#F82067">T</span><span
//! style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span
//! style="color:#F31874"> </span><span style="color:#F11678">l</span><span
//! style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span
//! style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span
//! style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span
//! style="color:#E10895"> </span><span style="color:#DE0799">a</span><span
//! style="color:#DB069E">l</span><span style="color:#D804A2">l</span><span
//! style="color:#D503A6">o</span><span style="color:#D203AA">w</span><span
//! style="color:#CF02AE">s</span><span style="color:#CB01B2"> </span><span
//! style="color:#C801B6">y</span><span style="color:#C501B9">o</span><span
//! style="color:#C101BD">u</span><span style="color:#BD01C1"> </span><span
//! style="color:#BA01C4">t</span><span style="color:#B601C8">o</span><span
//! style="color:#B201CB"> </span><span style="color:#AE02CF">c</span><span
//! style="color:#AA03D2">r</span><span style="color:#A603D5">e</span><span
//! style="color:#A204D8">a</span><span style="color:#9E06DB">t</span><span
//! style="color:#9A07DE">e</span><span style="color:#9608E1"> </span><span
//! style="color:#910AE3">a</span><span style="color:#8D0BE6">p</span><span
//! style="color:#890DE8">p</span><span style="color:#850FEB">s</span><span
//! style="color:#8111ED"> </span><span style="color:#7C13EF">t</span><span
//! style="color:#7815F1">o</span><span style="color:#7418F3"> </span><span
//! style="color:#701AF5">e</span><span style="color:#6B1DF6">n</span><span
//! style="color:#6720F8">h</span><span style="color:#6322F9">a</span><span
//! style="color:#5F25FA">n</span><span style="color:#5B28FB">c</span><span
//! style="color:#572CFC">e</span><span style="color:#532FFD"> </span><span
//! style="color:#4F32FD">d</span><span style="color:#4B36FE">e</span><span
//! style="color:#4739FE">v</span><span style="color:#443DFE">e</span><span
//! style="color:#4040FE">l</span><span style="color:#3C44FE">o</span><span
//! style="color:#3948FE">p</span><span style="color:#354CFE">e</span><span
//! style="color:#324FFD">r</span><span style="color:#2E53FD"> </span><span
//! style="color:#2B57FC">p</span><span style="color:#285BFB">r</span><span
//! style="color:#245EFA">o</span><span style="color:#215FF9">d</span><span
//! style="color:#1E63F8">u</span><span style="color:#1A67F7">c</span><span
//! style="color:#176BF6">t</span><span style="color:#136FF5">i</span><span
//! style="color:#1073F4">v</span><span style="color:#0C77F3">i</span><span
//! style="color:#097BF2">t</span><span style="color:#057FF1">y</span>.
//!
//! Please read the
//! main [README.md](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md) of
//! the `r3bl-open-core` monorepo and workspace to get a better understanding of the
//! context in which this crate is meant to exist.
//!
//! # Table of contents
//!
//! <!-- TOC -->
//! - [Introduction](#introduction)
//! - [Framework highlights](#framework-highlights)
//! - [Full TUI, Partial TUI, and async
//!   readline](#full-tui-partial-tui-and-async-readline)
//!   - [Partial TUI for simple choice](#partial-tui-for-simple-choice)
//!   - [Partial TUI for REPL](#partial-tui-for-repl)
//!   - [Full TUI for immersive apps](#full-tui-for-immersive-apps)
//!   - [Power via composition](#power-via-composition)
//! - [Changelog](#changelog)
//! - [Learn how these crates are built, provide
//!   feedback](#learn-how-these-crates-are-built-provide-feedback)
//! - [Run the demo locally](#run-the-demo-locally)
//!   - [Prerequisites](#prerequisites)
//!   - [Running examples](#running-examples)
//! - [TUI Development Workflow](#tui-development-workflow)
//!   - [TUI-Specific Commands](#tui-specific-commands)
//!   - [Testing and Development](#testing-and-development)
//!     - [VT100 ANSI Conformance Testing](#vt100-ansi-conformance-testing)
//!     - [Markdown Parser Conformance Testing](#markdown-parser-conformance-testing)
//!   - [Performance Analysis Features](#performance-analysis-features)
//! - [Examples to get you started](#examples-to-get-you-started)
//!   - [Video of the demo in action](#video-of-the-demo-in-action)
//! - [Type-safe bounds checking](#type-safe-bounds-checking)
//!   - [The Problem](#the-problem)
//!   - [The Solution](#the-solution)
//!   - [Key Benefits](#key-benefits)
//!   - [Architecture](#architecture)
//!   - [Common Patterns](#common-patterns)
//!   - [Learn More](#learn-more)
//! - [Grapheme support](#grapheme-support)
//!   - [The Challenge](#the-challenge)
//!   - [The Solution: Three Index Types](#the-solution-three-index-types)
//!   - [Type-Safe String Handling](#type-safe-string-handling)
//!   - [Key Features](#key-features)
//!   - [Learn More](#learn-more-1)
//! - [Layout, rendering, and event handling](#layout-rendering-and-event-handling)
//! - [Architecture overview, is message passing, was shared
//!   memory](#architecture-overview-is-message-passing-was-shared-memory)
//! - [I/O devices for full TUI, choice, and
//!   REPL](#io-devices-for-full-tui-choice-and-repl)
//! - [Life of an input event for a Full TUI
//!   app](#life-of-an-input-event-for-a-full-tui-app)
//! - [Life of a signal (aka "out of band
//!   event")](#life-of-a-signal-aka-out-of-band-event)
//! - [The window](#the-window)
//! - [Layout and styling](#layout-and-styling)
//! - [Component registry, event routing, focus
//!   mgmt](#component-registry-event-routing-focus-mgmt)
//! - [Input event specificity](#input-event-specificity)
//! - [Rendering and painting](#rendering-and-painting)
//!   - [Offscreen buffer](#offscreen-buffer)
//!   - [Render pipeline](#render-pipeline)
//!   - [First render](#first-render)
//!   - [Subsequent render](#subsequent-render)
//! - [How does the editor component work?](#how-does-the-editor-component-work)
//!   - [Zero-Copy Gap Buffer for High
//!     Performance](#zero-copy-gap-buffer-for-high-performance)
//!     - [Key Performance Features](#key-performance-features)
//!     - [Storage Architecture](#storage-architecture)
//!     - [UTF-8 Safety Strategy](#utf-8-safety-strategy)
//!     - [Optimization: Append Detection](#optimization-append-detection)
//!     - [Learn More](#learn-more-2)
//! - [Markdown Parser with R3BL Extensions](#markdown-parser-with-r3bl-extensions)
//!   - [Key Features](#key-features-1)
//!   - [Architecture and Parser Priority](#architecture-and-parser-priority)
//!   - [Integration with Syntax Highlighting](#integration-with-syntax-highlighting)
//!   - [Performance Characteristics](#performance-characteristics)
//!   - [Learn More](#learn-more-3)
//! - [Terminal Multiplexer with VT-100 ANSI
//!   Parsing](#terminal-multiplexer-with-vt-100-ansi-parsing)
//!   - [Core Capabilities](#core-capabilities)
//!   - [Architecture: The Virtual Terminal
//!     Pipeline](#architecture-the-virtual-terminal-pipeline)
//!   - [VT-100 ANSI Parser Implementation](#vt-100-ansi-parser-implementation)
//!   - [Usage Example](#usage-example)
//!   - [Learn More](#learn-more-4)
//! - [Painting the caret](#painting-the-caret)
//! - [How do modal dialog boxes work?](#how-do-modal-dialog-boxes-work)
//!   - [Two callback functions](#two-callback-functions)
//!   - [Async Autocomplete Provider](#async-autocomplete-provider)
//! - [Lolcat support](#lolcat-support)
//! - [Issues and PRs](#issues-and-prs)
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! You can build fully async TUI (text user interface) apps with a modern API that brings
//! the best of the web frontend development ideas to TUI apps written in Rust:
//!
//! 1. Reactive & unidirectional data flow architecture from frontend development ([React](https://react.dev/),
//!    [SolidJS](https://www.solidjs.com/), [Elm](https://guide.elm-lang.org/architecture/),
//!    [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack Compose](https://developer.android.com/compose)).
//! 2. [Responsive design](https://developer.mozilla.org/en-US/docs/Learn/CSS/CSS_layout/Responsive_Design)
//!    with [CSS](https://www.w3.org/TR/CSS/#css), [flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox)
//!    like concepts.
//! 3. [Declarative style](https://ui.dev/imperative-vs-declarative-programming) of
//!    expressing styling and layouts.
//!
//! And since this is using Rust and [Tokio](https://crates.io/crates/tokio) you get the
//! advantages of concurrency and parallelism built-in. No more blocking the main thread
//! for user input, for async middleware, or even rendering 🎉.
//!
//! This framework is [loosely coupled and strongly
//! coherent](https://developerlife.com/2015/11/05/loosely-coupled-strongly-coherent/)
//! meaning that you can pick and choose whatever pieces you would like to use without
//! having the cognitive load of having to grok all the things in the codebase. Its more
//! like a collection of mostly independent modules that work well with each other, but
//! know very little about each other.
//!
//! This is the main crate that contains the core functionality for building TUI apps. It
//! allows you to build apps that range from "full" TUI to "partial" TUI, and everything
//! in the middle.
//!
//! Here are some videos that you can watch to get a better understanding of TTY
//! programming.
//!
//! - [Build with Naz: TTY playlist](https://www.youtube.com/playlist?list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3)
//! - [Build with Naz: async readline](https://www.youtube.com/playlist?list=PLofhE49PEwmwelPkhfiqdFQ9IXnmGdnSE)
//!
//! # Framework highlights
//!
//! Here are some highlights of this library:
//!
//! - It works over SSH without flickering, since it uses double buffering to paint the
//!   UI, and diffs the output of renders, to only paint the parts of the screen that
//!   changed.
//! - It automatically detects terminal capabilities and gracefully degrades to the lowest
//!   common denominator.
//! - Uses very few dependencies. Almost all the code required for the core functionality
//!   is written in Rust in this crate. This ensures that over time, as open source
//!   projects get unfunded, and abandoned, there's minimized risk of this crate being
//!   affected. Any dependencies that are used are well maintained and supported.
//! - It is a modern & easy to use and approachable API that is inspired by React, JSX,
//!   CSS, Elm. Lots of components and things are provided for you so you don't have to
//!   build them from scratch. This is a full featured component library including:
//!   - Elm like architecture with unidirectional data flow. The state is mutable. Async
//!     middleware functions are supported, and they communicate with the main thread and
//!     the [App] using an async `tokio::mpsc` channel and signals.
//!   - CSS like declarative styling engine.
//!   - CSS like flexbox like declarative layout engine which is fully responsive. You can
//!     resize your terminal window and everything will be laid out correctly.
//!   - A terminal independent underlying rendering and painting engine (can use crossterm
//!     or termion or whatever you want).
//!   - Markdown text editor with syntax highlighting support, metadata (tags, title,
//!     author, date), smart lists. This uses a custom Markdown parser and custom syntax
//!     highlighter. Syntax highlighting for code blocks is provided by the syntect crate.
//!   - Modal dialog boxes. And autocompletion dialog boxes.
//!   - Lolcat (color gradients) implementation with a rainbow color-wheel palette. All
//!     the color output is sensitive to the capabilities of the terminal. Colors are
//!     gracefully downgraded from truecolor, to ANSI256, to grayscale.
//!   - Support for Unicode grapheme clusters in strings. You can safely use emojis, and
//!     other Unicode characters in your TUI apps.
//!   - Support for mouse events.
//! - The entire TUI framework itself supports concurrency & parallelism (user input,
//!   rendering, etc. are generally non blocking).
//! - It is fast! There are no needless re-renders, or flickering. Animations and color
//!   changes are smooth (check this out for yourself by running the examples). You can
//!   even build your TUI in layers (like z-order in a browser's DOM).
//!
//! # Full TUI, Partial TUI, and async readline
//!
//! This crate allows you to build apps that range from "full" TUI to "partial" TUI, and
//! everything in the middle. Here are some videos that you can watch to get a better
//! understanding of TTY programming.
//!
//! - [Build with Naz: TTY playlist](https://www.youtube.com/playlist?list=PLofhE49PEwmw3MKOU1Kn3xbP4FRQR4Mb3)
//! - [Build with Naz: async readline](https://www.youtube.com/playlist?list=PLofhE49PEwmwelPkhfiqdFQ9IXnmGdnSE)
//!
//! ## Partial TUI for simple choice
//!
//! [`mod@readline_async::choose_api`] allows you to build less interactive apps that ask
//! a user user to make choices from a list of options and then use a decision tree to
//! perform actions.
//!
//! An example of this is this "Partial TUI" app `giti` in the
//! [`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate. You
//! can install & run this with the following command:
//!
//! ```sh
//! cargo install r3bl-cmdr
//! giti
//! ```
//!
//! ## Partial TUI for REPL
//!
//! [`mod@readline_async::readline_async_api`] gives you the ability to easily ask for
//! user input in a line editor. You can customize the prompt, and other behaviors, like
//! input history.
//!
//! Using this, you can build your own async shell programs using "async readline &
//! stdout". Use advanced features like showing indeterminate progress spinners, and even
//! write to stdout in an async manner, without clobbering the prompt / async readline, or
//! the spinner. When the spinner is active, it pauses output to stdout, and resumes it
//! when the spinner is stopped.
//!
//! An example of this is this "Partial TUI" app `giti` in the
//! [`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate. You
//! can install & run this with the following command:
//!
//! ```sh
//! cargo install r3bl-cmdr
//! giti
//! ```
//!
//! Here are other examples of this:
//!
//! 1. <https://github.com/nazmulidris/rust-scratch/tree/main/tcp-api-server>
//! 2. <https://github.com/r3bl-org/r3bl-open-core/tree/main/tui/examples>
//! ## Full TUI for immersive apps
//!
//! **The bulk of this document is about this**. [`mod@tui::terminal_window_api`] gives
//! you "raw mode", "alternate screen" and "full screen" support, while being totally
//! async. An example of this is the "Full TUI" app `edi` in the
//! [`r3bl-cmdr`](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) crate. You
//! can install & run this with the following command:
//!
//! ```sh
//! cargo install r3bl-cmdr
//! edi
//! ```
//!
//! ## Power via composition
//!
//! You can mix and match "Full TUI" with "Partial TUI" to build for whatever use case you
//! need. `r3bl_tui` allows you to create application state that can be moved between
//! various "applets", where each "applet" can be "Full TUI" or "Partial TUI".
//!
//! # Changelog
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_tui)
//! to see how the library has evolved over time.
//!
//! # Learn how these crates are built, provide feedback
//!
//! To learn how we built this crate, please take a look at the following resources.
//! - If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom).
//!   Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer [site](https://developerlife.com/).
//!
//! # Run the demo locally
//!
//! Once you've cloned [the repo](https://github.com/r3bl-org/r3bl-open-core) to a folder
//! on your computer, follow these steps:
//!
//! ## Prerequisites
//!
//! 🌠 The easiest way to get started is to use the bootstrap script:
//!
//! ```bash
//! ./bootstrap.sh
//! fish run.fish install-cargo-tools
//! ```
//!
//! This script above automatically installs:
//! - Rust toolchain via rustup
//! - Fish shell
//! - File watchers (inotifywait/fswatch)
//! - All required cargo development tools
//!
//! For complete development setup and all available commands, see the
//! [repository README](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md).
//!
//! ## Running examples
//!
//! After setup, you can run the examples interactively from the repository root:
//!
//! ```sh
//! # Run examples interactively (choose from list)
//! fish run.fish run-examples
//!
//! # Run examples with release optimizations
//! fish run.fish run-examples --release
//!
//! # Run examples without logging
//! fish run.fish run-examples --no-log
//! ```
//!
//! You can also run examples directly:
//! ```sh
//! cd tui/examples
//! cargo run --release --example demo -- --no-log
//! ```
//!
//! These examples cover the entire surface area of the TUI API. The unified
//! [`run.fish`](https://github.com/r3bl-org/r3bl-open-core/blob/main/run.fish) script
//! at the repository root provides all development commands for the entire workspace.
//!
//! # TUI Development Workflow
//!
//! For TUI library development, use these commands from the repository root:
//!
//! ```sh
//! # Terminal 1: Monitor logs from examples
//! fish run.fish log
//!
//! # Terminal 2: Run examples interactively
//! fish run.fish run-examples
//! ```
//!
//! ## TUI-Specific Commands
//!
//! | Command                                     | Description                                      |
//! | ------------------------------------------- | ------------------------------------------------ |
//! | `fish run.fish run-examples`                | Run TUI examples interactively with options      |
//! | `fish run.fish run-examples-flamegraph-svg` | Generate SVG flamegraph for performance analysis |
//! | `fish run.fish run-examples-flamegraph-fold`| Generate perf-folded format for analysis         |
//! | `fish run.fish bench`                       | Run benchmarks with real-time output             |
//! | `fish run.fish log`                         | Monitor log files with smart detection           |
//!
//! ## Testing and Development
//!
//! | Command                                    | Description                         |
//! | ------------------------------------------ | ----------------------------------- |
//! | `fish run.fish test`                       | Run all tests                       |
//! | `fish run.fish watch-all-tests`            | Watch files, run all tests          |
//! | `fish run.fish watch-one-test <pattern>`   | Watch files, run specific test      |
//! | `fish run.fish clippy`                     | Run clippy with fixes               |
//! | `fish run.fish watch-clippy`               | Watch files, run clippy             |
//! | `fish run.fish docs`                       | Generate documentation              |
//!
//! ### VT100 ANSI Conformance Testing
//!
//! The TUI library includes comprehensive VT100/ANSI escape sequence conformance tests
//! that validate the terminal emulation pipeline:
//!
//! ```bash
//! # Run all VT100 ANSI conformance tests
//! cargo test vt_100_ansi_conformance_tests
//!
//! # Run specific conformance test categories
//! cargo test test_real_world_scenarios     # vim, emacs, tmux patterns
//! cargo test test_cursor_operations        # cursor positioning & movement
//! cargo test test_sgr_and_character_sets   # text styling & colors
//! ```
//!
//! **Testing Architecture Features:**
//! - **Type-safe sequence builders**: Uses [`CsiSequence`], [`EscSequence`], and
//!   [`SgrCode`] builders instead of hardcoded escape strings
//! - **Real-world scenarios**: Tests realistic terminal applications (vim, emacs, tmux)
//!   with authentic 80x25 terminal dimensions
//! - **VT100 specification compliance**: Comprehensive coverage of ANSI escape sequences
//!   with proper bounds checking and edge case handling
//! - **Conformance data modules**: Organized sequence patterns for different terminal
//!   applications and use cases
//!
//! The conformance tests ensure the ANSI parser correctly processes sequences from
//! real terminal applications and maintains compatibility with VT100 specifications.
//!
//! ### Markdown Parser Conformance Testing
//!
//! The markdown parser includes a comprehensive conformance test suite with organized
//! test data that validates parsing correctness across diverse markdown content:
//!
//! ```bash
//! # Run all markdown parser tests
//! cargo test md_parser
//!
//! # Run specific test categories
//! cargo test parser_snapshot_tests     # Snapshot testing for parser output
//! cargo test parser_bench_tests        # Performance benchmarks
//! cargo test conformance_test_data     # Conformance test data validation
//! ```
//!
//! **Testing Infrastructure Features:**
//! - **Conformance test data organization**: Test inputs organized by complexity
//!   (invalid, small, medium, large, jumbo)
//! - **Snapshot testing**: Validates parser output structure and correctness using insta
//!   snapshots
//! - **Performance benchmarks**: Ensures parser maintains efficient performance across
//!   content sizes
//! - **Real-world documents**: Tests with authentic markdown files including complex
//!   nested structures
//!
//! **Test Data Categories:**
//! - **Invalid inputs**: Edge cases and malformed syntax for error handling validation
//! - **Valid small inputs**: Simple formatting and single-line markdown
//! - **Valid medium inputs**: Multi-paragraph content and structured documents
//! - **Valid large inputs**: Complex nested structures and advanced features
//! - **Valid jumbo inputs**: Real-world files and comprehensive documents
//!
//! The conformance tests ensure the parser correctly handles both standard markdown
//! syntax and R3BL extensions while maintaining performance and reliability.
//!
//! For complete development setup and all available commands, see the
//! [repository README](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md).
//!
//! ## Performance Analysis Features
//!
//! - **Flamegraph profiling**: Generate SVG and perf-folded formats for performance
//!   analysis
//! - **Real-time benchmarking**: Run benchmarks with live output
//! - **Cross-platform file watching**: Uses `inotifywait` (Linux) or `fswatch` (macOS)
//! - **Interactive example selection**: Choose examples with fuzzy search
//! - **Smart log monitoring**: Automatically detects and manages log files
//!
//! # Examples to get you started
//!
//! <!-- How to upload video: https://stackoverflow.com/a/68269430/2085356 -->
//!
//! ## Video of the demo in action
//!
//! ![video-gif](https://user-images.githubusercontent.com/2966499/233799311-210b887e-0aa6-470a-bcea-ee8e0e3eb019.gif)
//!
//! Here's a video of a prototype of [R3BL CMDR](https://github.com/r3bl-org/r3bl-cmdr)
//! app built using this TUI engine.
//!
//! ![rc](https://user-images.githubusercontent.com/2966499/234949476-98ad595a-3b72-497f-8056-84b6acda80e2.gif)
//!
//! # Type-safe bounds checking
//!
//! The R3BL TUI engine uses a comprehensive type-safe bounds checking system that
//! eliminates off-by-one errors and prevents mixing incompatible index types (like
//! comparing row positions with column widths) at compile time.
//!
//! ## The Problem
//!
//! Off-by-one errors and index confusion have plagued programming since its inception.
//! UI and layout development (web, mobile, desktop, GUI, TUI) amplifies these challenges
//! with multiple sources of confusion:
//!
//! - **0-based vs 1-based**: Mixing indices (positions, 0-based) with lengths (sizes,
//!   1-based)
//! - **Dimension confusion**: Mixing row and column types
//! - **Semantic ambiguity**: Is this value a position, a size, or a count?
//! - **Range boundary confusion**: Inclusive `[min, max]` vs exclusive `[start, end)` vs
//!   position+size `[start, start+width)` - different use cases demand different
//!   semantics
//!
//! ```rust,should_panic
//! // ❌ Unsafe: raw integers hide these distinctions
//! let cursor_row: usize = 5;        // Is this 0-based or 1-based?
//! let viewport_width: usize = 80;   // Is this a size or position?
//! let buffer_size: usize = 100;     // Can I use this as an index?
//! let buffer: Vec<u8> = vec![0; 100];
//!
//! // Problem 1: Dimension confusion
//! if cursor_row < viewport_width { /* Mixing row index with column size! */ }
//!
//! // Problem 2: 0-based vs 1-based confusion
//! if buffer_size > 0 {
//!     let last = buffer[buffer_size];  /* Off-by-one: size is 1-based! PANICS! */
//! }
//!
//! // Problem 3: Range boundary confusion
//! let scroll_region_start = 2_usize;
//! let scroll_region_end = 5_usize;
//! // Is this [2, 5] inclusive or [2, 5) exclusive?
//! // VT-100 uses inclusive, but iteration needs exclusive!
//! for row in scroll_region_start..scroll_region_end {
//!     // Processes rows 2, 3, 4 (exclusive end)
//!     // But VT-100 scroll region 2..=5 includes row 5!
//!     // Easy to create off-by-one errors when converting
//! }
//! ```
//!
//! ## The Solution
//!
//! Use strongly-typed indices and lengths with semantic validation:
//!
//! ```rust
//! use r3bl_tui::{row, height, ArrayBoundsCheck, ArrayOverflowResult};
//!
//! let cursor_row = row(5);          // RowIndex (0-based position)
//! let viewport_height = height(24); // RowHeight (1-based size)
//!
//! // ✅ Type-safe: Compiler prevents row/column confusion
//! if cursor_row.overflows(viewport_height) == ArrayOverflowResult::Within {
//!     // Safe to access buffer[cursor_row]
//! }
//! ```
//!
//! ## Key Benefits
//!
//! 1. **Compile-time safety**: Impossible to compare [`RowIndex`] with [`ColWidth`]
//! 2. **Semantic clarity**: Code intent is explicit (position vs size, row vs column)
//! 3. **Zero-cost abstraction**: No runtime overhead compared to raw integers
//! 4. **Comprehensive coverage**: Handles array access, cursor positioning, viewport
//!    visibility, and range validation
//!
//! ## Architecture
//!
//! The system uses a two-tier trait architecture:
//!
//! - **Foundational traits**: Core operations ([`IndexOps`], [`LengthOps`]) that work
//!   with any index/length type
//! - **Semantic traits**: Use-case specific validation ([`ArrayBoundsCheck`],
//!   [`CursorBoundsCheck`], [`ViewportBoundsCheck`], [`RangeBoundsExt`],
//!   [`RangeConvertExt`])
//!
//! ## Common Patterns
//!
//! **Array/buffer access** (strict bounds):
//! ```rust
//! use r3bl_tui::{col, width, ArrayBoundsCheck, ArrayOverflowResult};
//! # let buffer: Vec<char> = vec!['a'; 10];
//! let index = col(5);
//! let buffer_width = width(10);
//!
//! // Check before accessing
//! if index.overflows(buffer_width) == ArrayOverflowResult::Within {
//!     let ch = buffer[index.as_usize()]; // Safe access
//! }
//! ```
//!
//! **Text cursor positioning** (allows end-of-line):
//! ```rust
//! use r3bl_tui::{col, width, CursorBoundsCheck, CursorPositionBoundsStatus};
//! let cursor_col = col(10);
//! let line_width = width(10);
//!
//! // Cursor can be placed after last character (position == length)
//! match line_width.check_cursor_position_bounds(cursor_col) {
//!     CursorPositionBoundsStatus::AtEnd => { /* Valid: cursor after last char */ }
//!     CursorPositionBoundsStatus::Within => { /* Valid: cursor on character */ }
//!     CursorPositionBoundsStatus::Beyond => { /* Invalid: out of bounds */ }
//!     _ => {}
//! }
//! ```
//!
//! **Viewport visibility** (rendering optimization):
//! ```rust
//! use r3bl_tui::{row, height, ViewportBoundsCheck, RangeBoundsResult};
//! let content_row = row(15);
//! let viewport_start = row(10);
//! let viewport_size = height(20);
//!
//! // Check if content is visible before rendering
//! if content_row.check_viewport_bounds(viewport_start, viewport_size) == RangeBoundsResult::Within {
//!     // Render this row
//! }
//! ```
//!
//! **Range boundary handling** (inclusive vs exclusive):
//! ```rust
//! use r3bl_tui::{row, RangeConvertExt};
//!
//! // VT-100 scroll region: inclusive bounds [2, 5] means rows 2,3,4,5
//! let scroll_region = row(2)..=row(5);
//!
//! // Convert to exclusive for Rust iteration: [2, 6) means rows 2,3,4,5
//! let iter_range = scroll_region.to_exclusive();  // row(2)..row(6)
//!
//! // Now safe to use for iteration - no off-by-one errors!
//! // for row in iter_range { /* process rows 2,3,4,5 */ }
//! ```
//!
//! ## Learn More
//!
//! For comprehensive documentation including:
//! - Complete trait reference and method details
//! - Decision trees for choosing the right trait
//! - Common pitfalls and best practices
//! - Advanced patterns (range validation, scroll regions, text selections)
//!
//! See the extensive and detailed [`bounds_check` module
//! documentation](mod@crate::core::units::bounds_check).
//!
//! # Grapheme support
//!
//! The R3BL TUI engine provides comprehensive Unicode support through grapheme cluster
//! handling, ensuring correct text manipulation regardless of character complexity.
//!
//! ## The Challenge
//!
//! Unicode text contains characters that may:
//! - Occupy multiple bytes (UTF-8 encoding: 1-4 bytes per character)
//! - Occupy multiple display columns (e.g., emoji take 2 columns, CJK characters)
//! - Be composed of multiple codepoints (e.g., `👨🏾‍🤝‍👨🏿` is 5 codepoints combined)
//!
//! This creates a fundamental mismatch between:
//! - **Memory layout** (byte indices in UTF-8)
//! - **Logical structure** (user-perceived characters)
//! - **Visual display** (terminal column positions)
//!
//! Traditional string indexing fails with such text:
//!
//! ```rust,should_panic
//! // ❌ Unsafe: byte indexing can split multi-byte characters
//! let text = "Hello 👋🏽";  // Wave emoji with skin tone modifier
//! let byte_len = text.len();        // 14 bytes (not 7 characters!)
//! let _substring = &text[0..7];     // PANICS! Splits 👋 emoji mid-character
//! ```
//!
//! ## The Solution: Three Index Types
//!
//! The grapheme system uses three distinct index types to handle text correctly:
//!
//! 1. **[`ByteIndex`]** - Memory position (UTF-8 byte offset)
//!    - For string slicing at valid UTF-8 boundaries
//!    - Example: In "H😀!", 'H' at byte 0, '😀' at byte 1, '!' at byte 5
//!
//! 2. **[`SegIndex`]** - Logical position (grapheme cluster index)
//!    - For cursor movement and text editing
//!    - Example: In "H😀!", 3 segments: seg\[0\]='H', seg\[1\]='😀', seg\[2\]='!'
//!
//! 3. **[`ColIndex`]** - Display position (terminal column)
//!    - For rendering and visual positioning
//!    - Example: In "H😀!", 'H' at col 0, '😀' spans cols 1-2, '!' at col 3
//!
//! ### Visual Example
//!
//! ```text
//! String: "H😀!"
//!
//! ByteIndex: 0 1 2 3 4 5
//! Content:  [H][😀----][!]
//!
//! SegIndex:  0    1     2
//! Segments: [H] [😀]  [!]
//!
//! ColIndex:  0  1  2   3
//! Display:  [H][😀--] [!]
//! ```
//!
//! ## Type-Safe String Handling
//!
//! Use [`GCStringOwned`] for grapheme-aware string operations:
//!
//! ```rust
//! use r3bl_tui::*;
//!
//! let text = GCStringOwned::new("Hello 👋🏽");
//! let grapheme_count = text.len();           // 7 grapheme clusters
//! let display_width = text.display_width;    // Actual terminal columns needed
//!
//! // Safe conversions between index types
//! // ByteIndex → SegIndex: find which character contains a byte
//! // ColIndex → SegIndex: find which character is at a column
//! // SegIndex → ColIndex: find the display column of a character
//! ```
//!
//! ## Key Features
//!
//! - **Grapheme cluster awareness**: Correctly handles composed characters
//!   - Emoji with modifiers: `👋🏽` (wave + skin tone)
//!   - Complex emoji: `👨🏾‍🤝‍👨🏿` (5 codepoints, 1 user-perceived character)
//!   - Accented letters: `é` (may be 1 or 2 codepoints)
//!
//! - **Display width calculation**: Accurately computes terminal column width
//!   - ASCII: 'H' = 1 column
//!   - Emoji: '😀' = 2 columns
//!   - CJK: '中' = 2 columns
//!
//! - **Safe slicing**: Substring operations never split multi-byte characters
//!   - Conversion methods return [`Option<SegIndex>`] for invalid indices
//!   - [`ByteIndex`] in the middle of a character → `None`
//!
//! - **Iterator support**: Iterate over graphemes, not bytes or codepoints
//!
//! ## Learn More
//!
//! For comprehensive documentation including:
//! - Detailed explanations of the three index types and conversions
//! - Platform-specific terminal rendering differences (Linux/macOS/Windows)
//! - Performance optimization details (memory latency considerations)
//! - Complete API reference for [`GCStringOwned`]
//!
//! See the extensive and detailed [`graphemes` module
//! documentation](mod@crate::core::graphemes) documentation.
//!
//! # Layout, rendering, and event handling
//!
//! The current render pipeline flow is:
//! 1. Input Event → State generation → [App] renders to [`RenderOps`]
//! 2. [`RenderOps`] → Rendered to [`OffscreenBuffer`] ([`PixelChar`] grid)
//! 3. [`OffscreenBuffer`] → Diffed with previous buffer → Generate diff chunks
//! 4. Diff chunks → Converted back to [`RenderOps`] for painting
//! 5. [`RenderOps`] execution → Each op routed through crossterm backend
//! 6. Crossterm → Converts to ANSI escape sequences → Queued to stdout → Flushed
//!
//! ```text
//! ╭───────────────────────────────────────────────╮
//! │                                               │
//! │  main.rs                                      │
//! │                          ╭──────────────────╮ │
//! │  GlobalData ────────────>│ window size      │ │
//! │  HasFocus                │ offscreen buffer │ │
//! │  ComponentRegistryMap    │ state            │ │
//! │  App & Component(s)      │ channel sender   │ │
//! │                          ╰──────────────────╯ │
//! │                                               │
//! ╰───────────────────────────────────────────────╯
//! ```
//! <!-- https://asciiflow.com/#/share/eJzNkE0KwjAQha9SZiEK4kIUsTtR1I0b19mMdaqFdFKSFK0iXkI8jHgaT2JcqPUHoS7E4REmJN97k6yBMSbwOZWyChIz0uDDWsBSgN9utKoCMtfVW03XWVpatxFw2h3%2FVkKwW73ClUNjjLimzTfo51tfKx8xkGqCsocWC1ruDxd%2BEfFULTwTreg2V95%2BiKavgvTd6y%2FnKgxNoIl4O0nDkPQz3lVxopjYjmkWGauzESY53Fi0tL3Wa3onSbzS3aRsKg%2FpwRyZSXqGeOqyX%2FAffH%2FRuqF%2FKwEb2JwB17oGMg%3D%3D) -->
//!
//! - The main struct for building a TUI app is your struct which implements the [App]
//!   trait.
//! - The main event loop takes an [App] trait object and starts listening for input
//!   events. It enters raw mode, and paints to an alternate screen buffer, leaving your
//!   original scroll back buffer and history intact. When you `request_shutdown` this TUI
//!   app, it will return your terminal to where you'd left off.
//! - The [`main_event_loop`] is where many global structs live which are shared across
//!   the lifetime of your app. These include the following:
//!   - [`HasFocus`]
//!   - [`ComponentRegistryMap`]
//!   - [`GlobalData`] which contains the following
//!     - Global application state. This is mutable. Whenever an input event or signal is
//!       processed the entire [App] gets re-rendered. This is the unidirectional data
//!       flow architecture inspired by React and Elm.
//! - Your [App] trait impl is the main entry point for laying out the entire application.
//!   Before the first render, the [App] is initialized (via a call to [`App::app_init`]),
//!   and is responsible for creating all the [Component]s that it uses, and saving them
//!   to the [`ComponentRegistryMap`].
//!   - State is stored in many places. Globally at the [`GlobalData`] level, and also in
//!     [App], and also in [Component].
//! - This sets everything up so that [`App::app_render`],
//!   [`App::app_handle_input_event`], and [`App::app_handle_signal`] can be called at a
//!   later time.
//! - The [`App::app_render`] method is responsible for creating the layout by using
//!   [Surface] and [`FlexBox`] to arrange whatever [Component]'s are in the
//!   [`ComponentRegistryMap`].
//! - The [`App::app_handle_input_event`] method is responsible for handling events that
//!   are sent to the [App] trait when user input is detected from the keyboard or mouse.
//!   Similarly the [`App::app_handle_signal`] deals with signals that are sent from
//!   background threads (Tokio tasks) to the main thread, which then get routed to the
//!   [App] trait object. Typically this will then get routed to the [Component] that
//!   currently has focus.
//!
//! # Architecture overview, is message passing, was shared memory
//!
//! Versions of this crate <= `0.3.10` used shared memory to communicate between the
//! background threads and the main thread. This was done using the async `Arc<RwLock<T>>`
//! from tokio. The state storage, mutation, subscription (on change handlers) were all
//! managed by the
//! [`r3bl_redux`](https://github.com/r3bl-org/r3bl-open-core-archive/tree/main/redux)
//! crate. The use of the Redux pattern, inspired by React, brought with it a lot of
//! overhead both mentally and in terms of performance (since state changes needed to be
//! cloned every time a change was made, and `memcpy` or `clone` is expensive).
//!
//! Versions > `0.3.10` use message passing to communicate between the background threads
//! using the `tokio::mpsc` channel (also async). This is a much easier and more
//! performant model given the nature of the engine and the use cases it has to handle. It
//! also has the benefit of providing an easy way to attach protocol servers in the future
//! over various transport layers (eg: TCP, IPC, etc.); these protocol servers can be used
//! to manage a connection between a process running the engine, and other processes
//! running on the same host or on other hosts, in order to handle use cases like
//! synchronizing rendered output, or state.
//!
//! > Here are some papers outlining the differences between message passing and shared
//! > memory for communication between threads.
//! >
//! > 1. <https://rits.github-pages.ucl.ac.uk/intro-hpchtc/morea/lesson2/reading4.html>
//! > 2. <https://www.javatpoint.com/shared-memory-vs-message-passing-in-operating-system>
//!
//! # I/O devices for full TUI, choice, and REPL
//!
//! [Dependency injection](https://developerlife.com/category/DI) is used to inject the
//! required resources into the `main_event_loop` function. This allows for easy testing
//! and for modularity and extensibility in the codebase. The `r3bl_terminal_async` crate
//! shares the same infrastructure for input and output devices. In fact the
//! [`crate::InputDevice`] and [`crate::OutputDevice`] structs are in the `r3bl_core`
//! crate.
//!
//! 1. The advantage of this approach is that for testing, test fixtures can be used to
//!    perform end-to-end testing of the TUI.
//! 2. This also facilitates some other interesting capabilities, such as preserving all
//!    the state for an application and make it span multiple applets (smaller apps, and
//!    their components). This makes the entire UI composable, and removes the monolithic
//!    approaches to building complex UI and large apps that may consist of many reusable
//!    components and applets.
//! 3. It is easy to swap out implementations of input and output devices away from
//!    `stdin` and `stdout` while preserving all the existing code and functionality. This
//!    can produce some interesting headless apps in the future, where the UI might be
//!    delegated to a window using [eGUI](https://github.com/emilk/egui) or
//!    [iced-rs](https://iced.rs/) or [wgpu](https://wgpu.rs/).
//!
//! # Life of an input event for a Full TUI app
//!
//! There is a clear separation of concerns in this library. To illustrate what goes
//! where, and how things work let's look at an example that puts the main event loop
//! front and center & deals with how the system handles an input event (key press or
//! mouse).
//!
//! - The diagram below shows an app that has 3 [Component]s for (flexbox like) layout &
//!   (CSS like) styling.
//! - Let's say that you run this app (by hypothetically executing `cargo run`).
//! - And then you click or type something in the terminal window that you're running this
//!   app in.
//!
//! ```text
//! ╭─────────────────────────────────────────────────────────────────────────╮
//! │In band input event                                                      │
//! │                                                                         │
//! │  Input ──> [TerminalWindow]                                             │
//! │  Event          ⎫      │                                                │
//! │                 │      ⎩                  [ComponentRegistryMap] stores │
//! │                 │    [App]──────────────> [Component]s at 1st render    │
//! │                 │      │                                                │
//! │                 │      │                                                │
//! │                 │      │          ╭──────> id=1 has focus               │
//! │                 │      │          │                                     │
//! │                 │      ├──> [Component] id=1 ─────╮                     │
//! │                 │      │                          │                     │
//! │                 │      ╰──> [Component] id=2      │                     │
//! │                 │                                 │                     │
//! │          default handler                          │                     │
//! │                 ⎫                                 │                     │
//! │                 ╰─────────────────────────────────╯                     │
//! │                                                                         │
//! ╰─────────────────────────────────────────────────────────────────────────╯
//!
//! ╭────────────────────────────────────────────────────────────╮
//! │Out of band app signal                                      │
//! │                                                            │
//! │  App                                                       │
//! │  Signal ──> [App]                                          │
//! │               ⎫                                            │
//! │               │                                            │
//! │               ╰──────> Update state                        │
//! │                        main thread rerender                │
//! │                               ⎫                            │
//! │                               │                            │
//! │                               ╰─────>[App]                 │
//! │                                        ⎫                   │
//! │                                        ╰────> [Component]s │
//! │                                                            │
//! ╰────────────────────────────────────────────────────────────╯
//! ```
//! <!-- https://asciiflow.com/#/share/eJzdls9OwjAcx1%2Fll565wEEiiQdjPHAwJv6JB7ZDtQWabF3TdgohZC9h9iAeiU%2FDk1gcY8AAXbdh5JdfmkGbT7%2Ff7te1E8SxT1GHh57XQB4eU4k6aOKgkYM65%2B2zhoPG5qnVbpsnTUfa%2FHDQ%2FP3z5NNxuGm7HJ4xJ8C4CDXQV8o12MUKGWVhicohAbrf%2Bpbi4xn0Hqj0GcfeE%2BMkeHOtwdeblufxx2pIGb35npS%2FA9u7CnwRcCPkjg6Y0nJ8g4ULSgeSqh%2BxUe9SCLdwBcSzbFpXAdbQVBok5YTKX7upaZGOgN23KMDIRROGWEE%2FeAlVBdNUqX9tA2QvL5Gcd1NmooNCa3HQKo8%2FEEWwhPZx6GlTBJx4y81QGpr2pN%2BXirRmPcfJosKsY4U8%2BTQ2k%2FxzJWUsmPbWnNBBP7lPYCFAsYE5oAu%2B7kpqBsAcieUh94mBpc3FJ2tx0lqhtv%2B3VFQTZkfGs0dBsKaR0qYtDE3Dx4xHeigpJpGka7eLIpBsmJXB2jD5NdtTIEWre89IC8y2vvUrX9W77p%2Bmg6Zo%2BgU42osD) -->
//!
//! Let's trace the journey through the diagram when an input even is generated by the
//! user (eg: a key press, or mouse event). When the app is started via `cargo run` it
//! sets up a main loop, and lays out all the 3 components, sizes, positions, and then
//! paints them. Then it asynchronously listens for input events (no threads are blocked).
//! When the user types something, this input is processed by the main loop of
//! [`TerminalWindow`].
//!
//! 1. The [Component] that is in [`FlexBox`] with `id=1` currently has focus.
//! 2. When an input event comes in from the user (key press or mouse input) it is routed
//!    to the [App] first, before [`TerminalWindow`] looks at the event.
//! 3. The specificity of the event handler in [App] is higher than the default input
//!    handler in [`TerminalWindow`]. Further, the specificity of the [Component] that
//!    currently has focus is the highest. In other words, the input event gets routed by
//!    the [App] to the [Component] that currently has focus ([Component] id=1 in our
//!    example).
//! 4. Since it is not guaranteed that some [Component] will have focus, this input event
//!    can then be handled by [App], and if not, then by [`TerminalWindow`]'s default
//!    handler. If the default handler doesn't process it, then it is simply ignored.
//! 5. In this journey, as the input event is moved between all these different entities,
//!    each entity decides whether it wants to handle the input event or not. If it does,
//!    then it returns an enum indicating that the event has been consumed, else, it
//!    returns an enum that indicates the event should be propagated.
//!
//! An input event is processed by the main thread in the main event loop. This is a
//! synchronous operation and thus it is safe to mutate state directly in this code path.
//! This is why there is no sophisticated locking in place. You can mutate the state
//! directly in
//! - [`App::app_handle_input_event`]
//! - [`Component::handle_event`]
//!
//! # Life of a signal (aka "out of band event")
//!
//! This is great for input events which are generated by the user using their keyboard or
//! mouse. These are all considered "in-band" events or signals, which have no delay or
//! asynchronous behavior. But what about "out of band" signals or events, which do have
//! unknown delays and asynchronous behaviors? These are important to handle as well. For
//! example, if you want to make an HTTP request, you don't want to block the main thread.
//! In these cases you can use a `tokio::mpsc` channel to send a signal from a background
//! thread to the main thread. This is how you can handle "out of band" events or signals.
//!
//! To provide support for these "out of band" events or signals, the [App] trait has a
//! method called [`App::app_handle_signal`]. This is where you can handle signals that
//! are sent from background threads. One of the arguments to this associated function is
//! a `signal`. This signal needs to contain all the data that is needed for a state
//! mutation to occur on the main thread. So the background thread has the responsibility
//! of doing some work (eg: making an HTTP request), getting some information as a result,
//! and then packaging that information into a `signal` and sending it to the main thread.
//! The main thread then handles this signal by calling the [`App::app_handle_signal`]
//! method. This method can then mutate the state of the [App] and return an
//! [`EventPropagation`] enum indicating whether the main thread should repaint the UI or
//! not.
//!
//! So far we have covered what happens when the [App] receives a signal. Who sends this
//! signal? Who actually creates the `tokio::spawn` task that sends this signal? This can
//! happen anywhere in the [App] and [Component]. Any code that has access to
//! [`GlobalData`] can use the [`crate::send_signal`!] macro to send a signal in a
//! background task. However, only the [App] can receive the signal and do something with
//! it, which is usually apply the signal to update the state and then tell the main
//! thread to repaint the UI.
//!
//! Now that we have seen this whirlwind overview of the life of an input event, let's
//! look at the details in each of the sections below.
//!
//! # The window
//!
//! The main building blocks of a TUI app are:
//! 1. [`TerminalWindow`] - You can think of this as the main "window" of the app. All the
//!    content of your app is painted inside of this "window". And the "window"
//!    conceptually maps to the screen that is contained inside your terminal emulator
//!    program (eg: tilix, Terminal.app, etc). Your TUI app will end up taking up 100% of
//!    the screen space of this terminal emulator. It will also enter raw mode, and paint
//!    to an alternate screen buffer, leaving your original scroll back buffer and history
//!    intact. When you `request_shutdown` this TUI app, it will return your terminal to
//!    where you'd left off. You don't write this code, this is something that you use.
//! 2. [App] - This is where you write your code. You pass in a [App] to the
//!    [`TerminalWindow`] to bootstrap your TUI app. You can just use [App] to build your
//!    app, if it is a simple one & you don't really need any sophisticated layout or
//!    styling. But if you want layout and styling, now we have to deal with [`FlexBox`],
//!    [Component], and [`crate::TuiStyle`].
//!
//! # Layout and styling
//!
//! Inside of your [App] if you want to use flexbox like layout and CSS like styling you
//! can think of composing your code in the following way:
//!
//! 1. [App] is like a box or container. You can attach styles and an id here. The id has
//!    to be unique, and you can reference as many styles as you want from your
//!    stylesheet. Yes, cascading styles are supported! 👏 You can put boxes inside of
//!    boxes. You can make a container box and inside of that you can add other boxes (you
//!    can give them a direction and even relative sizing out of 100%).
//! 2. As you approach the "leaf" nodes of your layout, you will find [Component] trait
//!    objects. These are black boxes which are sized, positioned, and painted _relative_
//!    to their parent box. They get to handle input events and render [`RenderOp`]s into
//!    a [`RenderPipeline`]. This is kind of like virtual DOM in React. This queue of
//!    commands is collected from all the components and ultimately painted to the screen,
//!    for each render! Your app's state is mutable and is stored in the [`GlobalData`]
//!    struct. You can handle out of band events as well using the signal mechanism.
//!
//! # Component registry, event routing, focus mgmt
//!
//! Typically your [App] will look like this:
//!
//! ```
//! #[derive(Default)]
//! pub struct AppMain {
//!   // Might have some app data here as well.
//!   // Or `_phantom: std::marker::PhantomData<(State, AppSignal)>,`
//! }
//! ```
//!
//! As we look at [Component] & [App] more closely we will find a curious thing
//! [`ComponentRegistry`] (that is managed by the [App]). The reason this exists is for
//! input event routing. The input events are routed to the [`Component`] that currently
//! has focus.
//!
//! The [`HasFocus`] struct takes care of this. This provides 2 things:
//!
//! 1. It holds an `id` of a [`FlexBox`] / [`Component`] that has focus.
//! 2. It also holds a map that holds a [`crate::Pos`] for each `id`. This is used to
//!    represent a cursor (whatever that means to your app & component). This cursor is
//!    maintained for each `id`. This allows a separate cursor for each [Component] that
//!    has focus. This is needed to build apps like editors and viewers that maintains a
//!    cursor position between focus switches.
//!
//! Another thing to keep in mind is that the [App] and [`TerminalWindow`] is persistent
//! between re-renders.
//!
//! # Input event specificity
//!
//! [`TerminalWindow`] gives [App] first dibs when it comes to handling input events.
//! [`ComponentRegistry::route_event_to_focused_component`] can be used to route events
//! directly to components that have focus. If it punts handling this event, it will be
//! handled by the default input event handler. And if nothing there matches this event,
//! then it is simply dropped.
//!
//! # Rendering and painting
//!
//! The R3BL TUI engine uses a high performance compositor to render the UI to the
//! terminal. This ensures that only "pixels" that have changed are painted to the
//! terminal. This is done by creating a concept of [`PixelChar`] which represents a
//! single "pixel" in the terminal screen at a given col and row index position. There are
//! only as many [`PixelChar`]s as there are rows and cols in a terminal screen. And the
//! index maps directly to the position of the pixel in the terminal screen.
//!
//! ## Offscreen buffer
//!
//! Here is an example of what a single row of rendered output might look like in a row of
//! the [`OffscreenBuffer`]. This diagram shows each [`PixelChar`] in `row_index: 1` of
//! the [`OffscreenBuffer`]. In this example, there are 80 columns in the terminal screen.
//! This actual log output generated by the TUI engine when logging is enabled.
//!
//! ```text
//! row_index: 1
//! 000 S ░░░░░░░╳░░░░░░░░001 P    'j'→fg‐bg    002 P    'a'→fg‐bg    003 P    'l'→fg‐bg    004 P    'd'→fg‐bg    005 P    'k'→fg‐bg
//! 006 P    'f'→fg‐bg    007 P    'j'→fg‐bg    008 P    'a'→fg‐bg    009 P    'l'→fg‐bg    010 P    'd'→fg‐bg    011 P    'k'→fg‐bg
//! 012 P    'f'→fg‐bg    013 P    'j'→fg‐bg    014 P    'a'→fg‐bg    015 P     '▒'→rev     016 S ░░░░░░░╳░░░░░░░░017 S ░░░░░░░╳░░░░░░░░
//! 018 S ░░░░░░░╳░░░░░░░░019 S ░░░░░░░╳░░░░░░░░020 S ░░░░░░░╳░░░░░░░░021 S ░░░░░░░╳░░░░░░░░022 S ░░░░░░░╳░░░░░░░░023 S ░░░░░░░╳░░░░░░░░
//! 024 S ░░░░░░░╳░░░░░░░░025 S ░░░░░░░╳░░░░░░░░026 S ░░░░░░░╳░░░░░░░░027 S ░░░░░░░╳░░░░░░░░028 S ░░░░░░░╳░░░░░░░░029 S ░░░░░░░╳░░░░░░░░
//! 030 S ░░░░░░░╳░░░░░░░░031 S ░░░░░░░╳░░░░░░░░032 S ░░░░░░░╳░░░░░░░░033 S ░░░░░░░╳░░░░░░░░034 S ░░░░░░░╳░░░░░░░░035 S ░░░░░░░╳░░░░░░░░
//! 036 S ░░░░░░░╳░░░░░░░░037 S ░░░░░░░╳░░░░░░░░038 S ░░░░░░░╳░░░░░░░░039 S ░░░░░░░╳░░░░░░░░040 S ░░░░░░░╳░░░░░░░░041 S ░░░░░░░╳░░░░░░░░
//! 042 S ░░░░░░░╳░░░░░░░░043 S ░░░░░░░╳░░░░░░░░044 S ░░░░░░░╳░░░░░░░░045 S ░░░░░░░╳░░░░░░░░046 S ░░░░░░░╳░░░░░░░░047 S ░░░░░░░╳░░░░░░░░
//! 048 S ░░░░░░░╳░░░░░░░░049 S ░░░░░░░╳░░░░░░░░050 S ░░░░░░░╳░░░░░░░░051 S ░░░░░░░╳░░░░░░░░052 S ░░░░░░░╳░░░░░░░░053 S ░░░░░░░╳░░░░░░░░
//! 054 S ░░░░░░░╳░░░░░░░░055 S ░░░░░░░╳░░░░░░░░056 S ░░░░░░░╳░░░░░░░░057 S ░░░░░░░╳░░░░░░░░058 S ░░░░░░░╳░░░░░░░░059 S ░░░░░░░╳░░░░░░░░
//! 060 S ░░░░░░░╳░░░░░░░░061 S ░░░░░░░╳░░░░░░░░062 S ░░░░░░░╳░░░░░░░░063 S ░░░░░░░╳░░░░░░░░064 S ░░░░░░░╳░░░░░░░░065 S ░░░░░░░╳░░░░░░░░
//! 066 S ░░░░░░░╳░░░░░░░░067 S ░░░░░░░╳░░░░░░░░068 S ░░░░░░░╳░░░░░░░░069 S ░░░░░░░╳░░░░░░░░070 S ░░░░░░░╳░░░░░░░░071 S ░░░░░░░╳░░░░░░░░
//! 072 S ░░░░░░░╳░░░░░░░░073 S ░░░░░░░╳░░░░░░░░074 S ░░░░░░░╳░░░░░░░░075 S ░░░░░░░╳░░░░░░░░076 S ░░░░░░░╳░░░░░░░░077 S ░░░░░░░╳░░░░░░░░
//! 078 S ░░░░░░░╳░░░░░░░░079 S ░░░░░░░╳░░░░░░░░080 S ░░░░░░░╳░░░░░░░░spacer [ 0, 16-80 ]
//! ```
//!
//! When [`RenderOps`] are executed and used to create an [`OffscreenBuffer`] that maps to
//! the size of the terminal window, clipping is performed automatically. This means that
//! it isn't possible to move the caret outside of the bounds of the viewport (terminal
//! window size). And it isn't possible to paint text that is larger than the size of the
//! offscreen buffer. The buffer really represents the current state of the viewport.
//! Scrolling has to be handled by the component itself (an example of this is the editor
//! component).
//!
//! Each [`PixelChar`] can be one of 4 things:
//!
//! 1. **Space**. This is just an empty space. There is no flickering in the TUI engine.
//!    When a new offscreen buffer is created, it is fulled with spaces. Then components
//!    paint over the spaces. Then the diffing algorithm only paints over the pixels that
//!    have changed. You don't have to worry about clearing the screen and painting, which
//!    typically will cause flickering in terminals. You also don't have to worry about
//!    printing empty spaces over areas that you would like to clear between renders. All
//!    of this handled by the TUI engine.
//! 2. **Void**. This is a special pixel that is used to indicate that the pixel should be
//!    ignored. It is used to indicate a wide emoji is to the left somewhere. Most
//!    terminals don't support emojis, so there's a discrepancy between the display width
//!    of the character and its index in the string.
//! 3. **Plain text**. This is a normal pixel which wraps a single character that maybe a
//!    grapheme cluster segment. Styling information is encoded in each
//!    `PixelChar::PlainText` and is used to paint the screen via the diffing algorithm
//!    which is smart enough to "stack" styles that appear beside each other for quicker
//!    rendering in terminals.
//!
//! ## Render pipeline
//!
//! The following diagram provides a high level overview of how apps (that contain
//! components, which may contain components, and so on) are rendered to the terminal
//! screen.
//!
//! ```text
//! ╭──────────────────────────────────╮
//! │ Container                        │
//! │                                  │
//! │ ╭─────────────╮  ╭─────────────╮ │
//! │ │ Col 1       │  │ Col 2       │ │
//! │ │             │  │             │ │
//! │ │             │  │     ────────┼─┼────⟩ RenderPipeline ─────╮
//! │ │             │  │             │ │                          │
//! │ │             │  │             │ │                          │
//! │ │      ───────┼──┼─────────────┼─┼────⟩ RenderPipeline ─╮   │
//! │ │             │  │             │ │                      │   │
//! │ │             │  │             │ │                      ⎩ ✚ ⎩
//! │ │             │  │             │ │       ╭─────────────────────╮
//! │ └─────────────┘  └─────────────┘ │       │                     │
//! │                                  │       │  OffscreenBuffer    │
//! ╰──────────────────────────────────╯       │                     │
//!                                            ╰─────────────────────╯
//! ```
//! <!-- https://asciiflow.com/#/share/eJyrVspLzE1VssorzcnRUcpJrEwtUrJSqo5RqohRsrK0MNaJUaoEsozMTYGsktSKEiAnRunRlD10QzExeUBSwTk%2FryQxMy%2B1SAEHQCglCBBKSXKJAonKUawBeiBHwRDhAAW4oBGSIKoWNDcrYBUkUgulETFtl0JQal5KalFAZkFqDjAicMYUKS4nJaJoaCgdkjExgUkLH9PK2Gl7FLRBJFWMpUqo0ilL4wpirOIklEg4BP3T0oqTi1JT85xK09IgpR%2FcXLohUv1M2MM49FIhFSjVKtUCAEVNQq0%3D) -->
//!
//! Each component produces a [`RenderPipeline`], which is a map of [`ZOrder`] and
//! `Vec<`[`RenderOps`]`>`. [`RenderOps`] are the instructions that are grouped together,
//! such as move the caret to a position, set a color, and paint some text.
//!
//! Inside of each [`RenderOps`] the caret is stateful, meaning that the caret position is
//! remembered after each [`RenderOp`] is executed. However, once a new [`RenderOps`] is
//! executed, the caret position reset just for that [`RenderOps`]. Caret position is not
//! stored globally. You should read more about "atomic paint operations" in the
//! [`RenderOp`] documentation.
//!
//! Once a set of these [`RenderPipeline`]s have been generated, typically after the user
//! enters some input event, and that produces a new state which then has to be rendered,
//! they are combined and painted into an [`OffscreenBuffer`].
//!
//! ## First render
//!
//! The `paint.rs` file contains the `paint` function, which is the entry point for all
//! rendering. Once the first render occurs, the [`OffscreenBuffer`] that is generated is
//! saved to `GlobalSharedState`. The following table shows the various tasks that have to
//! be performed in order to render to an [`OffscreenBuffer`]. There is a different code
//! path that is taken for ANSI text and plain text (which includes `StyledText` which is
//! just plain text with a color). Syntax highlighted text is also just `StyledText`.
//!
//! | UTF-8 | Task                                                                                                           |
//! | ----- | -------------------------------------------------------------------------------------------------------------- |
//! | Y     | convert [`RenderPipeline`] to `List<List<`[`PixelChar`]`>>` ([`OffscreenBuffer`])                            |
//! | Y     | paint each [`PixelChar`] in `List<List<`[`PixelChar`]`>>` to stdout using `OffscreenBufferPainterImplCrossterm` |
//! | Y     | save the `List<List<`[`PixelChar`]`>>` to `GlobalSharedState`                                                 |
//!
//! Currently only `crossterm` is supported for actually painting to the terminal. But
//! this process is really simple making it very easy to swap out other terminal libraries
//! such as `termion`, or even a GUI backend, or some other custom output driver.
//!
//! ## Subsequent render
//!
//! Since the [`OffscreenBuffer`] is cached in `GlobalSharedState` a diff to be performed
//! for subsequent renders. And only those diff chunks are painted to the screen. This
//! ensures that there is no flicker when the content of the screen changes. It also
//! minimizes the amount of work that the terminal or terminal emulator has to do put the
//! [`PixelChar`]s on the screen.
//!
//! # How does the editor component work?
//!
//! The [`EditorComponent`] struct can hold data in its own memory, in addition to relying
//! on the state.
//!
//! - It has an [`EditorEngine`] which holds syntax highlighting information, and
//!   configuration options for the editor (such as multiline mode enabled or not, syntax
//!   highlighting enabled or not, etc.). Note that this information lives outside of the
//!   state.
//! - It also implements the `Component<S, AS>` trait.
//! - However, for the reusable editor component we need the data representing the
//!   document being edited to be stored in the state ([`EditorBuffer`]) and not inside of
//!   the [`EditorComponent`] itself.
//!   - This is why the state must implement the trait [`HasEditorBuffers`] which is where
//!     the document data is stored (the key is the id of the flex box in which the editor
//!     component is placed).
//!   - The [`EditorBuffer`] contains the text content in a `Vec` of `UnicodeString`.
//!     Where each line is represented by a `UnicodeString`. It also contains the scroll
//!     offset, caret position, and file extension for syntax highlighting.
//!
//! In other words,
//!
//! 1. [`EditorEngine`] -> **This goes in [`EditorComponent`]**
//!     - Contains the logic to process keypresses and modify an editor buffer.
//! 2. [`EditorBuffer`] -> **This goes in the `State`**
//!     - Contains the data that represents the document being edited. This contains the
//!       caret (insertion point) position and scroll position. And in the future can
//!       contain lots of other information such as undo / redo history, etc.
//!
//! Here are the connection points with the impl of `Component<S, AS>` in
//! [`EditorComponent`]:
//!
//! 1. `handle_event(global_data: &mut GlobalData<S, AS>, input_event: InputEvent,
//!    has_focus: &mut HasFocus)`
//!     - Can simply relay the arguments to `EditorEngine::apply(state.editor_buffer,
//!       input_event)` which will return another [`EditorBuffer`].
//!     - Return value can be dispatched to the store via an action
//!       `UpdateEditorBuffer(EditorBuffer)`.
//! 2. `render(global_data: &mut GlobalData<S, AS>, current_box: FlexBox, surface_bounds:
//!    SurfaceBounds, has_focus: &mut HasFocus,)`
//!     - Can simply relay the arguments to `EditorEngine::render(state.editor_buffer)`
//!     - Which will return a [`RenderPipeline`].
//!
//! ## Zero-Copy Gap Buffer for High Performance
//!
//! The editor uses a [`ZeroCopyGapBuffer`] for text storage, delivering exceptional
//! performance through careful memory management and zero-copy access patterns.
//!
//! ### Key Performance Features
//!
//! **Zero-copy access**: Read operations return `&str` slices directly into the buffer
//! without allocation or copying:
//! - `as_str()` access: **0.19 ns** (essentially free)
//! - `get_line_content()`: **0.37 ns** (direct pointer return)
//! - Perfect for markdown parsing and text rendering hot paths
//!
//! **Efficient Unicode handling**: All text operations are grapheme-cluster aware:
//! - Handles emojis, combining characters, and complex scripts correctly
//! - Insert operations: **88-408 ns** depending on content complexity
//! - Delete operations: **128-559 ns** for various deletion scenarios
//!
//! **Scalable line management**: Dynamic growth with predictable performance:
//! - Lines start at 256 bytes, grow in 256-byte pages as needed
//! - Adding 100 lines: **~16 ns per line**
//! - Line capacity extension: **12 ns**
//!
//! ### Storage Architecture
//!
//! Each line is stored as a null-padded byte array:
//! ```text
//! Line: [H][e][l][l][o][\\n][\\0][\\0]...[\\0]  // 256 bytes
//! ```
//!
//! This enables:
//! - **In-place editing**: No allocations for small edits
//! - **Safe slicing**: Null padding ensures valid UTF-8 boundaries
//! - **Zero-copy parsing**: Direct `&str` access for syntax highlighting and rendering
//!
//! ### UTF-8 Safety Strategy
//!
//! The implementation uses a **"validate once, trust thereafter"** approach:
//! - **Input validation**: Rust's `&str` type guarantees UTF-8 at API boundaries
//! - **Zero-copy reads**: `unsafe { from_utf8_unchecked() }` in hot paths for maximum
//!   performance
//! - **Debug validation**: Development builds verify UTF-8 invariants
//!
//! This provides both safety (through type system guarantees) and performance (zero
//! validation overhead in production).
//!
//! ### Optimization: Append Detection
//!
//! End-of-line append operations are detected and optimized:
//! - Single character append: **1.48 ns** (68x faster than full rebuild)
//! - Word append: **2.91 ns** (94x faster than full rebuild)
//!
//! This makes typing at the end of lines (the most common editing pattern) extremely
//! fast.
//!
//! ### Learn More
//!
//! For comprehensive implementation details including:
//! - Complete benchmark results across all operation types
//! - Null-padding invariant and safety guarantees
//! - Segment rebuilding strategies
//! - Dynamic growth algorithms
//!
//! See the detailed and extensive [`zero_copy_gap_buffer` module
//! documentation](crate::tui::editor::zero_copy_gap_buffer).
//!
//! # Markdown Parser with R3BL Extensions
//!
//! The TUI includes a high-performance markdown parser built with `nom` that supports
//! both standard markdown syntax and R3BL-specific extensions.
//!
//! ### Key Features
//!
//! **Standard markdown support**:
//! - Headings, bold, italic, links, images
//! - Ordered and unordered lists with smart indentation tracking
//! - Fenced code blocks with syntax highlighting
//! - Inline code, checkboxes
//!
//! **R3BL extensions** for enhanced document metadata:
//! - `@title: <text>` - Document title metadata
//! - `@tags: <tag1>, <tag2>` - Tag lists for categorization
//! - `@authors: <name1>, <name2>` - Author attribution
//! - `@date: <date>` - Publication date
//!
//! **Smart lists** - Multi-line list items with automatic indentation:
//! ```text
//! - This is a list item that spans
//!   multiple lines and maintains proper
//!   indentation automatically
//!   - Nested items work correctly
//! ```
//!
//! ### Architecture and Parser Priority
//!
//! The parser uses a **priority-based composition** strategy where more specific parsers
//! are attempted first:
//!
//! ```text
//! parse_markdown() {
//!   many0(
//!     parse_title_value()          → MdBlock::Title
//!     parse_tags_list()            → MdBlock::Tags
//!     parse_authors_list()         → MdBlock::Authors
//!     parse_date_value()           → MdBlock::Date
//!     parse_heading()              → MdBlock::Heading
//!     parse_smart_list_block()     → MdBlock::SmartList
//!     parse_fenced_code_block()    → MdBlock::CodeBlock
//!     parse_block_text()           → MdBlock::Text (catch-all)
//!   )
//! }
//! ```
//!
//! Within each block, inline fragments are parsed with similar priority:
//! - Bold (`**text**`), italic (`_text_`), inline code (`` `code` ``)
//! - Images (`![alt](url)`), links (`[text](url)`)
//! - Checkboxes (`[ ]`, `[x]`)
//! - Plain text (catch-all for everything else)
//!
//! ### Integration with Syntax Highlighting
//!
//! The parser works seamlessly with the editor's syntax highlighting through several
//! key functions:
//! - [`try_parse_and_highlight`] - Main entry point for parsing and syntax highlighting
//! - [`parse_markdown()`] - Core parser that produces the [`MdDocument`] AST
//! - [`parse_smart_list`] - Specialized parser for multi-line list handling
//! - Code blocks use `syntect` via
//!   [`render_engine()`](crate::editor_engine::engine_public_api::render_engine) for
//!   syntax highlighting
//! - The styled content is rendered through the standard [`RenderPipeline`]
//!
//! ### Performance Characteristics
//!
//! The parser was chosen after extensive benchmarking against alternatives (including
//! `markdown-rs`):
//! - **Streaming parser**: Built with [`nom`](https://developerlife.com/2023/02/20/guide-to-nom-parsing/)
//!   for efficient memory usage
//! - **Low CPU overhead**: No unnecessary allocations or copies
//! - **Proven reliability**: Powers all markdown rendering in `r3bl_tui`
//!
//! ### Learn More
//!
//! For comprehensive implementation details including:
//! - Complete parser composition diagrams
//! - Detailed explanation of the priority system
//! - "Catch-all" parser edge case handling
//! - Full conformance test suite documentation
//!
//! See:
//! - The [`parse_markdown()`] function entry point
//! - The detailed [`md_parser` module documentation](crate::tui::md_parser)
//! - [Blog post: Building a Markdown Parser in Rust](https://developerlife.com/2024/06/28/md-parser-rust-from-r3bl-tui/)
//! - [Video: Markdown Parser Deep Dive](https://youtu.be/SbwvSHZRb1E)
//!
//! # Terminal Multiplexer with VT-100 ANSI Parsing
//!
//! The [`PTYMux`] module provides tmux-like functionality with **universal
//! compatibility** for all programs: TUI applications, interactive shells, and
//! command-line tools.
//!
//! ### Core Capabilities
//!
//! **Per-process virtual terminals**: Each process maintains its own [`OffscreenBuffer`]
//! that acts as a complete virtual terminal, enabling:
//! - **Instant switching** between processes (F1-F9) - no delays or rendering artifacts
//! - **Independent state**: Each process's screen state is fully preserved
//! - **True multiplexing**: All processes update their buffers continuously, only the
//!   active one is displayed
//!
//! **Universal program support**:
//! - Interactive shells (bash, zsh, fish)
//! - TUI applications (vim, htop, any `r3bl_tui` app)
//! - Command-line tools (compilers, build systems)
//! - All programs that use terminal output
//!
//! **Advanced features**:
//! - Dynamic keyboard shortcuts (F-keys based on process count)
//! - Status bar with live process information
//! - OSC sequence support for dynamic terminal titles
//! - Clean resource management (PTY cleanup, raw mode handling)
//!
//! ### Architecture: The Virtual Terminal Pipeline
//!
//! ```text
//! ╭─────────────╮    ╭──────────╮    ╭────────────╮    ╭─────────────────╮
//! │ Child Proc  │────▶ PTY      │────▶ VTE Parser │────▶ OffscreenBuffer │
//! │ (vim, bash) │    │ (bytes)  │    │ (ANSI)     │    │ (virtual        │
//! ╰────▲────────╯    ╰──────────╯    ╰────────────╯    │  terminal)      │
//!      │                                    │          ╰─────────────────╯
//!      │                                    │                  │
//!      │                           ╔════════▼══════╗           │
//!      │                           ║ Perform Trait ║           │
//!      │                           ║ Implementation║           │
//!      │                           ╚═══════════════╝           │
//!      │                                                       │
//!      │                           ╭────────────────╮          │
//!      │                           │ RenderPipeline ◀──────────╯
//!      ╰───────────────────────────│ paint()        │
//!                                  ╰────────────────╯
//! ```
//!
//! ### VT-100 ANSI Parser Implementation
//!
//! The parser provides comprehensive VT100 compliance using the `vte` crate (same as
//! Alacritty):
//!
//! **Supported sequences**:
//! - **CSI sequences**: Cursor movement, text styling, scrolling, device control
//! - **ESC sequences**: Simple escape commands, character set selection
//! - **OSC sequences**: Operating system commands (window titles, etc.)
//! - **Control characters**: Backspace, tab, line feed, carriage return
//! - **SGR codes**: Text styling (colors, bold, italic, underline)
//!
//! **Three-layer architecture** for maintainability:
//! ```text
//! Layer 1: SHIM           → Protocol delegation (char_ops.rs)
//! Layer 2: IMPLEMENTATION → Business logic (impl_char_ops.rs)
//! Layer 3: TESTS          → Conformance validation (test_char_ops.rs)
//! ```
//!
//! This naming convention enables **predictable IDE navigation**: searching for
//! `char_ops` shows you the shim, implementation, and tests all together.
//!
//! **VT100 specification compliance**:
//! - [VT100 User Guide](https://vt100.net/docs/vt100-ug/)
//! - [ANSI X3.64 Standard](https://www.ecma-international.org/wp-content/uploads/ECMA-48_5th_edition_june_1991.pdf)
//! - [XTerm Control Sequences](https://invisible-island.net/xterm/ctlseqs/ctlseqs.html)
//!
//! **Intentionally unimplemented legacy features**: Custom tab stops (HTS, TBC), legacy
//! line control (NEL), and legacy terminal modes (IRM, DECOM) are not implemented as
//! they're primarily used by mainframe terminals and very old applications.
//!
//! ### Usage Example
//!
//! ```rust,no_run
//! use r3bl_tui::core::{pty_mux::{PTYMux, Process}, get_size};
//!
//! #[tokio::main]
//! async fn main() -> miette::Result<()> {
//!     let terminal_size = get_size()?;
//!     let processes = vec![
//!         Process::new("bash", "bash", vec![], terminal_size),
//!         Process::new("editor", "nvim", vec![], terminal_size),
//!         Process::new("monitor", "htop", vec![], terminal_size),
//!     ];
//!
//!     let multiplexer = PTYMux::builder()
//!         .processes(processes)
//!         .build()?;
//!
//!     multiplexer.run().await?;  // F1/F2/F3 to switch, Ctrl+Q to quit
//!     Ok(())
//! }
//! ```
//!
//! ### Learn More
//!
//! For comprehensive implementation details including:
//! - Complete VT-100 sequence support matrix
//! - Virtual terminal state management
//! - Process lifecycle and resource cleanup
//! - VT-100 conformance test suite
//!
//! See the detailed [`pty_mux` module documentation](crate::core::pty_mux) and
//! [`vt_100_ansi_parser` module](crate::core::pty_mux::vt_100_ansi_parser).
//!
//! # Painting the caret
//!
//! Definitions:
//!
//! 1. **Caret** - the block that is visually displayed in a terminal which represents the
//!    insertion point for whatever is in focus. While only one insertion point is
//!    editable for the local user, there may be multiple of them, in which case there has
//!    to be a way to distinguish a local caret from a remote one (this can be done with
//!    bg color).
//!
//! 2. **Cursor** - the global "thing" provided in terminals that shows by blinking
//!    usually where the cursor is. This cursor is moved around and then paint operations
//!    are performed on various different areas in a terminal window to paint the output
//!    of render operations.
//!
//! There are two ways of showing cursors which are quite different (each with very
//! different constraints).
//!
//! 1. Using a global terminal cursor (we don't use this).
//!    - Both [termion::cursor](https://docs.rs/termion/1.5.6/termion/cursor/index.html) and
//!      [crossterm::cursor](https://docs.rs/crossterm/0.25.0/crossterm/cursor/index.html)
//!      support this. The cursor has lots of effects like blink, etc.
//!    - The downside is that there is one global cursor for any given terminal window.
//!      And this cursor is constantly moved around in order to paint anything (eg:
//!      `MoveTo(col, row), SetColor, PaintText(...)` sequence).
//!
//! 2. Paint the character at the cursor with the colors inverted (or some other bg color)
//!    giving the visual effect of a cursor.
//!    - This has the benefit that we can display multiple cursors in the app, since this
//!      is not global, rather it is component specific. For the use case requiring google
//!      docs style multi user editing where multiple cursors need to be shown, this
//!      approach can be used in order to implement that. Each user for eg can get a
//!      different caret background color to differentiate their caret from others.
//!    - The downside is that it isn't possible to blink the cursor or have all the other
//!      "standard" cursor features that are provided by the actual global cursor
//!      (discussed above).
//!
//! # How do modal dialog boxes work?
//!
//! A modal dialog box is different than a normal reusable component. This is because:
//!
//! 1. It paints on top of the entire screen (in front of all other components, in
//!    [`ZOrder::Glass`], and outside of any layouts using [`FlexBox`]es).
//! 2. Is "activated" by a keyboard shortcut (hidden otherwise). Once activated, the user
//!    can accept or cancel the dialog box. And this results in a callback being called
//!    with the result.
//!
//! So this activation trigger must be done at the [App] trait impl level (in the
//! `app_handle_event()` method). Also, when this trigger is detected it has to:
//!
//! 1. When a trigger is detected, send a signal via the channel sender (out of band) so
//!    that it will show when that signal is processed.
//! 2. When the signal is handled, set the focus to the dialog box, and return a
//!    `EventPropagation::ConsumedRerender` which will re-render the UI with the dialog
//!    box on top.
//!
//! There is a question about where does the response from the user (once a dialog is
//! shown) go? This seems as though it would be different in nature from an
//! [`EditorComponent`] but it is the same. Here's why:
//!
//! - The [`EditorComponent`] is always updating its buffer based on user input, and
//!   there's no "handler" for when the user performs some action on the editor. The
//!   editor needs to save all the changes to the buffer to the state. This requires the
//!   trait bound [`HasEditorBuffers`] to be implemented by the state.
//! - The dialog box seems different in that you would think that it doesn't always
//!   updating its state and that the only time we really care about what state the dialog
//!   box has is when the user has accepted something they've typed into the dialog box
//!   and this needs to be sent to the callback function that was passed in when the
//!   component was created. However, due to the reactive nature of the TUI engine, even
//!   before the callback is called (due to the user accepting or cancelling), while the
//!   user is typing things into the dialog box, it has to be updating the state,
//!   otherwise, re-rendering the dialog box won't be triggered and the user won't see
//!   what they're typing. This means that even intermediate information needs to be
//!   recorded into the state via the [`HasDialogBuffers`] trait bound. This will hold
//!   stale data once the dialog is dismissed or accepted, but that's ok since the title
//!   and text should always be set before it is shown.
//!   - **Note**: it might be possible to save this type of intermediate data in
//!     `ComponentRegistry::user_data`. And it is possible for `handle_event()` to return
//!     a `EventPropagation::ConsumedRerender` to make sure that changes are re-rendered.
//!     This approach may have other issues related to having both immutable and mutable
//!     borrows at the same time to some portion of the component registry if one is not
//!     careful.
//!
//! ## Two callback functions
//!
//! When creating a new dialog box component, two callback functions are passed in:
//!
//! 1. `on_dialog_press_handler()` - this will be called if the user choose no, or yes
//!    (with their typed text).
//! 2. `on_dialog_editors_changed_handler()` - this will be called if the user types
//!    something into the editor.
//!
//! ## Async Autocomplete Provider
//!
//! So far we have covered the use case for a simple modal dialog box. The dialog system
//! also supports **async autocomplete capabilities** through the
//! [`DialogEngineConfigOptions`] struct, which allows configuring the dialog in
//! autocomplete mode.
//!
//! In autocomplete mode, you can provide an async autocomplete provider that performs
//! long-running operations such as:
//! - **Network requests** to web services or APIs
//! - **Database queries** for search results
//! - **File system operations** for file/path completion
//! - Any other async operation that generates completion suggestions
//!
//! The autocomplete mode displays an extra "results panel" and uses a different layout
//! (top of screen instead of centered). The same callback functions are used, but the
//! provider can now perform async operations to populate the results.
//!
//! # Lolcat support
//!
//! An implementation of lolcat color wheel is provided. Here's an example.
//!
//! ```
//! use r3bl_tui::*;
//!
//! let mut lolcat = LolcatBuilder::new()
//!   .set_color_change_speed(ColorChangeSpeed::Rapid)
//!   .set_seed(1.0)
//!   .set_seed_delta(1.0)
//!   .build();
//!
//! let content = "Hello, world!";
//! let content_gcs = GCStringOwned::new(content);
//! let lolcat_mut = &mut lolcat;
//! let st = lolcat_mut.colorize_to_styled_texts(&content_gcs);
//! lolcat.next_color();
//! ```
//!
//! This [`crate::Lolcat`] that is returned by `build()` is safe to re-use.
//! - The colors it cycles through are "stable" meaning that once constructed via the
//!   [builder](crate::LolcatBuilder) (which sets the speed, seed, and delta that
//!   determine where the color wheel starts when it is used). For eg, when used in a
//!   dialog box component that re-uses the instance, repeated calls to the `render()`
//!   function of this component will produce the same generated colors over and over
//!   again.
//! - If you want to change where the color wheel "begins", you have to change the speed,
//!   seed, and delta of this [`crate::Lolcat`] instance.
//!
//! # Issues and PRs
//!
//! Please report any issues to the [issue
//! tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues). And if you have any
//! feature requests, feel free to add them there too 👍.
//!
//! <!-- Type references for documentation links -->
//! [App]: crate::App
//! [Component]: crate::Component
//! [TerminalWindow]: crate::TerminalWindow
//! [FlexBox]: crate::FlexBox
//! [Surface]: crate::Surface
//! [HasFocus]: crate::HasFocus
//! [ComponentRegistry]: crate::ComponentRegistry
//! [ComponentRegistryMap]: crate::ComponentRegistryMap
//! [GlobalData]: crate::GlobalData
//! [EventPropagation]: crate::EventPropagation
//!
//! [RenderOp]: crate::RenderOp
//! [RenderOps]: crate::RenderOps
//! [RenderPipeline]: crate::RenderPipeline
//! [OffscreenBuffer]: crate::OffscreenBuffer
//! [PixelChar]: crate::PixelChar
//! [ZOrder]: crate::ZOrder
//!
//! [EditorComponent]: crate::EditorComponent
//! [EditorEngine]: crate::EditorEngine
//! [EditorBuffer]: crate::EditorBuffer
//! [HasEditorBuffers]: crate::HasEditorBuffers
//! [ZeroCopyGapBuffer]: crate::tui::editor::zero_copy_gap_buffer::ZeroCopyGapBuffer
//!
//! [MdDocument]: crate::tui::md_parser::MdDocument
//! [parse_markdown()]: fn@crate::tui::md_parser::parse_markdown::parse_markdown
//! [parse_smart_list]: crate::tui::md_parser::parse_smart_list
//! [try_parse_and_highlight]: crate::tui::syntax_highlighting::md_parser_syn_hi::try_parse_and_highlight
//!
//! [PTYMux]: crate::core::pty_mux::PTYMux
//! [CsiSequence]: crate::core::pty_mux::vt_100_ansi_parser::CsiSequence
//! [EscSequence]: crate::core::pty_mux::vt_100_ansi_parser::EscSequence
//! [SgrCode]: crate::core::ansi::SgrCode
//!
//! [RowIndex]: crate::RowIndex
//! [ColIndex]: crate::ColIndex
//! [ColWidth]: crate::ColWidth
//! [RowHeight]: crate::RowHeight
//! [IndexOps]: crate::IndexOps
//! [LengthOps]: crate::LengthOps
//! [ArrayBoundsCheck]: crate::ArrayBoundsCheck
//! [CursorBoundsCheck]: crate::CursorBoundsCheck
//! [ViewportBoundsCheck]: crate::ViewportBoundsCheck
//! [RangeBoundsExt]: crate::RangeBoundsExt
//! [RangeConvertExt]: crate::RangeConvertExt
//!
//! [ByteIndex]: crate::ByteIndex
//! [SegIndex]: crate::SegIndex
//! [GCStringOwned]: crate::GCStringOwned
//!
//! [HasDialogBuffers]: crate::HasDialogBuffers
//! [DialogEngineConfigOptions]: crate::DialogEngineConfigOptions

// Enable benchmarking for nightly Rust.
#![cfg_attr(test, feature(test))]

// Attach.
pub mod core;
pub mod network_io;
pub mod readline_async;
pub mod tui;

// Re-export.
pub use core::*;
pub use network_io::*;
pub use readline_async::*;
pub use tui::*;
