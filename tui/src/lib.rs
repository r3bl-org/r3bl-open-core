/*
 *   Copyright (c) 2022 R3BL LLC
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

//! # Context
//!
//! <!-- R3BL TUI library & suite of apps focused on developer productivity -->
//!
//! <span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span style="color:#F92363">
//! </span><span style="color:#F82067">T</span><span style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span style="color:#F31874">
//! </span><span style="color:#F11678">l</span><span style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span style="color:#E10895">
//! </span><span style="color:#DE0799">&amp;</span><span style="color:#DB069E">
//! </span><span style="color:#D804A2">s</span><span style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span style="color:#C801B6">
//! </span><span style="color:#C501B9">o</span><span style="color:#C101BD">f</span><span style="color:#BD01C1">
//! </span><span style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span style="color:#AA03D2">
//! </span><span style="color:#A603D5">f</span><span style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span style="color:#890DE8">
//! </span><span style="color:#850FEB">o</span><span style="color:#8111ED">n</span><span style="color:#7C13EF">
//! </span><span style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span style="color:#572CFC">r</span><span style="color:#532FFD">
//! </span><span style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>
//!
//! We are working on building command line apps in Rust which have rich text user interfaces (TUI).
//! We want to lean into the terminal as a place of productivity, and build all kinds of awesome
//! apps for it.
//!
//! 1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
//!    development w/ a twist: taking concepts that work really well for the frontend mobile and web
//!    development world and re-imagining them for TUI & Rust.
//!
//!    - Taking things like React, JSX, CSS, and Redux, but making everything async (they can be run
//!      in parallel & concurrent via Tokio).
//!    - Even the thread running the main event loop doesn't block since it is async.
//!    - Using proc macros to create DSLs to implement CSS & JSX.
//!
//! 2. 🌎 We are building apps to enhance developer productivity & workflows.
//!
//!    - The idea here is not to rebuild tmux in Rust (separate processes mux'd onto a single
//!      terminal window). Rather it is to build a set of integrated "apps" (or "tasks") that run in
//!      the same process that renders to one terminal window.
//!    - Inside of this terminal window, we can implement things like "app" switching, routing,
//!      tiling layout, stacking layout, etc. so that we can manage a lot of TUI apps (which are
//!      tightly integrated) that are running in the same process, in the same window. So you can
//!      imagine that all these "app"s have shared application state (that is in a Redux store).
//!      Each "app" may also have its own Redux store.
//!    - Here are some examples of the types of "app"s we want to build:
//!      1. multi user text editors w/ syntax highlighting
//!      2. integrations w/ github issues
//!      3. integrations w/ calendar, email, contacts APIs
//!
//! This crate provides lots of useful functionality to help you build TUI (text user interface)
//! apps, along w/ general niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉:
//!
//! 1. Thread-safe & fully asynchronous [Redux](https://docs.rs/r3bl_redux/latest/r3bl_redux/) crate
//!    (using Tokio to run subscribers and middleware in separate tasks). The reducer functions are
//!    run sequentially.
//! 2. Loosely coupled & fully asynchronous [TUI
//!    framework](https://docs.rs/r3bl_tui/latest/r3bl_tui/) to make it possible (and easy) to build
//!    sophisticated TUIs (Text User Interface apps) in Rust.
//! 3. Lots of [declarative macros](https://docs.rs/r3bl_rs_utils_core/latest/r3bl_rs_utils_core/),
//!    and [procedural macros](https://docs.rs/r3bl_rs_utils_macro/latest/r3bl_rs_utils_macro/)
//!    (both function like and derive) to avoid having to write lots of boilerplate code for many
//!    common (and complex) tasks. And even less noisy `Result` and `Error` types.
//! 4. [Non binary tree data](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/) structure
//!    inspired by memory arenas, that is thread safe and supports parallel tree walking.
//! 5. Utility functions to improve
//!    [ergonomics](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/) of commonly used patterns
//!    in Rust programming, ranging from things like colorizing `stdout`, `stderr` output to lazy
//!    value holders.
//!
//! ## Learn more about how this library is built
//!
//! 🦜 Here are some articles (on [developerlife.com](https://developerlife.com)) about how this
//! crate is made:
//! 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
//! 2. <https://developerlife.com/2022/03/12/rust-redux/>
//! 3. <https://developerlife.com/2022/03/30/rust-proc-macro/>
//!
//! 🦀 You can also find all the Rust related content on developerlife.com
//! [here](https://developerlife.com/category/Rust/).
//!
//! # r3bl_tui crate
//!
//! You can build fully async TUI (text user interface) apps with a modern API that brings the best
//! of the web frontend development ideas to TUI apps written in Rust:
//!
//! 1. Reactive & unidirectional data flow architecture from frontend web development (React,
//!    Redux).
//! 2. Responsive design w/ CSS, flexbox like concepts.
//! 3. Declarative style of expressing styling and layouts.
//!
//! And since this is using Rust and Tokio you get the advantages of concurrency and parallelism
//! built-in. No more blocking the main thread for user input, for async middleware, or even
//! rendering 🎉.
//!
//! > This framework is [loosely coupled and strongly
//! > coherent](https://developerlife.com/2015/11/05/loosely-coupled-strongly-coherent/) meaning
//! > that you can pick and choose whatever pieces you would like to use w/out having the cognitive
//! > load of having to grok all the things in the codebase. Its more like a collection of mostly
//! > independent modules that work well w/ each other, but know very little about each other.
//!
//! Here are some framework highlights:
//!
//! - An easy to use and approachable API that is inspired by React, JSX, CSS, and Redux. Lots of
//!   components and things are provided for you so you don't have to build them from scratch. This
//!   is a full featured component library including:
//!   - Redux for state management (fully async, concurrent & parallel).
//!   - CSS like declarative styling engine.
//!   - CSS flexbox like declarative layout engine which is fully responsive. You can resize your
//!     terminal window and everything will be laid out correctly.
//!   - A terminal independent underlying rendering and painting engine (can use crossterm or
//!     termion or whatever you want).
//!   - Text editor w/ syntax highlighting support.
//!   - Modal dialog boxes.
//!   - Lolcat implementation w/ a rainbow color-wheel palette.
//!   - Support for Unicode grapheme clusters in strings. You can safely use emojis, and other
//!     Unicode characters in your TUI apps.
//!   - Support for ANSI text.
//!   - Support for mouse events.
//! - The entire TUI framework itself supports concurrency & parallelism (user input, rendering,
//!   etc. are generally non blocking).
//! - It is fast! There are no needless re-renders, or flickering. Animations and color changes are
//!   smooth (check this out for yourself by running the examples). You can even build your TUI in
//!   layers (like z-order in a browser's DOM).
//!
//! # Examples to get you started
//!
//! Here's a video of the demo in action:
//! <https://user-images.githubusercontent.com/2966499/200138653-c0cf925f-2c91-4908-9ed5-1e216b5dd547.webm>
//!
//! 1. You can run `cargo run --example demo` in the `tui/examples` folder to see a demo of the
//!    library in action and play with it. The examples cover the entire surface area of the TUI
//!    API. You can also take a look at the tests in the source as well `tui/src/`.
//!
//! 2. The design docs and architecture diagrams in the [`docs`
//!    folder](https://github.com/r3bl-org/r3bl_rs_utils/tree/main/docs) are a good place to start
//!    to get a feel for the architecture of the framework. You can get a mental model of how
//!    everything fits and what the TUI lifecycle is.
//!
//! 3. Additionally the [r3bl_rs_utils_core](https://crates.io/crates/r3bl_rs_utils_core) has the
//!    `tui_core` module which contains dependencies that are used by the `tui` module. They
//!    include:
//!    1. ANSI text support.
//!    2. Core dimensions and units that are used for positioning and sizing.
//!    3. Grapheme cluster segment and unicode support (emoji support).
//!    4. Lolcat support.
//!    5. CSS like styling support.
//!
//! # Life of an input event
//!
//! There is a clear separation of concerns in this module. To illustrate what goes where, and how
//! things work let's look at an example that puts the main event loop front and center & deals w/
//! how the system handles an input event (key press or mouse).
//!
//! - The diagram below shows an app that has 3 [Component]s for (flexbox like) layout & (CSS like)
//!   styling.
//! - Let's say that you run this app (by hypothetically executing `cargo run`).
//! - And then you click or type something in the terminal window that you're running this app in.
//!
//! ```text
//! 🧍⌨️🖱️
//! input → [TerminalWindow]
//! event       ↑      ↓               [ComponentRegistry] creates
//!             ┊   [App] ───────────■ [Component]s at 1st render
//!             ┊      │                 
//!             ┊      │        ┌──────■ id=1 has focus
//!             ┊      │        │
//!             ┊      ├→ [Component] id=1 ───┐
//!             ┊      ├→ [Component] id=2    │
//!             ┊      └→ [Component] id=3    │
//!          default                          │
//!          handler  ←───────────────────────┘
//! ```
//!
//! Let's trace the journey through the diagram when an input even is generated by the user (eg: a
//! key press, or mouse event). When the app is started via `cargo run` it sets up a main loop, and
//! lays out all the 3 components, sizes, positions, and then paints them. Then it asynchronously
//! listens for input events (no threads are blocked). When the user types something, this input is
//! processed by the main loop of [TerminalWindow].
//!
//! 1. The [Component] that is in [FlexBox] w/ `id=1` currently has focus.
//! 2. When an input event comes in from the user (key press or mouse input) it is routed to the
//!    [App] first, before [TerminalWindow] looks at the event.
//! 3. The specificity of the event handler in [App] is higher than the default input handler in
//!    [TerminalWindow]. Further, the specificity of the [Component] that currently has focus is the
//!    highest. In other words, the input event gets routed by the [App] to the [Component] that
//!    currently has focus ([Component] id=1 in our example).
//! 4. Since it is not guaranteed that some [Component] will have focus, this input event can then
//!    be handled by [App], and if not, then by [TerminalWindow]'s default handler. If the default
//!    handler doesn't process it, then it is simply ignored.
//! 5. In this journey, as the input event is moved between all these different entities, each
//!    entity decides whether it wants to handle the input event or not. If it does, then it returns
//!    an enum indicating that the event has been consumed, else, it returns an enum that indicates
//!    the event should be propagated.
//!
//! Now that we have seen this whirlwind overview of the life of an input event, let's look at the
//! details in each of the sections below.
//!
//! Here's an architecture diagram that will be useful to keep in mind as we go through the details of
//! the following sections:
//!
//! ![](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/docs/memory-architecture.drawio.svg)
//!
//! # The window
//!
//! The main building blocks of a TUI app are:
//! 1. [TerminalWindow] - You can think of this as the main "window" of the app. All the content of
//!    your app is painted inside of this "window". And the "window" conceptually maps to the screen
//!    that is contained inside your terminal emulator program (eg: tilix, Terminal.app, etc). Your
//!    TUI app will end up taking up 100% of the screen space of this terminal emulator. It will
//!    also enter raw mode, and paint to an alternate screen buffer, leaving your original scroll
//!    back buffer and history intact. When you exit this TUI app, it will return your terminal to
//!    where you'd left off. You don't write this code, this is something that you use.
//! 2. [App] - This is where you write your code. You pass in a [App] to the [TerminalWindow] to
//!    bootstrap your TUI app. You can just use [App] to build your app, if it is a simple one & you
//!    don't really need any sophisticated layout or styling. But if you want layout and styling,
//!    now we have to deal with [FlexBox], [Component], and [r3bl_rs_utils_core::Style].
//!
//! # Layout and styling
//!
//! Inside of your [App] if you want to use flexbox like layout and CSS like styling you can think
//! of composing your code in the following way:
//!
//! 1. [App] is like a box or container. You can attach styles and an id here. The id has to be
//!    unique, and you can reference as many styles as you want from your stylesheet. Yes, cascading
//!    styles are supported! 👏 You can put boxes inside of boxes. You can make a container box and
//!    inside of that you can add other boxes (you can give them a direction and even relative
//!    sizing out of 100%).
//! 2. As you approach the "leaf" nodes of your layout, you will find [Component] trait objects.
//!    These are black boxes which are sized, positioned, and painted *relative* to their parent
//!    box. They get to handle input events and render [RenderOp]s into a [RenderPipeline]. This is
//!    kind of like virtual DOM in React. This queue of commands is collected from all the
//!    components and ultimately painted to the screen, for each render! You can also use Redux to
//!    maintain your app's state, and dispatch actions to the store, and even have async middleware!
//!
//! # [Component] and [ComponentRegistry], focus management, and event routing
//!
//! Typically your [App] will look like this:
//!
//! ```ignore
//! /// Async trait object that implements the [App] trait.
//! #[derive(Default)]
//! pub struct AppWithLayout {
//!   pub component_registry: ComponentRegistry<AppWithLayoutState, AppWithLayoutAction>,
//!   pub has_focus: HasFocus,
//! }
//! ```
//!
//! As we look at [Component] & [App] more closely we will find a curious thing [ComponentRegistry]
//! (that is managed by the [App]). The reason this exists is for input event routing. The input
//! events are routed to the [Component] that currently has focus.
//!
//! The [HasFocus] struct takes care of this. This provides 2 things:
//! 1. It holds an `id` of a [FlexBox] / [Component] that has focus.
//! 2. It also holds a map that holds a [r3bl_rs_utils_core::Position] for each `id`. This is used
//!    to represent a cursor (whatever that means to your app & component). This cursor is
//!    maintained for each `id`. This allows a separate cursor for each [Component] that has focus.
//!    This is needed to build apps like editors and viewers that maintains a cursor position
//!    between focus switches.
//!
//! Another thing to keep in mind is that the [App] and [TerminalWindow] is persistent between
//! re-renders. The Redux store is also persistent between re-renders.
//!
//! # Input event specificity
//!
//! [TerminalWindow] gives [Component] first dibs when it comes to handling input events. If it
//! punts handling this event, it will be handled by the default input event handler. And if nothing
//! there matches this event, then it is simply dropped.
//!
//! # Redux for state management
//!
//! If you use Redux for state management, then you will create a [r3bl_redux::Store] that is passed
//! into the [TerminalWindow]. Here's an example of this.
//!
//! ```ignore
//! use crossterm::event::*;
//! use r3bl_rs_utils::*;
//! use super::*;
//!
//! const DEBUG: bool = true;
//!
//! pub async fn run_app() -> CommonResult<()> {
//!   throws!({
//!     if DEBUG {
//!       try_to_set_log_level(log::LevelFilter::Trace)?;
//!     } else {
//!       try_to_set_log_level(log::LevelFilter::Off)?;
//!     }
//!
//!     // Create store.
//!     let store = create_store().await;
//!
//!     // Create an App (renders & responds to user input).
//!     let shared_app = AppWithLayout::new_shared();
//!
//!     // Exit if these keys are pressed.
//!     let exit_keys: Vec<KeyEvent> = vec![KeyEvent {
//!       code: KeyCode::Char('q'),
//!       modifiers: KeyModifiers::CONTROL,
//!     }];
//!
//!     // Create a window.
//!     TerminalWindow::main_event_loop(store, shared_app, exit_keys).await?
//!   });
//! }
//!
//! async fn create_store() -> Store<AppWithLayoutState, AppWithLayoutAction> {
//!   let mut store: Store<AppWithLayoutState, AppWithLayoutAction> = Store::default();
//!   store.add_reducer(AppReducer::new()).await;
//!   store
//! }
//! ```
//!
//! # Grapheme support
//!
//! Unicode is supported (to an extent). There are some caveats. The
//! [r3bl_rs_utils_core::UnicodeString] struct has lots of great information on this graphemes and
//! what is supported and what is not.
//!
//! # Lolcat support
//!
//! An implementation of [r3bl_rs_utils_core::lolcat::cat] w/ a color wheel is provided.

// Attach.
pub mod tui;

// Re-export.
pub use tui::*;
