<p align="center">
  <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg" height="128px">
</p>

# Context
<a id="markdown-context" name="context"></a>


<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span style="color:#F92363">
</span><span style="color:#F82067">T</span><span style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span style="color:#F31874">
</span><span style="color:#F11678">l</span><span style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span style="color:#E10895">
</span><span style="color:#DE0799">&amp;</span><span style="color:#DB069E">
</span><span style="color:#D804A2">s</span><span style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span style="color:#C801B6">
</span><span style="color:#C501B9">o</span><span style="color:#C101BD">f</span><span style="color:#BD01C1">
</span><span style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span style="color:#AA03D2">
</span><span style="color:#A603D5">f</span><span style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span style="color:#890DE8">
</span><span style="color:#850FEB">o</span><span style="color:#8111ED">n</span><span style="color:#7C13EF">
</span><span style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span style="color:#572CFC">r</span><span style="color:#532FFD">
</span><span style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

We are working on building command line apps in Rust which have rich text user interfaces (TUI). We
want to lean into the terminal as a place of productivity, and build all kinds of awesome apps for
it.

1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
   development w/ a twist: taking concepts that work really well for the frontend mobile and web
   development world and re-imagining them for TUI & Rust.

   - Taking things like React, JSX, CSS, and Redux, but making everything async (they can be run in
     parallel & concurrent via Tokio).
   - Even the thread running the main event loop doesn't block since it is async.
   - Using proc macros to create DSLs to implement CSS & JSX.

2. 🌎 We are building apps to enhance developer productivity & workflows.

   - The idea here is not to rebuild tmux in Rust (separate processes mux'd onto a single terminal
     window). Rather it is to build a set of integrated "apps" (or "tasks") that run in the same
     process that renders to one terminal window.
   - Inside of this terminal window, we can implement things like "app" switching, routing, tiling
     layout, stacking layout, etc. so that we can manage a lot of TUI apps (which are tightly
     integrated) that are running in the same process, in the same window. So you can imagine that
     all these "app"s have shared application state (that is in a Redux store). Each "app" may also
     have its own Redux store.
   - Here are some examples of the types of "app"s we want to build:
     1. multi user text editors w/ syntax highlighting
     2. integrations w/ github issues
     3. integrations w/ calendar, email, contacts APIs

# r3bl_redux
<a id="markdown-r3bl_redux" name="r3bl_redux"></a>


This crate is related to the first thing that's described above. It provides lots of useful
functionality to help you build TUI (text user interface) apps, along w/ general niceties &
ergonomics that all Rustaceans 🦀 can enjoy 🎉:

<!-- TOC depthfrom:2 updateonsave:true orderedlist:false insertanchor:true -->

- [redux](#redux)
  - [Middlewares](#middlewares)
  - [Subscribers](#subscribers)
  - [Reducers](#reducers)
  - [Summary](#summary)
  - [Examples](#examples)
- [Other crates that depend on this](#other-crates-that-depend-on-this)
- [Issues, comments, feedback, and PRs](#issues-comments-feedback-and-prs)

<!-- /TOC -->

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
    state: &mut State,
  ) {
    match action {
      Action::Add(a, b) => {
        let sum = a + b;
        state.stack = vec![sum];
      }
      Action::AddPop(a) => {
        let sum = a + state.stack[0];
        state.stack = vec![sum];
      }
      Action::Clear => State {
        state.stack.clear();
      },
      _ => {}
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
            return Action::Reset.into();
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

## Other crates that depend on this
<a id="markdown-other-crates-that-depend-on-this" name="other-crates-that-depend-on-this"></a>


This crate is a dependency of the following crates:

1. [`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crates (the "main" library)

## Issues, comments, feedback, and PRs
<a id="markdown-issues%2C-comments%2C-feedback%2C-and-prs" name="issues%2C-comments%2C-feedback%2C-and-prs"></a>


Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.
