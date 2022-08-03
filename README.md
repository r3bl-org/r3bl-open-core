# r3bl_rs_utils
<a id="markdown-r3bl_rs_utils" name="r3bl_rs_utils"></a>


This crate provides lots of useful functionality to help you build TUI (text user interface) apps,
along w/ general niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉:

1. Loosely coupled & fully asynchronous [TUI framework](#tui) to make it possible (and easy) to
   build sophisticated TUIs (Text User Interface apps) in Rust.
2. Fully asynchronous & thread safe [Redux](#redux) library (using Tokio to run subscribers and
   middleware in separate tasks). The reducer functions are run sequentially.
3. [Declarative macros](#declarative), and [procedural macros](#procedural) (both function like and
   derive) to avoid having to write lots of boilerplate code for many common (and complex) tasks.
4. Utility functions to improve [ergonomics](#utils) of commonly used patterns in Rust programming,
   ranging from things like colorizing `stdout`, `stderr` output, to having less noisy `Result` and
   `Error` types.
5. [Non binary tree data](#treememoryarena-non-binary-tree-data-structure) structure (written more
   like a graph than a non binary tree) inspired by memory arenas, that is thread safe and supports
   parallel tree walking.

> 🦜 To learn more about this library, please read how it was built (on
> [developerlife.com](https://developerlife.com)):
>
> 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
> 2. <https://developerlife.com/2022/03/12/rust-redux/>
> 3. <https://developerlife.com/2022/03/30/rust-proc-macro/>
>
> 🦀 You can also find all the Rust related content on developerlife.com
> [here](https://developerlife.com/category/Rust/).
>
> - 🤷‍♂️ Fun fact: before we built this crate, we built a library that is similar in spirit for
>   TypeScript (for TUI apps on Node.js) called
>   [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/). We have since switched to Rust
>   🦀🎉.

<hr/>

Table of contents:

<!-- TOC depthfrom:2 updateonsave:true orderedlist:false insertanchor:true -->

- [Usage](#usage)
- [tui](#tui)
  - [Life of an input event](#life-of-an-input-event)
  - [The window](#the-window)
  - [Layout and styling](#layout-and-styling)
  - [Component, ComponentRegistry, focus management, and event routing](#component-componentregistry-focus-management-and-event-routing)
  - [Input event specificity](#input-event-specificity)
  - [Redux for state management](#redux-for-state-management)
  - [Grapheme support](#grapheme-support)
  - [Lolcat support](#lolcat-support)
  - [Examples to get you started](#examples-to-get-you-started)
- [redux](#redux)
  - [Middlewares](#middlewares)
  - [Subscribers](#subscribers)
  - [Reducers](#reducers)
  - [Summary](#summary)
  - [Examples](#examples)
- [Macros](#macros)
  - [Declarative](#declarative)
    - [assert_eq2!](#assert_eq2)
    - [throws!](#throws)
    - [throws_with_return!](#throws_with_return)
    - [log!](#log)
    - [log_no_err!](#log_no_err)
    - [debug_log_no_err!](#debug_log_no_err)
    - [trace_log_no_err!](#trace_log_no_err)
    - [make_api_call_for!](#make_api_call_for)
    - [fire_and_forget!](#fire_and_forget)
    - [call_if_true!](#call_if_true)
    - [debug!](#debug)
    - [with!](#with)
    - [with_mut!](#with_mut)
    - [with_mut_returns!](#with_mut_returns)
    - [unwrap_option_or_run_fn_returning_err!](#unwrap_option_or_run_fn_returning_err)
    - [unwrap_option_or_compute_if_none!](#unwrap_option_or_compute_if_none)
  - [Procedural](#procedural)
    - [Builder derive macro](#builder-derive-macro)
    - [make_struct_safe_to_share_and_mutate!](#make_struct_safe_to_share_and_mutate)
    - [make_safe_async_fn_wrapper!](#make_safe_async_fn_wrapper)
- [tree_memory_arena non-binary tree data structure](#tree_memory_arena-non-binary-tree-data-structure)
- [utils](#utils)
  - [CommonResult and CommonError](#commonresult-and-commonerror)
  - [LazyField](#lazyfield)
  - [LazyMemoValues](#lazymemovalues)
  - [tty](#tty)
  - [safe_unwrap](#safe_unwrap)
  - [color_text](#color_text)
- [Stability](#stability)
- [Issues, comments, feedback, and PRs](#issues-comments-feedback-and-prs)
- [Notes](#notes)

<!-- /TOC -->

<hr/>

## Usage
<a id="markdown-usage" name="usage"></a>


Please add the following to your `Cargo.toml` file:

```toml
[dependencies]
r3bl_rs_utils = "0.7.40"
```

## tui
<a id="markdown-tui" name="tui"></a>


You can build fully async TUI apps with a modern API that brings the best of reactive &
unidirectional data flow architecture from frontend web development (React, Redux, CSS, flexbox) to
Rust and TUI apps. And since this is using Tokio you get the advantages of concurrency and
parallelism built-in. No more blocking on the main thread for user input, for async middleware, or
even rendering 🎉.

This framework is
[loosely coupled and strongly coherent](https://developerlife.com/2015/11/05/loosely-coupled-strongly-coherent/)
meaning that you can pick and choose whatever pieces you would like to use w/out having the
cognitive load of having to grok all the things in the codebase. Its more like a collection of
mostly independent modules that work well w/ each other, but know very little about each other.

Here are some framework highlights:

- The entire TUI framework itself supports concurrency & parallelism (user input, rendering, etc.
  are generally non blocking).
- Flexbox-like responsive layout.
- CSS-like styling.
- Redux for state management (fully async, concurrent & parallel).
- Lolcat implementation w/ a rainbow color-wheel palette.
- Support for Unicode grapheme clusters in strings.

### Life of an input event
<a id="markdown-life-of-an-input-event" name="life-of-an-input-event"></a>


There is a clear separation of concerns in this module. To illustrate what goes where, and how
things work let's look at an example that puts the main event loop front and center & deals w/ how
the system handles an input event (key press or mouse).

- The diagram below shows an app that has 3 [Component]s for (flexbox like) layout & (CSS like)
  styling.
- Let's say that you run this app (by hypothetically executing `cargo run`).
- And then you click or type something in the terminal window that you're running this app in.

```text
input event → [TerminalWindow]
                  ↑      ↓                 [ComponentRegistry] creates
                  ┊   [TWApp] ───────────■ [Component]s at 1st render
                  ┊      │
                  ┊      │        ┌──────■ id=1 has focus
                  ┊      │        │
                  ┊      ├→ [Component] id=1 ───┐
                  ┊      ├→ [Component] id=2    │
                  ┊      └→ [Component] id=3    │
               default                          │
               handler  ←───────────────────────┘
```

Let's trace the journey through the diagram when an input even is generated by the user (eg: a key
press, or mouse event). When the app is started via `cargo run` it sets up a main loop, and lays out
all the 3 components, sizes, positions, and then paints them. Then it asynchronously listens for
input events (no threads are blocked). When the user types something, this input is processed by the
main loop of [TerminalWindow].

1.  The [Component] that is in [TWBox] w/ `id=1` currently has focus.
2.  When an input event comes in from the user (key press or mouse input) it is routed to the
    [TWApp] first, before [TerminalWindow] looks at the event.
3.  The specificity of the event handler in [TWApp] is higher than the default input handler in
    [TerminalWindow]. Further, the specificity of the [Component] that currently has focus is the
    highest. In other words, the input event gets routed by the [TWApp] to the [Component] that
    currently has focus ([Component] id=1 in our example).
4.  Since it is not guaranteed that some [Component] will have focus, this input event can then be
    handled by [TWApp], and if not, then by [TerminalWindow]'s default handler. If the default
    handler doesn't process it, then it is simply ignored.
5.  In this journey, as the input event is moved between all these different entities, each entity
    decides whether it wants to handle the input event or not. If it does, then it returns an enum
    indicating that the event has been consumed, else, it returns an enum that indicates the event
    should be propagated.

Now that we have seen this whirlwind overview of the life of an input event, let's look at the
details in each of the sections below.

### The window
<a id="markdown-the-window" name="the-window"></a>


The main building blocks of a TUI app are:

1.  [TerminalWindow] - You can think of this as the main "window" of the app. All the content of
    your app is painted inside of this "window". And the "window" conceptually maps to the screen
    that is contained inside your terminal emulator program (eg: tilix, Terminal.app, etc). Your TUI
    app will end up taking up 100% of the screen space of this terminal emulator. It will also enter
    raw mode, and paint to an alternate screen buffer, leaving your original scroll back buffer and
    history intact. When you exit this TUI app, it will return your terminal to where you'd left
    off. You don't write this code, this is something that you use.
2.  [TWApp] - This is where you write your code. You pass in a [TWApp] to the [TerminalWindow] to
    bootstrap your TUI app. You can just use [TWApp] to build your app, if it is a simple one & you
    don't really need any sophisticated layout or styling. But if you want layout and styling, now
    we have to deal with [TWBox], [Component], and [crate::Style].

### Layout and styling
<a id="markdown-layout-and-styling" name="layout-and-styling"></a>


Inside of your [TWApp] if you want to use flexbox like layout and CSS like styling you can think of
composing your code in the following way:

1.  [TWApp] is like a box or container. You can attach styles and an id here. The id has to be
    unique, and you can reference as many styles as you want from your stylesheet. Yes, cascading
    styles are supported! 👏 You can put boxes inside of boxes. You can make a container box and
    inside of that you can add other boxes (you can give them a direction and even relative sizing
    out of 100%).
2.  As you approach the "leaf" nodes of your layout, you will find [Component] trait objects. These
    are black boxes which are sized, positioned, and painted _relative_ to their parent box. They
    get to handle input events and render [TWCommand]s into a [TWCommandQueue]. This is kind of like
    virtual DOM in React. This queue of commands is collected from all the components and ultimately
    painted to the screen, for each render! You can also use Redux to maintain your app's state, and
    dispatch actions to the store, and even have async middleware!

### Component, ComponentRegistry, focus management, and event routing
<a id="markdown-component%2C-componentregistry%2C-focus-management%2C-and-event-routing" name="component%2C-componentregistry%2C-focus-management%2C-and-event-routing"></a>


Typically your [TWApp] will look like this:

```rust
/// Async trait object that implements the [TWApp] trait.
#[derive(Default)]
pub struct AppWithLayout {
  pub component_registry: ComponentRegistry<AppWithLayoutState, AppWithLayoutAction>,
  pub has_focus: HasFocus,
}
```

As we look at [Component] & [TWApp] more closely we will find a curious thing [ComponentRegistry]
(that is managed by the [TWApp]). The reason this exists is for input event routing. The input
events are routed to the [Component] that currently has focus.

The [HasFocus] struct takes care of this. This provides 2 things:

1.  It holds an `id` of a [TWBox] / [Component] that has focus.
2.  It also holds a map that holds a [crate::Position] for each `id`. This is used to represent a
    cursor (whatever that means to your app & component). This cursor is maintained for each `id`.
    This allows a separate cursor for each [Component] that has focus. This is needed to build apps
    like editors and viewers that maintains a cursor position between focus switches.

Another thing to keep in mind is that the [TWApp] and [TerminalWindow] is persistent between
re-renders. The Redux store is also persistent between re-renders.

### Input event specificity
<a id="markdown-input-event-specificity" name="input-event-specificity"></a>


[TerminalWindow] gives [Component] first dibs when it comes to handling input events. If it punts
handling this event, it will be handled by the default input event handler. And if nothing there
matches this event, then it is simply dropped.

### Redux for state management
<a id="markdown-redux-for-state-management" name="redux-for-state-management"></a>


If you use Redux for state management, then you will create a [crate::redux] [crate::Store] that is
passed into the [TerminalWindow]. Here's an example of this.

```rust
use crossterm::event::*;
use r3bl_rs_utils::*;
use super::*;

const DEBUG: bool = true;

pub async fn run_app() -> CommonResult<()> {
  throws!({
    if DEBUG {
      try_to_set_log_level(log::LevelFilter::Trace)?;
    } else {
      try_to_set_log_level(log::LevelFilter::Off)?;
    }

    // Create store.
    let store = create_store().await;

    // Create an App (renders & responds to user input).
    let shared_app = AppWithLayout::new_shared();

    // Exit if these keys are pressed.
    let exit_keys: Vec<KeyEvent> = vec![KeyEvent {
      code: KeyCode::Char('q'),
      modifiers: KeyModifiers::CONTROL,
    }];

    // Create a window.
    TerminalWindow::main_event_loop(store, shared_app, exit_keys).await?
  });
}

async fn create_store() -> Store<AppWithLayoutState, AppWithLayoutAction> {
  let mut store: Store<AppWithLayoutState, AppWithLayoutAction> = Store::default();
  store.add_reducer(AppReducer::new()).await;
  store
}
```

### Grapheme support
<a id="markdown-grapheme-support" name="grapheme-support"></a>


Unicode is supported (to an extent). There are some caveats. The [crate::UnicodeStringExt] trait has
lots of great information on this graphemes and what is supported and what is not.

### Lolcat support
<a id="markdown-lolcat-support" name="lolcat-support"></a>


An implementation of [crate::lolcat::cat] w/ a color wheel is provided.

### Examples to get you started
<a id="markdown-examples-to-get-you-started" name="examples-to-get-you-started"></a>


1.  [Code example of an address book using Redux](https://github.com/r3bl-org/address-book-with-redux-tui).
2.  [Code example of TUI apps using Redux](https://github.com/r3bl-org/r3bl-cmdr).

## redux
<a id="markdown-redux" name="redux"></a>


`Store` is thread safe and asynchronous (using Tokio). You have to implement `async` traits in order
to use it, by defining your own reducer, subscriber, and middleware trait objects. You also have to
supply the Tokio runtime, this library will not create its own runtime. However, for best results,
it is best to use the multithreaded Tokio runtime.

Once you setup your Redux store w/ your reducer, subscriber, and middleware, you can use it by
calling `spawn_dispatch_action!( store, action )`. This kicks off a parallel Tokio task that will
run the middleware functions, reducer functions, and finally the subscriber functions. So this will
not block the thread of whatever code you call this from. The `spawn_dispatch_action!()` macro
itself is not `async`. So you can call it from non `async` code, however you still have to provide a
Tokio executor / runtime, without which you will get a panic when `spawn_dispatch_action!()` is
called.

### Middlewares
<a id="markdown-middlewares" name="middlewares"></a>


Your middleware (`async` trait implementations) will be run concurrently or in parallel via Tokio
tasks. You get to choose which `async` trait to implement to do one or the other. And regardless of
which kind you implement the `Action` that is optionally returned will be dispatched to the Redux
store at the end of execution of all the middlewares (for that particular `spawn_dispatch_action!()`
call).

1. `AsyncMiddlewareSpawns<State, Action>` - Your middleware has to use `tokio::spawn` to run `async`
   blocks in a [separate thread](https://docs.rs/tokio/latest/tokio/task/index.html#spawning) and
   return a `JoinHandle` that contains an `Option<Action>`. A macro
   [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
   is provided so that you can easily spawn parallel blocks of code in your `async` functions. These
   are added to the store via a call to `add_middleware_spawns(...)`.

2. `AsyncMiddleware<State, Action>` - They are will all be run together concurrently using
   [`futures::join_all()`](https://docs.rs/futures/latest/futures/future/fn.join_all.html). These
   are added to the store via a call to `add_middleware(...)`.

### Subscribers
<a id="markdown-subscribers" name="subscribers"></a>


The subscribers will be run asynchronously via Tokio tasks. They are all run together concurrently
but not in parallel, using
[`futures::join_all()`](https://docs.rs/futures/latest/futures/future/fn.join_all.html).

### Reducers
<a id="markdown-reducers" name="reducers"></a>


The reducer functions are also are `async` functions that are run in the tokio runtime. They're also
run one after another in the order in which they're added.

> ⚡ **Any functions or blocks that you write which uses the Redux library will have to be marked
> `async` as well. And you will have to spawn the Tokio runtime by using the `#[tokio::main]` macro.
> If you use the default runtime then Tokio will use multiple threads and its task stealing
> implementation to give you parallel and concurrent behavior. You can also use the single threaded
> runtime; its really up to you.**

1. To create middleware you have to implement the `AsyncMiddleware<S,A>` trait or
   `AsyncMiddlewareSpawns<S,A>` trait. Please read the
   [`AsyncMiddleware` docs](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/redux/async_middleware/trait.AsyncMiddleware.html)
   for examples of both. The `run()` method is passed two arguments: the `State` and the `Action`.

   1. For `AsyncMiddlewareSpawns<S,A>` in your `run()` implementation you have to use the
      [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
      macro to surround your code. And this will return a `JoinHandle<Option<A>>`.
   2. For `AsyncMiddleware<S,A>` in your `run()` implementation you just have to return an
      `Option<A>>`.

2. To create reducers you have to implement the `AsyncReducer` trait.

   - These should be
     [pure functions](https://redux.js.org/understanding/thinking-in-redux/three-principles#changes-are-made-with-pure-functions)
     and simply return a new `State` object.
   - The `run()` method will be passed two arguments: a ref to `Action` and ref to `State`.

3. To create subscribers you have to implement the `AsyncSubscriber` trait.

   - The `run()` method will be passed a `State` object as an argument.
   - It returns nothing `()`.

### Summary
<a id="markdown-summary" name="summary"></a>


Here's the gist of how to make & use one of these:

1. Create a struct. Make it derive `Default`. Or you can add your own properties / fields to this
   struct, and construct it yourself, or even provide a constructor function.
   - A default constructor function `new()` is provided for you by the trait.
   - Just follow how that works for when you need to make your own constructor function for a struct
     w/ your own properties.
2. Implement the `AsyncMiddleware`, `AsyncMiddlewareSpawns`, `AsyncReducer`, or `AsyncSubscriber`
   trait on your struct.
3. Register this struct w/ the store using one of the `add_middleware()`, `add_middleware_spawns()`,
   `add_reducer()`, or `add_subscriber()` methods. You can register as many of these as you like.
   - If you have a struct w/ no properties, you can just use the default `::new()` method to create
     an instance and pass that to the `add_???()` methods.
   - If you have a struct w/ custom properties, you can either implement your own constructor
     function or use the following as an argument to the `add_???()` methods:
     `Box::new($YOUR_STRUCT))`.

### Examples
<a id="markdown-examples" name="examples"></a>


> 💡 There are lots of examples in the
> [tests](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/test_redux.rs) for this library
> and in this [CLI application](https://github.com/r3bl-org/address-book-with-redux-tui/) built
> using it.

Here's an example of how to use it. Let's start w/ the import statements.

```rust
/// Imports.
use async_trait::async_trait;
use r3bl_rs_utils::redux::{
  AsyncMiddlewareSpawns, AsyncMiddleware, AsyncReducer,
  AsyncSubscriber, Store, StoreStateMachine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
```

> 1. Make sure to have the `tokio` and `async-trait` crates installed as well as `r3bl_rs_utils` in
>    your `Cargo.toml` file.
> 2. Here's an example
>    [`Cargo.toml`](https://github.com/r3bl-org/address-book-with-redux-tui/blob/main/Cargo.toml).

Let's say we have the following action enum, and state struct.

```rust
/// Action enum.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Action {
  Add(i32, i32),
  AddPop(i32),
  Clear,
  MiddlewareCreateClearAction,
  Noop,
}

impl Default for Action {
  fn default() -> Self {
    Action::Noop
  }
}

/// State.
#[derive(Clone, Default, PartialEq, Debug)]
pub struct State {
  pub stack: Vec<i32>,
}
```

Here's an example of the reducer function.

```rust
/// Reducer function (pure).
#[derive(Default)]
struct MyReducer;

#[async_trait]
impl AsyncReducer<State, Action> for MyReducer {
  async fn run(
    &self,
    action: &Action,
    state: &State,
  ) -> State {
    match action {
      Action::Add(a, b) => {
        let sum = a + b;
        State { stack: vec![sum] }
      }
      Action::AddPop(a) => {
        let sum = a + state.stack[0];
        State { stack: vec![sum] }
      }
      Action::Clear => State { stack: vec![] },
      _ => state.clone(),
    }
  }
}
```

Here's an example of an async subscriber function (which are run in parallel after an action is
dispatched). The following example uses a lambda that captures a shared object. This is a pretty
common pattern that you might encounter when creating subscribers that share state in your enclosing
block or scope.

```rust
/// This shared object is used to collect results from the subscriber
/// function & test it later.
let shared_object = Arc::new(Mutex::new(Vec::<i32>::new()));

#[derive(Default)]
struct MySubscriber {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncSubscriber<State> for MySubscriber {
  async fn run(
    &self,
    state: State,
  ) {
    let mut stack = self
      .shared_object_ref
      .lock()
      .unwrap();
    if !state.stack.is_empty() {
      stack.push(state.stack[0]);
    }
  }
}

let my_subscriber = MySubscriber {
  shared_object_ref: shared_object_ref.clone(),
};
```

Here are two types of async middleware functions. One that returns an action (which will get
dispatched once this middleware returns), and another that doesn't return anything (like a logger
middleware that just dumps the current action to the console). Note that both these functions share
the `shared_object` reference from above.

```rust
/// This shared object is used to collect results from the subscriber
/// function & test it later.
#[derive(Default)]
struct MwExampleNoSpawn {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwExampleNoSpawn {
  async fn run(
    &self,
    action: Action,
    _store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) {
    let mut stack = self
      .shared_object_ref
      .lock()
      .unwrap();
    match action {
      Action::MwExampleNoSpawn_Add(_, _) => stack.push(-1),
      Action::MwExampleNoSpawn_AddPop(_) => stack.push(-2),
      Action::MwExampleNoSpawn_Clear => stack.push(-3),
      _ => {}
    }
    None
  }
}

let mw_example_no_spawn = MwExampleNoSpawn {
  shared_object_ref: shared_object_ref.clone(),
};

/// This shared object is used to collect results from the subscriber
/// function & test it later.
#[derive(Default)]
struct MwExampleSpawns {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddlewareSpawns<State, Action> for MwExampleSpawns {
  async fn run(
    &self,
    action: Action,
    store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) -> JoinHandle<Option<Action>> {
    fire_and_forget!(
      {
        let mut stack = self
          .shared_object_ref
          .lock()
          .unwrap();
        match action {
          Action::MwExampleSpawns_ModifySharedObject_ResetState => {
            shared_vec.push(-4);
            return Some(Action::Reset);
          }
          _ => {}
        }
        None
      }
    );
  }
}

let mw_example_spawns = MwExampleSpawns {
  shared_object_ref: shared_object_ref.clone(),
};
```

Here's how you can setup a store with the above reducer, middleware, and subscriber functions.

```rust
// Setup store.
let mut store = Store::<State, Action>::default();
store
  .add_reducer(MyReducer::new()) // Note the use of `::new()` here.
  .await
  .add_subscriber(Box::new(         // We aren't using `::new()` here
    my_subscriber,                  // because the struct has properties.
  ))
  .await
  .add_middleware_spawns(Box::new(  // We aren't using `::new()` here
    mw_example_spawns,              // because the struct has properties.
  ))
  .await
  .add_middleware(Box::new(         // We aren't using `::new()` here
    mw_example_no_spawn,            // because the struct has properties.
  ))
  .await;
```

Finally here's an example of how to dispatch an action in a test. You can dispatch actions in
parallel using `spawn_dispatch_action!()` which is "fire and forget" meaning that the caller won't
block or wait for the `spawn_dispatch_action!()` to return.

```rust
// Test reducer and subscriber by dispatching `Add`, `AddPop`, `Clear` actions in parallel.
spawn_dispatch_action!( store, Action::Add(1, 2) );
assert_eq!(shared_object.lock().unwrap().pop(), Some(3));

spawn_dispatch_action!( store, Action::AddPop(1) );
assert_eq!(shared_object.lock().unwrap().pop(), Some(4));

spawn_dispatch_action!( store, Action::Clear );
assert_eq!(store.get_state().stack.len(), 0);
```

## Macros
<a id="markdown-macros" name="macros"></a>


### Declarative
<a id="markdown-declarative" name="declarative"></a>


There are quite a few declarative macros that you will find in the library. They tend to be used
internally in the implementation of the library itself. Here are some that are actually externally
exposed via `#[macro_export]`.

#### assert_eq2!
<a id="markdown-assert_eq2!" name="assert_eq2!"></a>


Similar to [`assert_eq!`] but automatically prints the left and right hand side variables if the
assertion fails. Useful for debugging tests, since the cargo would just print out the left and right
values w/out providing information on what variables were being compared.

#### throws!
<a id="markdown-throws!" name="throws!"></a>


Wrap the given `block` or `stmt` so that it returns a `Result<()>`. It is just syntactic sugar that
helps having to write `Ok(())` repeatedly at the end of each block. Here's an example.

```rust
fn test_simple_2_col_layout() -> CommonResult<()> {
  throws! {
    match input_event {
      TWInputEvent::DisplayableKeypress(character) => {
        println_raw!(character);
      }
      _ => todo!()
    }
  }
}
```

Here's another example.

```rust
fn test_simple_2_col_layout() -> CommonResult<()> {
  throws!({
    let mut canvas = Canvas::default();
    canvas.stylesheet = create_stylesheet()?;
    canvas.canvas_start(
      CanvasPropsBuilder::new()
        .set_pos((0, 0).into())
        .set_size((500, 500).into())
        .build(),
    )?;
    layout_container(&mut canvas)?;
    canvas.canvas_end()?;
  });
}
```

#### throws_with_return!
<a id="markdown-throws_with_return!" name="throws_with_return!"></a>


This is very similar to [`throws!`](#throws) but it also returns the result of the block.

```rust
fn test_simple_2_col_layout() -> CommonResult<CommandQueue> {
  throws_with_return!({
    println!("⛵ Draw -> draw: {}\r", state);
    CommandQueue::default()
  });
}
```

#### log!
<a id="markdown-log!" name="log!"></a>


You can use this macro to dump log messages at 3 levels to a file. By default this file is named
`log.txt` and is dumped in the current directory. Here's how you can use it.

Please note that the macro returns a `Result`. A type alias is provided to save some typing called
`CommonResult<T>` which is just a short hand for `std::result::Result<T, Box<dyn Error>>`. The log
file itself is overwritten for each "session" that you run your program.

```rust
use r3bl_rs_utils::{init_file_logger_once, log, CommonResult};

fn run() -> CommonResult<()> {
  let msg = "foo";
  let msg_2 = "bar";

  log!(INFO, "This is a info message");
  log!(INFO, target: "foo", "This is a info message");

  log!(WARN, "This is a warning message {}", msg);
  log!(WARN, target: "foo", "This is a warning message {}", msg);

  log!(ERROR, "This is a error message {} {}", msg, msg_2);
  log!(ERROR, target: "foo", "This is a error message {} {}", msg, msg_2);

  log!(DEBUG, "This is a debug message {} {}", msg, msg_2);
  log!(DEBUG, target: "foo", "This is a debug message {} {}", msg, msg_2);

  log!(TRACE, "This is a debug message {} {}", msg, msg_2);
  log!(TRACE, target: "foo", "This is a debug message {} {}", msg, msg_2);

  Ok(())
}
```

To change the default log file to whatever you choose, you can use the `try_to_set_log_file_path()`
function. If the logger hasn't yet been initialized, this function will set the log file path.
Otherwise it will return an error.

```rust
use r3bl_rs_utils::{try_set_log_file_path, CommonResult, CommonError};
fn run() {
  match try_set_log_file_path("new_log.txt") {
      Ok(path_set) => debug!(path_set),
      Err(error) => debug!(error),
  }
}
```

To change the default log level or to disable the log itself, you can use the
`try_to_set_log_level()` function.

If you want to override the default log level `LOG_LEVEL`, you can use this function. If the logger
has already been initialized, then it will return a an error.

```rust
use r3bl_rs_utils::{try_to_set_log_level, CommonResult, CommonError};
use log::LevelFilter;

fn run() {
  match try_to_set_log_level(LevelFilter::Trace) {
      Ok(level_set) => debug!(level_set),
      Err(error) => debug!(error),
  }
}
```

To disable logging simply set the log level to
[`LevelFilter::Off`](https://docs.rs/log/latest/log/enum.LevelFilter.html).

```rust
use r3bl_rs_utils::{try_to_set_log_level, CommonResult, CommonError};
use log::LevelFilter;

fn run() {
  match try_to_set_log_level(LevelFilter::Off) {
      Ok(level_set) => debug!(level_set),
      Err(error) => debug!(error),
  }
}
```

Please check out the source
[here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/src/utils/file_logging.rs).

#### log_no_err!
<a id="markdown-log_no_err!" name="log_no_err!"></a>


This macro is very similar to the [log!](#log) macro, except that it won't return any error if the
underlying logging system fails. It will simply print a message to `stderr`. Here's an example.

```rust
pub fn log_state(&self, msg: &str) {
  log_no_err!(INFO, "{:?} -> {}", msg, self.to_string());
  log_no_err!(INFO, target: "foo", "{:?} -> {}", msg, self.to_string());
}
```

#### debug_log_no_err!
<a id="markdown-debug_log_no_err!" name="debug_log_no_err!"></a>


This is a really simple macro to make it effortless to debug into a log file. It outputs `DEBUG`
level logs. It takes a single identifier as an argument, or any number of them. It simply dumps an
arrow symbol, followed by the identifier `stringify`'d along with the value that it contains (using
the `Debug` formatter). All of the output is colorized for easy readability. You can use it like
this.

```rust
let my_string = "Hello World!";
debug_log_no_err!(my_string);
```

#### trace_log_no_err!
<a id="markdown-trace_log_no_err!" name="trace_log_no_err!"></a>


This is very similar to [debug_log_no_err!](#debuglognoerr) except that it outputs `TRACE` level
logs.

```rust
let my_string = "Hello World!";
trace_log_no_err!(my_string);
```

#### make_api_call_for!
<a id="markdown-make_api_call_for!" name="make_api_call_for!"></a>


This macro makes it easy to create simple HTTP GET requests using the `reqwest` crate. It generates
an `async` function called `make_request()` that returns a `CommonResult<T>` where `T` is the type
of the response body. Here's an example.

```rust
use std::{error::Error, fmt::Display};
use r3bl_rs_utils::make_api_call_for;
use serde::{Deserialize, Serialize};

const ENDPOINT: &str = "https://api.namefake.com/english-united-states/female/";

make_api_call_for! {
  FakeContactData at ENDPOINT
}
#[derive(Serialize, Deserialize, Debug, Default)]

pub struct FakeContactData {
  pub name: String,
  pub phone_h: String,
  pub email_u: String,
  pub email_d: String,
  pub address: String,
}

let fake_data = fake_contact_data_api()
            .await
            .unwrap_or_else(|_| FakeContactData {
              name: "Foo Bar".to_string(),
              phone_h: "123-456-7890".to_string(),
              email_u: "foo".to_string(),
              email_d: "bar.com".to_string(),
              ..FakeContactData::default()
            });
```

You can find lots of
[examples here](https://github.com/r3bl-org/address-book-with-redux-tui/blob/main/src/tui/middlewares).

#### fire_and_forget!
<a id="markdown-fire_and_forget!" name="fire_and_forget!"></a>


This is a really simple wrapper around `tokio::spawn()` for the given block. Its just syntactic
sugar. Here's an example of using it for a non-`async` block.

```rust
pub fn foo() {
  fire_and_forget!(
    { println!("Hello"); }
  );
}
```

And, here's an example of using it for an `async` block.

```rust
pub fn foo() {
  fire_and_forget!(
     let fake_data = fake_contact_data_api()
     .await
     .unwrap_or_else(|_| FakeContactData {
       name: "Foo Bar".to_string(),
       phone_h: "123-456-7890".to_string(),
       email_u: "foo".to_string(),
       email_d: "bar.com".to_string(),
       ..FakeContactData::default()
     });
  );
}
```

#### call_if_true!
<a id="markdown-call_if_true!" name="call_if_true!"></a>


Syntactic sugar to run a conditional statement. Here's an example.

```rust
const DEBUG: bool = true;
call_if_true!(
  DEBUG,
  eprintln!(
    "{} {} {}\r",
    r3bl_rs_utils::style_error("▶"),
    r3bl_rs_utils::style_prompt($msg),
    r3bl_rs_utils::style_dimmed(&format!("{:#?}", $err))
  )
);
```

#### debug!
<a id="markdown-debug!" name="debug!"></a>


This is a really simple macro to make it effortless to use the color console logger. It takes a
single identifier as an argument, or any number of them. It simply dumps an arrow symbol, followed
by the identifier (stringified) along with the value that it contains (using the `Debug` formatter).
All of the output is colorized for easy readability. You can use it like this.

```rust
let my_string = "Hello World!";
debug!(my_string);
let my_number = 42;
debug!(my_string, my_number);
```

You can also use it in these other forms for terminal raw mode output. This will dump the output to
stderr.

```rust
if let Err(err) = $cmd {
  let msg = format!("❌ Failed to {}", stringify!($cmd));
  debug!(ERROR_RAW &msg, err);
}
```

This will dump the output to stdout.

```rust
let msg = format!("✅ Did the thing to {}", stringify!($name));
debug!(OK_RAW &msg);
```

#### with!
<a id="markdown-with!" name="with!"></a>


This is a macro that takes inspiration from the `with` scoping function in Kotlin. It just makes it
easier to express a block of code that needs to run after an expression is evaluated and saved to a
given variable. Here's an example.

```rust
with! {
  /* $eval */ LayoutProps {
    id: id.to_string(),
    dir,
    req_size: RequestedSize::new(width_pc, height_pc),
  },
  as /* $id */ it,
  run /* $code */ {
    match self.is_layout_stack_empty() {
      true => self.add_root_layout(it),
      false => self.add_normal_layout(it),
    }?;
  }
}
```

It does the following:

1. Evaluates the `$eval` expression and assigns it to `$id`.
2. Runs the `$code` block.

#### with_mut!
<a id="markdown-with_mut!" name="with_mut!"></a>


This macro is just like [`with!`](#with) but it takes a mutable reference to the `$id` variable.
Here's a code example.

```rust
with_mut! {
  StyleFlag::BOLD_SET | StyleFlag::DIM_SET,
  as mask2,
  run {
    assert!(mask2.contains(StyleFlag::BOLD_SET));
    assert!(mask2.contains(StyleFlag::DIM_SET));
    assert!(!mask2.contains(StyleFlag::UNDERLINE_SET));
    assert!(!mask2.contains(StyleFlag::COLOR_FG_SET));
    assert!(!mask2.contains(StyleFlag::COLOR_BG_SET));
    assert!(!mask2.contains(StyleFlag::MARGIN_SET));
  }
}
```

#### with_mut_returns!
<a id="markdown-with_mut_returns!" name="with_mut_returns!"></a>


This macro is just like [`with_mut!`](#withmutreturns) except that it returns the value of the
`$code` block. Here's a code example.

```rust
let tw_queue = with_mut_returns! {
    ColumnRenderComponent { lolcat },
    as it,
    return {
      it.render_component(tw_surface.current_box()?, state, shared_store).await?
    }
};
```

#### unwrap_option_or_run_fn_returning_err!
<a id="markdown-unwrap_option_or_run_fn_returning_err!" name="unwrap_option_or_run_fn_returning_err!"></a>


This macro can be useful when you are working w/ an expression that returns an `Option` and if that
`Option` is `None` then you want to abort and return an error immediately. The idea is that you are
using this macro in a function that returns a `Result<T>` basically.

Here's an example to illustrate.

```rust
pub fn from(
  width_percent: u8,
  height_percent: u8,
) -> CommonResult<RequestedSize> {
  let size_tuple = (width_percent, height_percent);
  let (width_pc, height_pc) = unwrap_option_or_run_fn_returning_err!(
    convert_to_percent(size_tuple),
    || LayoutError::new_err(LayoutErrorType::InvalidLayoutSizePercentage)
  );
  Ok(Self::new(width_pc, height_pc))
}
```

#### unwrap_option_or_compute_if_none!
<a id="markdown-unwrap_option_or_compute_if_none!" name="unwrap_option_or_compute_if_none!"></a>


This macro is basically a way to compute something lazily when it (the `Option`) is set to `None`.
Unwrap the `$option`, and if `None` then run the `$next` closure which must return a value that is
set to `$option`. Here's an example.

```rust
use r3bl_rs_utils::unwrap_option_or_compute_if_none;

#[test]
fn test_unwrap_option_or_compute_if_none() {
  struct MyStruct {
    field: Option<i32>,
  }
  let mut my_struct = MyStruct { field: None };
  assert_eq!(my_struct.field, None);
  unwrap_option_or_compute_if_none!(my_struct.field, { || 1 });
  assert_eq!(my_struct.field, Some(1));
}
```

### Procedural
<a id="markdown-procedural" name="procedural"></a>


All the procedural macros are organized in 3 crates
[using an internal or core crate](https://developerlife.com/2022/03/30/rust-proc-macro/#add-an-internal-or-core-crate):
the public crate, an internal or core crate, and the proc macro crate.

#### Builder derive macro
<a id="markdown-builder-derive-macro" name="builder-derive-macro"></a>


This derive macro makes it easy to generate builders when annotating a `struct` or `enum`. It
generates It has full support for generics. It can be used like this.

```rust
#[derive(Builder)]
struct Point<X, Y>
where
  X: std::fmt::Display + Clone,
  Y: std::fmt::Display + Clone,
{
  x: X,
  y: Y,
}

let my_pt: Point<i32, i32> = PointBuilder::new()
  .set_x(1 as i32)
  .set_y(2 as i32)
  .build();

assert_eq!(my_pt.x, 1);
assert_eq!(my_pt.y, 2);
```

#### make_struct_safe_to_share_and_mutate!
<a id="markdown-make_struct_safe_to_share_and_mutate!" name="make_struct_safe_to_share_and_mutate!"></a>


This function like macro (with custom syntax) makes it easy to manage shareability and interior
mutability of a struct. We call this pattern the "manager" of "things").

> 🪄 You can read all about it
> [here](https://developerlife.com/2022/03/12/rust-redux/#of-things-and-their-managers).

1. This struct gets wrapped in a `RwLock` for thread safety.
2. That is then wrapped inside an `Arc` so we can share it across threads.
3. Additionally it works w/ Tokio so that it is totally async. It also fully supports generics and
   trait bounds w/ an optional `where` clause.

Here's a very simple usage:

```rust
make_struct_safe_to_share_and_mutate! {
  named MyMapManager<K, V>
  where K: Default + Send + Sync + 'static, V: Default + Send + Sync + 'static
  containing my_map
  of_type std::collections::HashMap<K, V>
}
```

Here's an async example.

```rust
#[tokio::test]
async fn test_custom_syntax_no_where_clause() {
  make_struct_safe_to_share_and_mutate! {
    named StringMap<K, V>
    // where is optional and is missing here.
    containing my_map
    of_type std::collections::HashMap<K, V>
  }

  let my_manager: StringMap<String, String> = StringMap::default();
  let locked_map = my_manager.my_map.read().await;
  assert_eq!(locked_map.len(), 0);
  drop(locked_map);
}
```

#### make_safe_async_fn_wrapper!
<a id="markdown-make_safe_async_fn_wrapper!" name="make_safe_async_fn_wrapper!"></a>


This function like macro (with custom syntax) makes it easy to share functions and lambdas that are
async. They should be safe to share between threads and they should support either being invoked or
spawned.

> 🪄 You can read all about how to write proc macros
> [here](https://developerlife.com/2022/03/30/rust-proc-macro/).

1. A struct is generated that wraps the given function or lambda in an `Arc<RwLock<>>` for thread
   safety and interior mutability.
2. A `get()` method is generated which makes it possible to share this struct across threads.
3. A `from()` method is generated which makes it easy to create this struct from a function or
   lambda.
4. A `spawn()` method is generated which makes it possible to spawn the enclosed function or lambda
   asynchronously using Tokio.
5. An `invoke()` method is generated which makes it possible to invoke the enclosed function or
   lambda synchronously.

Here's an example of how to use this macro.

```rust
use r3bl_rs_utils::make_safe_async_fn_wrapper;

make_safe_async_fn_wrapper! {
  named SafeMiddlewareFnWrapper<A>
  containing fn_mut
  of_type FnMut(A) -> Option<A>
}
```

Here's another example.

```rust
use r3bl_rs_utils::make_safe_async_fn_wrapper;

make_safe_async_fn_wrapper! {
  named SafeSubscriberFnWrapper<S>
  containing fn_mut
  of_type FnMut(S) -> ()
}
```

## tree_memory_arena (non-binary tree data structure)
<a id="markdown-tree_memory_arena-non-binary-tree-data-structure" name="tree_memory_arena-non-binary-tree-data-structure"></a>


[`Arena`] and [`MTArena`] types are the implementation of a
[non-binary tree](https://en.wikipedia.org/wiki/Binary_tree#Non-binary_trees) data structure that is
inspired by [memory arenas](https://en.wikipedia.org/wiki/Memory_arena).

Here's a simple example of how to use the [`Arena`] type:

```rust
use r3bl_rs_utils::{
  tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
  utils::{style_primary, style_prompt},
};

let mut arena = Arena::<usize>::new();
let node_1_value = 42 as usize;
let node_1_id = arena.add_new_node(node_1_value, None);
println!("{} {:#?}", style_primary("node_1_id"), node_1_id);
assert_eq!(node_1_id, 0);
```

Here's how you get weak and strong references from the arena (tree), and tree walk:

```rust
use r3bl_rs_utils::{
  tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
  utils::{style_primary, style_prompt},
};

let mut arena = Arena::<usize>::new();
let node_1_value = 42 as usize;
let node_1_id = arena.add_new_node(node_1_value, None);

{
  assert!(arena.get_node_arc(&node_1_id).is_some());
  let node_1_ref = dbg!(arena.get_node_arc(&node_1_id).unwrap());
  let node_1_ref_weak = arena.get_node_arc_weak(&node_1_id).unwrap();
  assert_eq!(node_1_ref.read().unwrap().payload, node_1_value);
  assert_eq!(
    node_1_ref_weak.upgrade().unwrap().read().unwrap().payload,
    42
  );
}

{
  let node_id_dne = 200 as usize;
  assert!(arena.get_node_arc(&node_id_dne).is_none());
}

{
  let node_1_id = 0 as usize;
  let node_list = dbg!(arena.tree_walk_dfs(&node_1_id).unwrap());
  assert_eq!(node_list.len(), 1);
  assert_eq!(node_list, vec![0]);
}
```

Here's an example of how to use the [`MTArena`] type:

```rust
use std::{
  sync::Arc,
  thread::{self, JoinHandle},
};

use r3bl_rs_utils::{
  tree_memory_arena::{Arena, HasId, MTArena, ResultUidList},
  utils::{style_primary, style_prompt},
};

type ThreadResult = Vec<usize>;
type Handles = Vec<JoinHandle<ThreadResult>>;

let mut handles: Handles = Vec::new();
let arena = MTArena::<String>::new();

// Thread 1 - add root. Spawn and wait (since the 2 threads below need the root).
{
  let arena_arc = arena.get_arena_arc();
  let thread = thread::spawn(move || {
    let mut arena_write = arena_arc.write().unwrap();
    let root = arena_write.add_new_node("foo".to_string(), None);
    vec![root]
  });
  thread.join().unwrap();
}

// Perform tree walking in parallel. Note the lambda does capture many enclosing variable context.
{
  let arena_arc = arena.get_arena_arc();
  let fn_arc = Arc::new(move |uid, payload| {
    println!(
      "{} {} {} Arena weak_count:{} strong_count:{}",
      style_primary("walker_fn - closure"),
      uid,
      payload,
      Arc::weak_count(&arena_arc),
      Arc::weak_count(&arena_arc)
    );
  });

  // Walk tree w/ a new thread using arc to lambda.
  {
    let thread_handle: JoinHandle<ResultUidList> =
      arena.tree_walk_parallel(&0, fn_arc.clone());

    let result_node_list = thread_handle.join().unwrap();
    println!("{:#?}", result_node_list);
  }

  // Walk tree w/ a new thread using arc to lambda.
  {
    let thread_handle: JoinHandle<ResultUidList> =
      arena.tree_walk_parallel(&1, fn_arc.clone());

    let result_node_list = thread_handle.join().unwrap();
    println!("{:#?}", result_node_list);
  }
}
```

> 📜 There are more complex ways of using [`Arena`] and [`MTArena`]. Please look at these extensive
> integration tests that put them thru their paces
> [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).

## utils
<a id="markdown-utils" name="utils"></a>


### CommonResult and CommonError
<a id="markdown-commonresult-and-commonerror" name="commonresult-and-commonerror"></a>


These two structs make it easier to work w/ `Result`s. They are just syntactic sugar and helper
structs. You will find them used everywhere in the
[`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crate.

Here's an example of using them both.

```rust
use r3bl_rs_utils::{CommonError, CommonResult};

#[derive(Default, Debug, Clone)]
pub struct Stylesheet {
  pub styles: Vec<Style>,
}

impl Stylesheet {
  pub fn add_style(
    &mut self,
    style: Style,
  ) -> CommonResult<()> {
    if style.id.is_empty() {
      return CommonError::new_err_with_only_msg("Style id cannot be empty");
    }
    self.styles.push(style);
    Ok(())
  }
}
```

### LazyField
<a id="markdown-lazyfield" name="lazyfield"></a>


This combo of struct & trait object allows you to create a lazy field that is only evaluated when it
is first accessed. You have to provide a trait implementation that computes the value of the field
(once). Here's an example.

```rust
use r3bl_rs_utils::{LazyExecutor, LazyField};

#[test]
fn test_lazy_field() {
  struct MyExecutor;
  impl LazyExecutor<i32> for MyExecutor {
    fn compute(&mut self) -> i32 {
      1
    }
  }

  let mut lazy_field = LazyField::new(Box::new(MyExecutor));
  assert_eq!(lazy_field.has_computed, false);

  // First access will trigger the computation.
  let value = lazy_field.compute();
  assert_eq!(lazy_field.has_computed, true);
  assert_eq!(value, 1);

  // Subsequent accesses will not trigger the computation.
  let value = lazy_field.compute();
  assert_eq!(lazy_field.has_computed, true);
  assert_eq!(value, 1);
}
```

### LazyMemoValues
<a id="markdown-lazymemovalues" name="lazymemovalues"></a>


This struct allows users to create a lazy hash map. A function must be provided that computes the
values when they are first requested. These values are cached for the lifetime this struct. Here's
an example.

```rust
use std::sync::atomic::{AtomicUsize, Ordering::SeqCst};
use r3bl_rs_utils::utils::LazyMemoValues;

// These are copied in the closure below.
let arc_atomic_count = AtomicUsize::new(0);
let mut a_variable = 123;
let mut a_flag = false;

let mut generate_value_fn = LazyMemoValues::new(|it| {
  arc_atomic_count.fetch_add(1, SeqCst);
  a_variable = 12;
  a_flag = true;
  a_variable + it
});

assert_eq!(arc_atomic_count.load(SeqCst), 0);
assert_eq!(generate_value_fn.get_ref(&1), &13);
assert_eq!(arc_atomic_count.load(SeqCst), 1);
assert_eq!(generate_value_fn.get_ref(&1), &13); // Won't regenerate the value.
assert_eq!(arc_atomic_count.load(SeqCst), 1); // Doesn't change.
```

### tty
<a id="markdown-tty" name="tty"></a>


This module contains a set of functions to make it easier to work with terminals.

The following is an example of how to use `is_stdin_piped()`:

```rust
fn run(args: Vec<String>) -> Result<(), Box<dyn Error>> {
  match is_stdin_piped() {
    true => piped_grep(PipedGrepOptionsBuilder::parse(args)?)?,
    false => grep(GrepOptionsBuilder::parse(args)?)?,
  }
  Ok(())
}
```

The following is an example of how to use `readline()`:

```rust
use r3bl_rs_utils::utils::{
  print_header, readline, style_dimmed, style_error, style_primary, style_prompt,
};

fn make_a_guess() -> String {
  println!("{}", Blue.paint("Please input your guess."));
  let (bytes_read, guess) = readline();
  println!(
    "{} {}, {} {}",
    style_dimmed("#bytes read:"),
    style_primary(&bytes_read.to_string()),
    style_dimmed("You guessed:"),
    style_primary(&guess)
  );
  guess
}
```

Here's a list of functions available in this module:

- `readline_with_prompt()`
- `print_prompt()`
- `readline()`
- `is_tty()`
- `is_stdout_piped()`
- `is_stdin_piped()`

### safe_unwrap
<a id="markdown-safe_unwrap" name="safe_unwrap"></a>


Functions that make it easy to unwrap a value safely. These functions are provided to improve the
ergonomics of using wrapped values in Rust. Examples of wrapped values are `<Arc<RwLock<T>>`, and
`<Option>`. These functions are inspired by Kotlin scope functions & TypeScript expression based
language library which can be found
[here on `r3bl-ts-utils`](https://github.com/r3bl-org/r3bl-ts-utils).

Here are some examples.

```rust
use r3bl_rs_utils::utils::{
  call_if_some, unwrap_arc_read_lock_and_call, unwrap_arc_write_lock_and_call, with_mut,
};
use r3bl_rs_utils::utils::{ReadGuarded, WriteGuarded};
use r3bl_rs_utils::{
  arena_types::HasId, ArenaMap, FilterFn, NodeRef, ResultUidList, WeakNodeRef,
};

if let Some(parent_id) = parent_id_opt {
  let parent_node_arc_opt = self.get_node_arc(parent_id);
  call_if_some(&parent_node_arc_opt, &|parent_node_arc| {
    unwrap_arc_write_lock_and_call(&parent_node_arc, &mut |parent_node| {
      parent_node.children.push(new_node_id);
    });
  });
}
```

Here's a list of functions that are provided:

- `call_if_some()`
- `call_if_none()`
- `call_if_ok()`
- `call_if_err()`
- `with()`
- `with_mut()`
- `unwrap_arc_write_lock_and_call()`
- `unwrap_arc_read_lock_and_call()`

Here's a list of type aliases provided for better readability:

- `ReadGuarded<T>`
- `WriteGuarded<T>`

### color_text
<a id="markdown-color_text" name="color_text"></a>


ANSI colorized text <https://github.com/ogham/rust-ansi-term> helper methods. Here's an example.

```rust
use r3bl_rs_utils::utils::{
  print_header, readline, style_dimmed, style_error, style_primary, style_prompt,
};

fn make_a_guess() -> String {
  println!("{}", Blue.paint("Please input your guess."));
  let (bytes_read, guess) = readline();
  println!(
    "{} {}, {} {}",
    style_dimmed("#bytes read:"),
    style_primary(&bytes_read.to_string()),
    style_dimmed("You guessed:"),
    style_primary(&guess)
  );
  guess
}
```

Here's a list of functions available in this module:

- `print_header()`
- `style_prompt()`
- `style_primary()`
- `style_dimmed()`
- `style_error()`

## Stability
<a id="markdown-stability" name="stability"></a>


🧑‍🔬 This library is in active development.

1. The `tui` module is current under active development. You can see what's baking in:
   - `r3bl-cmdr` [repo](https://github.com/r3bl-org/r3bl-cmdr)
   - `r3bl-cmdr` [crate](https://crates.io/crates/r3bl-cmdr)
2. The goal is not to have breaking changes for existing code, and be thoughtful when adding new
   functionality. This is why code lives in other repos for a while before being moved to this one.
3. There are extensive tests for code that is production ready.

## Issues, comments, feedback, and PRs
<a id="markdown-issues%2C-comments%2C-feedback%2C-and-prs" name="issues%2C-comments%2C-feedback%2C-and-prs"></a>


Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.

## Notes
<a id="markdown-notes" name="notes"></a>


Here are some notes on using experimental / unstable features in Tokio.

```toml
# The rustflags needs to be set since we are using unstable features
# in Tokio.
# - https://github.com/tokio-rs/console
# - https://docs.rs/tokio/latest/tokio/#unstable-features

# This is how you set rustflags for cargo build defaults.
# - https://github.com/rust-lang/rust-analyzer/issues/5828

[target.x86_64-unknown-linux-gnu]
rustflags = [
    "--cfg", "tokio_unstable",
]
```
