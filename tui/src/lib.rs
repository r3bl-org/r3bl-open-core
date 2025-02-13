/*
 *   Copyright (c) 2024-2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

// You can get the unicode symbols for the drawings here:
// - <https://symbl.cc/en/unicode/blocks/miscellaneous-symbols-and-arrows/>
// - <https://symbl.cc/en/unicode/blocks/box-drawing/>
// - <https://symbl.cc/en/collections/brackets/>
// - <https://symbl.cc/en/collections/crosses/>

//! # Why R3BL?
//!
//! <img
//! src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/tui/r3bl-tui.svg?raw=true"
//! height="256px">
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
//! style="color:#E10895"> </span><span style="color:#DE0799">&amp;</span><span
//! style="color:#DB069E"> </span><span style="color:#D804A2">s</span><span
//! style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span
//! style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span
//! style="color:#C801B6"> </span><span style="color:#C501B9">o</span><span
//! style="color:#C101BD">f</span><span style="color:#BD01C1"> </span><span
//! style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span
//! style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span
//! style="color:#AA03D2"> </span><span style="color:#A603D5">f</span><span
//! style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span
//! style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span
//! style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span
//! style="color:#890DE8"> </span><span style="color:#850FEB">o</span><span
//! style="color:#8111ED">n</span><span style="color:#7C13EF"> </span><span
//! style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span
//! style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span
//! style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span
//! style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span
//! style="color:#572CFC">r</span><span style="color:#532FFD"> </span><span
//! style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span
//! style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span
//! style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span
//! style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span
//! style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span
//! style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>
//!
//! We are working on building command line apps in Rust which have rich text user
//! interfaces (TUI). We want to lean into the terminal as a place of productivity, and
//! build all kinds of awesome apps for it.
//!
//! 1. 🔮 Instead of just building one app, we are building a library to enable any kind
//!    of rich TUI development w/ a twist: taking concepts that work really well for the
//!    frontend mobile and web development world and re-imagining them for TUI & Rust.
//!
//!    - Taking inspiration from things like [React](https://react.dev/),
//!      [SolidJS](https://www.solidjs.com/),
//!      [Elm](https://guide.elm-lang.org/architecture/),
//!      [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack
//!      Compose](https://developer.android.com/compose),
//!      [JSX](https://ui.dev/imperative-vs-declarative-programming),
//!      [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can
//!      be run in parallel & concurrent via [Tokio](https://crates.io/crates/tokio)).
//!    - Even the thread running the main event loop doesn't block since it is async.
//!    - Using proc macros to create DSLs to implement something inspired by
//!      [CSS](https://www.w3.org/TR/CSS/#css) &
//!      [JSX](https://ui.dev/imperative-vs-declarative-programming).
//!
//! 2. 🌎 We are building apps to enhance developer productivity & workflows.
//!
//!    - The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a
//!      single terminal window). Rather it is to build a set of integrated "apps" (or
//!      "tasks") that run in the same process that renders to one terminal window.
//!    - Inside of this terminal window, we can implement things like "app" switching,
//!      routing, tiling layout, stacking layout, etc. so that we can manage a lot of TUI
//!      apps (which are tightly integrated) that are running in the same process, in the
//!      same window. So you can imagine that all these "app"s have shared application
//!      state. Each "app" may also have its own local application state.
//!    - Here are some examples of the types of "app"s we plan to build (for which this
//!      infrastructure acts as the open source engine):
//!      1. Multi user text editors w/ syntax highlighting.
//!      2. Integrations w/ github issues.
//!      3. Integrations w/ calendar, email, contacts APIs.
//!
//! All the crates in the `r3bl-open-core`
//! [repo](https://github.com/r3bl-org/r3bl-open-core/) provide lots of useful
//! functionality to help you build TUI (text user interface) apps, along w/ general
//! niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉.
//!
//! # Table of contents
//!
//! <!-- TOC -->
//!
//! - [Introduction](#introduction)
//! - [Changelog](#changelog)
//! - [Learn how these crates are built, provide
//!   feedback](#learn-how-these-crates-are-built-provide-feedback)
//! - [Run the demo locally](#run-the-demo-locally)
//! - [Nushell scripts to build, run, test etc.](#nu-shell-scripts-to-build-run-test-etc)
//!   - [Examples to get you started](#examples-to-get-you-started)
//!     - [Video of the demo in action](#video-of-the-demo-in-action)
//!   - [How does layout, rendering, and event handling work in
//!     general?](#how-does-layout-rendering-and-event-handling-work-in-general)
//!   - [Switching from shared memory to message passing architecture after
//!     v0.3.10](#switching-from-shared-memory-to-message-passing-architecture-after-v0310)
//! - [Input and output devices](#input-and-output-devices)
//! - [Life of an input event](#life-of-an-input-event)
//!   - [Life of a signal aka “out of band
//!     event”](#life-of-a-signal-aka-out-of-band-event)
//! - [The window](#the-window)
//! - [Layout and styling](#layout-and-styling)
//!   - [Component, ComponentRegistry, focus management, and event
//!     routing](#component-componentregistry-focus-management-and-event-routing)
//!   - [Input event specificity](#input-event-specificity)
//!   - [Rendering and painting](#rendering-and-painting)
//!     - [Offscreen buffer](#offscreen-buffer)
//!     - [Render pipeline](#render-pipeline)
//!   - [First render](#first-render)
//!   - [Subsequent render](#subsequent-render)
//!   - [How does the editor component work?](#how-does-the-editor-component-work)
//!     - [Painting the caret](#painting-the-caret)
//!   - [How do modal dialog boxes work?](#how-do-modal-dialog-boxes-work)
//!     - [Two callback functions](#two-callback-functions)
//!     - [How to use this dialog to make an HTTP request & pipe the results into a
//!       selection
//!       area?](#how-to-use-this-dialog-to-make-an-http-request--pipe-the-results-into-a-selection-area)
//!     - [How to make HTTP requests](#how-to-make-http-requests)
//!   - [Custom Markdown MD parsing and custom syntax
//!     highlighting](#custom-markdown-md-parsing-and-custom-syntax-highlighting)
//! - [Grapheme support](#grapheme-support)
//! - [Lolcat support](#lolcat-support)
//! - [Issues and PRs](#issues-and-prs)
//!
//! <!-- /TOC -->
//!
//! # Introduction
//!
//! You can build fully async TUI (text user interface) apps with a modern API that brings
//! the best of the web frontend development ideas to TUI apps written in Rust:
//!
//! 1. Reactive & unidirectional data flow architecture from frontend development
//!    ([React](https://react.dev/), [SolidJS](https://www.solidjs.com/),
//!    [Elm](https://guide.elm-lang.org/architecture/),
//!    [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack
//!    Compose](https://developer.android.com/compose)).
//! 2. [Responsive
//!    design](https://developer.mozilla.org/en-US/docs/Learn/CSS/CSS_layout/Responsive_Design)
//!    w/ [CSS](https://www.w3.org/TR/CSS/#css),
//!    [flexbox](https://developer.mozilla.org/en-US/docs/Web/CSS/CSS_flexible_box_layout/Basic_concepts_of_flexbox)
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
//! meaning that you can pick and choose whatever pieces you would like to use w/out
//! having the cognitive load of having to grok all the things in the codebase. Its more
//! like a collection of mostly independent modules that work well w/ each other, but know
//! very little about each other.
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
//! - If you like consuming video content, here's our [YT
//!   channel](https://www.youtube.com/@developerlifecom). Please consider
//!   [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
//! - If you like consuming written content, here's our developer
//!   [site](https://developerlife.com/). Please consider subscribing to our
//!   [newsletter](https://developerlife.com/subscribe.html).
//! - If you have questions, please join our [discord
//!   server](https://discord.gg/8M2ePAevaM).
//!
//! # Run the demo locally
//!
//! Once you've cloned [the repo](https://github.com/r3bl-org/r3bl-open-core) to a folder
//! on your computer, you can run the examples you see in the video with the following
//! commands:
//!
//! ```sh
//! cd tui/examples
//! cargo run --release --example demo
//! ```
//!
//! These examples cover the entire surface area of the TUI API. You can also take a look
//! at the tests in the source (`tui/src/`) as well. A single [`nu`
//! shell](https://www.nushell.sh/) script `run` in **the `tui` sub folder** in the repo
//! allows you to easily build, run, test, and do so much more with the repo.
//!
//! > The `run` script works on Linux, macOS, and Windows. On Linux and macOS, you can
//! > simply run `./run` instead of `nu run`.
//!
//! # Nushell scripts to build, run, test etc.
//!
//! | Command                                | Description                                       |
//! | -------------------------------------- | ------------------------------------------------- |
//! | `nu run help`                          | See all the commands you can pass to the `run` script |
//! | `nu run examples`                      | Run all the examples                              |
//! | `nu run release-examples`              | Run all the examples with the release binary      |
//! | `nu run examples-with-flamegraph-profiling` | This will run the examples and generate a flamegraph at the end so you can see profile the performance of the app. This [video](https://www.youtube.com/watch?v=Sy26IMkOEiM) has a walkthrough of how to use this |
//! | `nu run log`                           | View the log output. This [video](https://www.youtube.com/watch?v=Sy26IMkOEiM) has a walkthrough of how to use this. |
//! | `nu run build`                         | Build                                             |
//! | `nu run clean`                         | Clean                                             |
//! | `nu run test`                          | Run tests                                         |
//! | `nu run clippy`                        | Run clippy                                        |
//! | `nu run docs`                          | Build docs                                        |
//! | `nu run serve-docs`                    | Serve docs over VSCode Remote SSH session         |
//! | `nu run rustfmt`                       | Run rustfmt                                       |
//!
//! The following commands will watch for changes in the source folder and re-run:
//!
//! | Command                                             | Description                        |
//! | --------------------------------------------------- | ---------------------------------- |
//! | `nu run watch-all-tests`                            | Watch all test                     |
//! | `nu run watch-one-test <test_name>`                 | Watch one test                     |
//! | `nu run watch-clippy`                               | Watch clippy                       |
//! | `nu run watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
//!
//! There's also a `run` script at the **top level folder** of the repo. It is intended to
//! be used in a CI/CD environment w/ all the required arguments supplied or in
//! interactive mode, where the user will be prompted for input.
//!
//! | Command                       | Description                        |
//! | ----------------------------- | ---------------------------------- |
//! | `nu run all`                  | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD |
//! | `nu run build-full`           | This will build all the crates in the Rust workspace. And it will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
//! | `nu run install-cargo-tools`  | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`, `flamegraph` will all be installed in one go) |
//! | `nu run check-licenses`       | Use `cargo-deny` to audit all licenses used in the Rust workspace |
//!
//! Here are some framework highlights:
//!
//! - An easy to use and approachable API that is inspired by React, JSX, CSS, Elm. Lots
//!   of components and things are provided for you so you don't have to build them from
//!   scratch. This is a full featured component library including:
//!   - Elm like architecture w/ unidirectional data flow. The state is mutable. Async
//!     middleware functions are supported, and they communicate w/ the main thread and
//!     the [App] using an async `tokio::mpsc` channel and signals.
//!   - CSS like declarative styling engine.
//!   - CSS like flexbox like declarative layout engine which is fully responsive. You can
//!     resize your terminal window and everything will be laid out correctly.
//!   - A terminal independent underlying rendering and painting engine (can use crossterm
//!     or termion or whatever you want).
//!   - Markdown text editor w/ syntax highlighting support, metadata (tags, title,
//!     author, date), smart lists. This uses a custom Markdown parser and custom syntax
//!     highlighter. Syntax highlighting for code blocks is provided by the syntect crate.
//!   - Modal dialog boxes. And autocompletion dialog boxes.
//!   - Lolcat (color gradients) implementation w/ a rainbow color-wheel palette. All the
//!     color output is sensitive to the capabilities of the terminal. Colors are
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
//! # How does layout, rendering, and event handling work in general?
//!
//! ```text
//! ╭───────────────────────────────────────────────╮
//! │                                               │
//! │  main.rs                                      │
//! │                          ╭──────────────────╮ │
//! │  GlobalData ────────────〉│ window size      │ │
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
//!   original scroll back buffer and history intact. When you exit this TUI app, it will
//!   return your terminal to where you'd left off.
//! - The [main_event_loop] is where many global structs live which are shared across the
//!   lifetime of your app. These include the following:
//!   - [HasFocus]
//!   - [ComponentRegistryMap]
//!   - [GlobalData] which contains the following
//!     - Global application state. This is mutable. Whenever an input event or signal is
//!       processed the entire [App] gets re-rendered. This is the unidirectional data
//!       flow architecture inspired by React and Elm.
//! - Your [App] trait impl is the main entry point for laying out the entire application.
//!   Before the first render, the [App] is initialized (via a call to [App::app_init]),
//!   and is responsible for creating all the [Component]s that it uses, and saving them
//!   to the [ComponentRegistryMap].
//!   - State is stored in many places. Globally at the [GlobalData] level, and also in
//!     [App], and also in [Component].
//! - This sets everything up so that [App::app_render], [App::app_handle_input_event],
//!   and [App::app_handle_signal] can be called at a later time.
//! - The [App::app_render] method is responsible for creating the layout by using
//!   [Surface] and [FlexBox] to arrange whatever [Component]'s are in the
//!   [ComponentRegistryMap].
//! - The [App::app_handle_input_event] method is responsible for handling events that are
//!   sent to the [App] trait when user input is detected from the keyboard or mouse.
//!   Similarly the [App::app_handle_signal] deals with signals that are sent from
//!   background threads (Tokio tasks) to the main thread, which then get routed to the
//!   [App] trait object. Typically this will then get routed to the [Component] that
//!   currently has focus.
//!
//! # Switching from shared memory to message passing architecture after v0.3.10
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
//! # Input and output devices
//!
//! [Dependency injection](https://developerlife.com/category/DI) is used to inject the
//! required resources into the `main_event_loop` function. This allows for easy testing
//! and for modularity and extensibility in the codebase. The `r3bl_terminal_async` crate
//! shares the same infrastructure for input and output devices. In fact the
//! [r3bl_core::InputDevice] and [r3bl_core::OutputDevice] structs are in the `r3bl_core`
//! crate.
//!
//! 1. The advantage of this approach is that for testing, test fixtures can be used to
//!    perform end to end testing of the TUI.
//! 2. This also facilitates some other interesting capabilities, such as preserving all
//!    the state for an application and make it span multiple applets (smaller apps, and
//!    their components). This makes the entire UI composable, and removes the monolithic
//!    approaches to building complex UI and large apps that may be comprised of many
//!    reusable components and applets.
//! 3. It is easy to swap out implementations of input and output devices away from
//!    `stdin` and `stdout` while preserving all the existing code and functionality. This
//!    can produce some interesting headless apps in the future, where the UI might be
//!    delegated to a window using [eGUI](https://github.com/emilk/egui) or
//!    [iced-rs](https://iced.rs/) or [wgpu](https://wgpu.rs/).
//!
//! # Life of an input event
//!
//! There is a clear separation of concerns in this module. To illustrate what goes where,
//! and how things work let's look at an example that puts the main event loop front and
//! center & deals w/ how the system handles an input event (key press or mouse).
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
//! │  Input ──〉 [TerminalWindow]                                             │
//! │  Event          ⎫      │                                                │
//! │                 │      ⎩                  [ComponentRegistryMap] stores │
//! │                 │    [App]──────────────〉 [Component]s at 1st render    │
//! │                 │      │                                                │
//! │                 │      │                                                │
//! │                 │      │          ╭──────〉 id=1 has focus               │
//! │                 │      │          │                                     │
//! │                 │      ├──〉 [Component] id=1 ─────╮                     │
//! │                 │      │                          │                     │
//! │                 │      ╰──〉 [Component] id=2      │                     │
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
//! │  Signal ──〉 [App]                                          │
//! │               ⎫                                            │
//! │               │                                            │
//! │               ╰──────〉 Update state                        │
//! │                        main thread rerender                │
//! │                               ⎫                            │
//! │                               │                            │
//! │                               ╰─────〉[App]                 │
//! │                                        ⎫                   │
//! │                                        ╰────〉 [Component]s │
//! │                                                            │
//! ╰────────────────────────────────────────────────────────────╯
//! <!-- https://asciiflow.com/#/share/eJzdls9OwjAcx1%2Fll565wEEiiQdjPHAwJv6JB7ZDtQWabF3TdgohZC9h9iAeiU%2FDk1gcY8AAXbdh5JdfmkGbT7%2Ff7te1E8SxT1GHh57XQB4eU4k6aOKgkYM65%2B2zhoPG5qnVbpsnTUfa%2FHDQ%2FP3z5NNxuGm7HJ4xJ8C4CDXQV8o12MUKGWVhicohAbrf%2Bpbi4xn0Hqj0GcfeE%2BMkeHOtwdeblufxx2pIGb35npS%2FA9u7CnwRcCPkjg6Y0nJ8g4ULSgeSqh%2BxUe9SCLdwBcSzbFpXAdbQVBok5YTKX7upaZGOgN23KMDIRROGWEE%2FeAlVBdNUqX9tA2QvL5Gcd1NmooNCa3HQKo8%2FEEWwhPZx6GlTBJx4y81QGpr2pN%2BXirRmPcfJosKsY4U8%2BTQ2k%2FxzJWUsmPbWnNBBP7lPYCFAsYE5oAu%2B7kpqBsAcieUh94mBpc3FJ2tx0lqhtv%2B3VFQTZkfGs0dBsKaR0qYtDE3Dx4xHeigpJpGka7eLIpBsmJXB2jD5NdtTIEWre89IC8y2vvUrX9W77p%2Bmg6Zo%2BgU42osD) -->
//!
//! Let's trace the journey through the diagram when an input even is generated by the
//! user (eg: a key press, or mouse event). When the app is started via `cargo run` it
//! sets up a main loop, and lays out all the 3 components, sizes, positions, and then
//! paints them. Then it asynchronously listens for input events (no threads are blocked).
//! When the user types something, this input is processed by the main loop of
//! [TerminalWindow].
//!
//! 1. The [Component] that is in [FlexBox] w/ `id=1` currently has focus.
//! 2. When an input event comes in from the user (key press or mouse input) it is routed
//!    to the [App] first, before [TerminalWindow] looks at the event.
//! 3. The specificity of the event handler in [App] is higher than the default input
//!    handler in [TerminalWindow]. Further, the specificity of the [Component] that
//!    currently has focus is the highest. In other words, the input event gets routed by
//!    the [App] to the [Component] that currently has focus ([Component] id=1 in our
//!    example).
//! 4. Since it is not guaranteed that some [Component] will have focus, this input event
//!    can then be handled by [App], and if not, then by [TerminalWindow]'s default
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
//! - [App::app_handle_input_event]
//! - [Component::handle_event]
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
//! method called [App::app_handle_signal]. This is where you can handle signals that are
//! sent from background threads. One of the arguments to this associated function is a
//! `signal`. This signal needs to contain all the data that is needed for a state
//! mutation to occur on the main thread. So the background thread has the responsibility
//! of doing some work (eg: making an HTTP request), getting some information as a result,
//! and then packaging that information into a `signal` and sending it to the main thread.
//! The main thread then handles this signal by calling the [App::app_handle_signal]
//! method. This method can then mutate the state of the [App] and return an
//! [EventPropagation] enum indicating whether the main thread should repaint the UI or
//! not.
//!
//! So far we have covered what happens when the [App] receives a signal. Who sends this
//! signal? Who actually creates the `tokio::spawn` task that sends this signal? This can
//! happen anywhere in the [App] and [Component]. Any code that has access to [GlobalData]
//! can use the [r3bl_core::send_signal!] macro to send a signal in a background task.
//! However, only the [App] can receive the signal and do something with it, which is
//! usually apply the signal to update the state and then tell the main thread to repaint
//! the UI.
//!
//! Now that we have seen this whirlwind overview of the life of an input event, let's
//! look at the details in each of the sections below.
//!
//! # The window
//!
//! The main building blocks of a TUI app are:
//! 1. [TerminalWindow] - You can think of this as the main "window" of the app. All the
//!    content of your app is painted inside of this "window". And the "window"
//!    conceptually maps to the screen that is contained inside your terminal emulator
//!    program (eg: tilix, Terminal.app, etc). Your TUI app will end up taking up 100% of
//!    the screen space of this terminal emulator. It will also enter raw mode, and paint
//!    to an alternate screen buffer, leaving your original scroll back buffer and history
//!    intact. When you exit this TUI app, it will return your terminal to where you'd
//!    left off. You don't write this code, this is something that you use.
//! 2. [App] - This is where you write your code. You pass in a [App] to the
//!    [TerminalWindow] to bootstrap your TUI app. You can just use [App] to build your
//!    app, if it is a simple one & you don't really need any sophisticated layout or
//!    styling. But if you want layout and styling, now we have to deal with [FlexBox],
//!    [Component], and [r3bl_core::TuiStyle].
//!
//! # Layout and styling
//!
//! Inside of your [App] if you want to use flexbox like layout and CSS like styling you
//! can think of composing your code in the following way:
//!
//! 1.  [App] is like a box or container. You can attach styles and an id here. The id has
//!     to be unique, and you can reference as many styles as you want from your
//!     stylesheet. Yes, cascading styles are supported! 👏 You can put boxes inside of
//!     boxes. You can make a container box and inside of that you can add other boxes
//!     (you can give them a direction and even relative sizing out of 100%).
//! 2.  As you approach the "leaf" nodes of your layout, you will find [Component] trait
//!     objects. These are black boxes which are sized, positioned, and painted _relative_
//!     to their parent box. They get to handle input events and render [RenderOp]s into a
//!     [RenderPipeline]. This is kind of like virtual DOM in React. This queue of
//!     commands is collected from all the components and ultimately painted to the
//!     screen, for each render! Your app's state is mutable and is stored in the
//!     [GlobalData] struct. You can handle out of band events as well using the signal
//!     mechanism.
//!
//! # Component, ComponentRegistry, focus management, and event routing
//!
//! Typically your [App] will look like this:
//!
//! ```rust
//! #[derive(Default)]
//! pub struct AppMain {
//!   // Might have some app data here as well.
//!   // Or `_phantom: std::marker::PhantomData<(State, AppSignal)>,`
//! }
//! ```
//!
//! As we look at [Component] & [App] more closely we will find a curious thing
//! [ComponentRegistry] (that is managed by the [App]). The reason this exists is for
//! input event routing. The input events are routed to the [Component] that currently has
//! focus.
//!
//! The [HasFocus] struct takes care of this. This provides 2 things:
//!
//! 1.  It holds an `id` of a [FlexBox] / [Component] that has focus.
//! 2.  It also holds a map that holds a [r3bl_core::Pos] for each `id`. This is used to
//!     represent a cursor (whatever that means to your app & component). This cursor is
//!     maintained for each `id`. This allows a separate cursor for each [Component] that
//!     has focus. This is needed to build apps like editors and viewers that maintains a
//!     cursor position between focus switches.
//!
//! Another thing to keep in mind is that the [App] and [TerminalWindow] is persistent
//! between re-renders.
//!
//! # Input event specificity
//!
//! [TerminalWindow] gives [App] first dibs when it comes to handling input events.
//! [ComponentRegistry::route_event_to_focused_component] can be used to route events
//! directly to components that have focus. If it punts handling this event, it will be
//! handled by the default input event handler. And if nothing there matches this event,
//! then it is simply dropped.
//!
//! # Rendering and painting
//!
//! The R3BL TUI engine uses a high performance compositor to render the UI to the
//! terminal. This ensures that only "pixels" that have changed are painted to the
//! terminal. This is done by creating a concept of `PixelChar` which represents a single
//! "pixel" in the terminal screen at a given col and row index position. There are only
//! as many `PixelChar`s as there are rows and cols in a terminal screen. And the index
//! maps directly to the position of the pixel in the terminal screen.
//!
//! ## Offscreen buffer
//!
//! Here is an example of what a single row of rendered output might look like in a row of
//! the `OffscreenBuffer`. This diagram shows each `PixelChar` in `row_index: 1` of the
//! `OffscreenBuffer`. In this example, there are 80 columns in the terminal screen. This
//! actual log output generated by the TUI engine when logging is enabled.
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
//! When `RenderOps` are executed and used to create an `OffscreenBuffer` that maps to the
//! size of the terminal window, clipping is performed automatically. This means that it
//! isn't possible to move the caret outside of the bounds of the viewport (terminal
//! window size). And it isn't possible to paint text that is larger than the size of the
//! offscreen buffer. The buffer really represents the current state of the viewport.
//! Scrolling has to be handled by the component itself (an example of this is the editor
//! component).
//!
//! Each `PixelChar` can be one of 4 things:
//!
//! 1. **Space**. This is just an empty space. There is no flickering in the TUI engine.
//!    When a new offscreen buffer is created, it is fulled w/ spaces. Then components
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
//! <!-- https://asciiflow.com/#/share/eJyrVspLzE1VssorzcnRUcpJrEwtUrJSqo5RqohRsrK0MNaJUaoEsozMTYGsktSKEiAnRunRlD10QzExeUBSwTk%2FryQxMy%2B1SAEHQCglCBBKSXKJAonKUawBeiBHwRDhAAW4oBGSIKoWNDcrYBUkUgulETFtl0JQal5KalFAZkFqDjAicMYUKS4nJaJoaCgdkjExgUkLH9PK2Gl7FLRBJFWMpUqo0ilL4wpirOIklEg4BP3T0oqTi1JT85xK09IgpR%2FcXLohUv1M2MM49FIhFSjVKtUCAEVNQq0%3D) -->
//!
//! Each component produces a `RenderPipeline`, which is a map of `ZOrder` and
//! `Vec<RenderOps>`. `RenderOps` are the instructions that are grouped together, such as
//! move the caret to a position, set a color, and paint some text.
//!
//! Inside of each `RenderOps` the caret is stateful, meaning that the caret position is
//! remembered after each `RenderOp` is executed. However, once a new `RenderOps` is
//! executed, the caret position reset just for that `RenderOps`. Caret position is not
//! stored globally. You should read more about "atomic paint operations" in the
//! `RenderOp` documentation.
//!
//! Once a set of these `RenderPipeline`s have been generated, typically after the user
//! enters some input event, and that produces a new state which then has to be rendered,
//! they are combined and painted into an `OffscreenBuffer`.
//!
//! ## First render
//!
//! The `paint.rs` file contains the `paint` function, which is the entry point for all
//! rendering. Once the first render occurs, the `OffscreenBuffer` that is generated is
//! saved to `GlobalSharedState`. The following table shows the various tasks that have to
//! be performed in order to render to an `OffscreenBuffer`. There is a different code
//! path that is taken for ANSI text and plain text (which includes `StyledText` which is
//! just plain text with a color). Syntax highlighted text is also just `StyledText`.
//!
//! | UTF-8 | Task                                                                                                    |
//! | ----- | ------------------------------------------------------------------------------------------------------- |
//! | Y     | convert `RenderPipeline` to `List<List<PixelChar>>` (`OffscreenBuffer`)                                 |
//! | Y     | paint each `PixelChar` in `List<List<PixelChar>>` to stdout using `OffscreenBufferPainterImplCrossterm` |
//! | Y     | save the `List<List<PixelChar>>` to `GlobalSharedState`                                                 |
//!
//! Currently only `crossterm` is supported for actually painting to the terminal. But
//! this process is really simple making it very easy to swap out other terminal libraries
//! such as `termion`, or even a GUI backend, or some other custom output driver.
//!
//! ## Subsequent render
//!
//! Since the `OffscreenBuffer` is cached in `GlobalSharedState` a diff to be performed
//! for subsequent renders. And only those diff chunks are painted to the screen. This
//! ensures that there is no flicker when the content of the screen changes. It also
//! minimizes the amount of work that the terminal or terminal emulator has to do put the
//! `PixelChar`s on the screen.
//!
//! # How does the editor component work?
//!
//! The `EditorComponent` struct can hold data in its own memory, in addition to relying
//! on the state.
//!
//! - It has an `EditorEngine` which holds syntax highlighting information, and
//!   configuration options for the editor (such as multiline mode enabled or not, syntax
//!   highlighting enabled or not, etc.). Note that this information lives outside of the
//!   state.
//! - It also implements the `Component<S, AS>` trait.
//! - However, for the reusable editor component we need the data representing the
//!   document being edited to be stored in the state (`EditorBuffer`) and not inside of
//!   the `EditorComponent` itself.
//!   - This is why the state must implement the trait `HasEditorBuffers` which is where
//!     the document data is stored (the key is the id of the flex box in which the editor
//!     component is placed).
//!   - The `EditorBuffer` contains the text content in a `Vec` of `UnicodeString`. Where
//!     each line is represented by a `UnicodeString`. It also contains the scroll offset,
//!     caret position, and file extension for syntax highlighting.
//!
//! In other words,
//!
//! 1.  `EditorEngine` -> **This goes in `EditorComponent`**
//!     - Contains the logic to process keypresses and modify an editor buffer.
//! 2.  `EditorBuffer` -> **This goes in the `State`**
//!     - Contains the data that represents the document being edited. This contains the
//!       caret (insertion point) position and scroll position. And in the future can
//!       contain lots of other information such as undo / redo history, etc.
//!
//! Here are the connection points w/ the impl of `Component<S, AS>` in `EditorComponent`:
//!
//! 1.  `handle_event(global_data: &mut GlobalData<S, AS>, input_event: InputEvent,
//!     has_focus: &mut HasFocus)`
//!     - Can simply relay the arguments to `EditorEngine::apply(state.editor_buffer,
//!       input_event)` which will return another `EditorBuffer`.
//!     - Return value can be dispatched to the store via an action
//!       `UpdateEditorBuffer(EditorBuffer)`.
//! 2.  `render(global_data: &mut GlobalData<S, AS>, current_box: FlexBox, surface_bounds:
//!     SurfaceBounds, has_focus: &mut HasFocus,)`
//!     - Can simply relay the arguments to `EditorEngine::render(state.editor_buffer)`
//!     - Which will return a `RenderPipeline`.
//!
//! # Painting the caret
//!
//! Definitions:
//!
//! 1. **`Caret`** - the block that is visually displayed in a terminal which represents
//!    the insertion point for whatever is in focus. While only one insertion point is
//!    editable for the local user, there may be multiple of them, in which case there has
//!    to be a way to distinguish a local caret from a remote one (this can be done w/ bg
//!    color).
//!
//! 2. **`Cursor`** - the global "thing" provided in terminals that shows by blinking
//!    usually where the cursor is. This cursor is moved around and then paint operations
//!    are performed on various different areas in a terminal window to paint the output
//!    of render operations.
//!
//! There are two ways of showing cursors which are quite different (each w/ very
//! different constraints).
//!
//! 1. Using a global terminal cursor (we don't use this).
//!    - Both [termion::cursor](https://docs.rs/termion/1.5.6/termion/cursor/index.html)
//!      and
//!      [crossterm::cursor](https://docs.rs/crossterm/0.25.0/crossterm/cursor/index.html)
//!      support this. The cursor has lots of effects like blink, etc.
//!    - The downside is that there is one global cursor for any given terminal window.
//!      And this cursor is constantly moved around in order to paint anything (eg:
//!      `MoveTo(col, row), SetColor, PaintText(...)` sequence).
//!
//! 2. Paint the character at the cursor w/ the colors inverted (or some other bg color)
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
//!    ZOrder::Glass, and outside of any layouts using `FlexBox`es).
//! 2. Is "activated" by a keyboard shortcut (hidden otherwise). Once activated, the user
//!    can accept or cancel the dialog box. And this results in a callback being called w/
//!    the result.
//!
//! So this activation trigger must be done at the `App` trait impl level (in the
//! `app_handle_event()` method). Also, when this trigger is detected it has to:
//!
//! 1. When a trigger is detected, send a signal via the channel sender (out of band) so
//!    that it will show when that signal is processed.
//! 2. When the signal is handled, set the focus to the dialog box, and return a
//!    `EventPropagation::ConsumedRerender` which will re-render the UI w/ the dialog box
//!    on top.
//!
//! There is a question about where does the response from the user (once a dialog is
//! shown) go? This seems as though it would be different in nature from an
//! `EditorComponent` but it is the same. Here's why:
//!
//! - The `EditorComponent` is always updating its buffer based on user input, and there's
//!   no "handler" for when the user performs some action on the editor. The editor needs
//!   to save all the changes to the buffer to the state. This requires the trait bound
//!   `HasEditorBuffers` to be implemented by the state.
//! - The dialog box seems different in that you would think that it doesn't always
//!   updating its state and that the only time we really care about what state the dialog
//!   box has is when the user has accepted something they've typed into the dialog box
//!   and this needs to be sent to the callback function that was passed in when the
//!   component was created. However, due to the reactive nature of the TUI engine, even
//!   before the callback is called (due to the user accepting or cancelling), while the
//!   user is typing things into the dialog box, it has to be updating the state,
//!   otherwise, re-rendering the dialog box won't be triggered and the user won't see
//!   what they're typing. This means that even intermediate information needs to be
//!   recorded into the state via the `HasDialogBuffers` trait bound. This will hold stale
//!   data once the dialog is dismissed or accepted, but that's ok since the title and
//!   text should always be set before it is shown.
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
//! 1. `on_dialog_press_handler()` - this will be called if the user choose no, or yes (w/
//!    their typed text).
//! 2. `on_dialog_editors_changed_handler()` - this will be called if the user types
//!    something into the editor.
//!
//! ## How to use this dialog to make an HTTP request & pipe the results into a selection area?
//!
//! So far we have covered the use case for a simple modal dialog box. In order to provide
//! auto-completion capabilities, via some kind of web service, there needs to be a
//! slightly more complex version of this. This is where the `DialogEngineConfigOptions`
//! struct comes in. It allows us to create a dialog component and engine to be configured
//! w/ the appropriate mode - simple or autocomplete.
//!
//! In autocomplete mode, an extra "results panel" is displayed, and the layout of the
//! dialog is different on the screen. Instead of being in the middle of the screen, it
//! starts at the top of the screen. The callbacks are the same.
//!
//! # How to make HTTP requests
//!
//! Crates like `reqwest` and `hyper` (which is part of Tokio) will work. Here's a link
//! that shows the pros and cons of using each:
//! - [Rust Crates for Networking and HTTP
//!   Foundations](https://blessed.rs/crates#section-networking-subsection-http-foundations)
//!
//! ## Custom Markdown (MD) parsing and custom syntax highlighting
//!
//! The code for parsing and syntax highlighting is in [try_parse_and_highlight].
//!
//! A custom Markdown parser is provided to provide some extensions over the standard
//! Markdown syntax. The parser code is in the [parse_markdown()] function. Here are some
//! of the extensions:
//! - Metadata title (eg: `@title: <title_text>`). Similar to front matter.
//! - Metadata tags (eg: `@tags: <tag1>, <tag2>`).
//! - Metadata authors (eg: `@authors: <author1>, <author2>`).
//! - Metadata date (eg: `@date: <date>`).
//!
//! Some other changes are adding support for smart lists. These are lists that span
//! multiple lines of text. And indentation levels are tracked. This information is used
//! to render the list items in a way that is visually appealing.
//! - The code for parsing smart lists is in [parse_smart_list].
//! - The code for syntax highlighting is in [StyleUSSpanLines::from_document].
//!
//! Also, `syntect` crate is still used by the editor component
//! [EditorEngineApi::render_engine] to syntax highlight the text inside code blocks of
//! Markdown documents.
//!
//! An alternative approach to doing this was considered using the crate `markdown-rs`,
//! but we decided to implement our own parser using
//! [`nom`](https://developerlife.com/2023/02/20/guide-to-nom-parsing/) since it was
//! streaming and used less CPU and memory.
//!
//! # Grapheme support
//!
//! Unicode is supported (to an extent). There are some caveats. The
//! [r3bl_core::UnicodeString] struct has lots of great information on this graphemes and
//! what is supported and what is not.
//!
//! # Lolcat support
//!
//! An implementation of lolcat color wheel is provided. Here's an example.
//!
//! ```rust
//! use r3bl_core::*;
//! use r3bl_tui::*;
//!
//! let mut lolcat = LolcatBuilder::new()
//!   .set_color_change_speed(ColorChangeSpeed::Rapid)
//!   .set_seed(1.0)
//!   .set_seed_delta(1.0)
//!   .build();
//!
//! let content = "Hello, world!";
//! let content_us = UnicodeString::new(content);
//! let lolcat_mut = &mut lolcat;
//! let st = lolcat_mut.colorize_to_styled_texts(&content_us);
//! lolcat.next_color();
//! ```
//!
//! This [r3bl_core::Lolcat] that is returned by `build()` is safe to re-use.
//! - The colors it cycles through are "stable" meaning that once constructed via the
//!   [builder](r3bl_core::LolcatBuilder) (which sets the speed, seed, and delta that
//!   determine where the color wheel starts when it is used). For eg, when used in a
//!   dialog box component that re-uses the instance, repeated calls to the `render()`
//!   function of this component will produce the same generated colors over and over
//!   again.
//! - If you want to change where the color wheel "begins", you have to change the speed,
//!   seed, and delta of this [r3bl_core::Lolcat] instance.
//!
//! # Issues and PRs
//!
//! Please report any issues to the [issue
//! tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues). And if you have any
//! feature requests, feel free to add them there too 👍.

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]
#![feature(trivial_bounds)]
#![feature(let_chains)]
#![allow(clippy::literal_string_with_formatting_args)]

// Attach.
pub mod tui;

// Re-export.
pub use tui::*;
