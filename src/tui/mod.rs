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

//! # tui package
//!
//! You can build fully async TUI apps with a modern API that brings the best of reactive &
//! unidirectional data flow architecture from frontend web development (React, Redux, CSS, flexbox)
//! to Rust and TUI apps. And since this is using Tokio you get the advantages of concurrency and
//! parallelism built-in. No more blocking on the main thread for user input, for async middleware,
//! or even rendering 🎉.
//!
//! This framework is [loosely coupled and strongly
//! coherent](https://developerlife.com/2015/11/05/loosely-coupled-strongly-coherent/) meaning that
//! you can pick and choose whatever pieces you would like to use w/out having the cognitive load of
//! having to grok all the things in the codebase. Its more like a collection of mostly independent
//! modules that work well w/ each other, but know very little about each other.
//!
//! Here are some framework highlights:
//! - The entire TUI framework itself supports concurrency & parallelism (user input, rendering,
//!   etc. are generally non blocking).
//! - Flexbox-like responsive layout.
//! - CSS-like styling.
//! - Redux for state management (fully async, concurrent & parallel).
//! - Lolcat implementation w/ a rainbow color-wheel palette.
//! - Support for Unicode grapheme clusters in strings.
//!
//! ## Life of an input event
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
//! event       ↑      ↓                 [ComponentRegistry] creates
//!             ┊   [TWApp] ───────────■ [Component]s at 1st render
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
//! 1. The [Component] that is in [TWBox] w/ `id=1` currently has focus.
//! 2. When an input event comes in from the user (key press or mouse input) it is routed to the
//!    [TWApp] first, before [TerminalWindow] looks at the event.
//! 3. The specificity of the event handler in [TWApp] is higher than the default input handler in
//!    [TerminalWindow]. Further, the specificity of the [Component] that currently has focus is the
//!    highest. In other words, the input event gets routed by the [TWApp] to the [Component] that
//!    currently has focus ([Component] id=1 in our example).
//! 4. Since it is not guaranteed that some [Component] will have focus, this input event can then
//!    be handled by [TWApp], and if not, then by [TerminalWindow]'s default handler. If the default
//!    handler doesn't process it, then it is simply ignored.
//! 5. In this journey, as the input event is moved between all these different entities, each
//!    entity decides whether it wants to handle the input event or not. If it does, then it returns
//!    an enum indicating that the event has been consumed, else, it returns an enum that indicates
//!    the event should be propagated.
//!
//! Now that we have seen this whirlwind overview of the life of an input event, let's look at the
//! details in each of the sections below.
//!
//! ## The window
//!
//! The main building blocks of a TUI app are:
//! 1. [TerminalWindow] - You can think of this as the main "window" of the app. All the content of
//!    your app is painted inside of this "window". And the "window" conceptually maps to the screen
//!    that is contained inside your terminal emulator program (eg: tilix, Terminal.app, etc). Your
//!    TUI app will end up taking up 100% of the screen space of this terminal emulator. It will
//!    also enter raw mode, and paint to an alternate screen buffer, leaving your original scroll
//!    back buffer and history intact. When you exit this TUI app, it will return your terminal to
//!    where you'd left off. You don't write this code, this is something that you use.
//! 2. [TWApp] - This is where you write your code. You pass in a [TWApp] to the [TerminalWindow] to
//!    bootstrap your TUI app. You can just use [TWApp] to build your app, if it is a simple one &
//!    you don't really need any sophisticated layout or styling. But if you want layout and
//!    styling, now we have to deal with [TWBox], [Component], and [crate::Style].
//!
//! ## Layout and styling
//!
//! Inside of your [TWApp] if you want to use flexbox like layout and CSS like styling you can think
//! of composing your code in the following way:
//!
//! 1. [TWApp] is like a box or container. You can attach styles and an id here. The id has to be
//!    unique, and you can reference as many styles as you want from your stylesheet. Yes, cascading
//!    styles are supported! 👏 You can put boxes inside of boxes. You can make a container box and
//!    inside of that you can add other boxes (you can give them a direction and even relative
//!    sizing out of 100%).
//! 2. As you approach the "leaf" nodes of your layout, you will find [Component] trait objects.
//!    These are black boxes which are sized, positioned, and painted *relative* to their parent
//!    box. They get to handle input events and render [TWCommand]s into a [TWCommandQueue]. This is
//!    kind of like virtual DOM in React. This queue of commands is collected from all the
//!    components and ultimately painted to the screen, for each render! You can also use Redux to
//!    maintain your app's state, and dispatch actions to the store, and even have async middleware!
//!
//! ## [Component] and [ComponentRegistry], focus management, and event routing
//!
//! Typically your [TWApp] will look like this:
//!
//! ```ignore
//! /// Async trait object that implements the [TWApp] trait.
//! #[derive(Default)]
//! pub struct AppWithLayout {
//!   pub component_registry: ComponentRegistry<AppWithLayoutState, AppWithLayoutAction>,
//!   pub has_focus: HasFocus,
//! }
//! ```
//!
//! As we look at [Component] & [TWApp] more closely we will find a curious thing
//! [ComponentRegistry] (that is managed by the [TWApp]). The reason this exists is for input event
//! routing. The input events are routed to the [Component] that currently has focus.
//!
//! The [HasFocus] struct takes care of this. This provides 2 things:
//! 1. It holds an `id` of a [TWBox] / [Component] that has focus.
//! 2. It also holds a map that holds a [crate::Position] for each `id`. This is used to represent a
//!    cursor (whatever that means to your app & component). This cursor is maintained for each
//!    `id`. This allows a separate cursor for each [Component] that has focus. This is needed to
//!    build apps like editors and viewers that maintains a cursor position between focus switches.
//!
//! Another thing to keep in mind is that the [TWApp] and [TerminalWindow] is persistent between
//! re-renders. The Redux store is also persistent between re-renders.
//!
//! ## Input event specificity
//!
//! [TerminalWindow] gives [Component] first dibs when it comes to handling input events. If it
//! punts handling this event, it will be handled by the default input event handler. And if nothing
//! there matches this event, then it is simply dropped.
//!
//! ## Redux for state management
//!
//! If you use Redux for state management, then you will create a [crate::redux] [crate::Store] that
//! is passed into the [TerminalWindow]. Here's an example of this.
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
//! ## Grapheme support
//!
//! Unicode is supported (to an extent). There are some caveats. The [crate::UnicodeStringExt] trait
//! has lots of great information on this graphemes and what is supported and what is not.
//!
//! ## Lolcat support
//!
//! An implementation of [crate::lolcat::cat] w/ a color wheel is provided.
//!
//! ## Examples to get you started
//!
//! 1. [Code example of an address book using Redux](https://github.com/r3bl-org/address-book-with-redux-tui).
//! 2. [Code example of TUI apps using Redux](https://github.com/r3bl-org/r3bl-cmdr).

// Attach sources.
pub mod crossterm_helpers;
pub mod layout;
pub mod syntax_highlighting;
pub mod terminal_window;

// Re-export.
pub use crossterm_helpers::*;
pub use layout::*;
pub use syntax_highlighting::*;
pub use terminal_window::*;
