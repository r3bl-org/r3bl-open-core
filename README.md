# r3bl_rs_utils

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Usage](#usage)
- [redux](#redux)
- [Macros](#macros)
  - [Declarative](#declarative)
    - [debug!](#debug)
  - [Procedural](#procedural)
    - [#[derive(Builder)]](#derivebuilder)
    - [make_struct_safe_to_share_and_mutate!](#make_struct_safe_to_share_and_mutate)
    - [make_safe_async_fn_wrapper!](#make_safe_async_fn_wrapper)
- [tree_memory_arena (non-binary tree data structure)](#tree_memory_arena-non-binary-tree-data-structure)
- [utils](#utils)
  - [LazyMemoValues](#lazymemovalues)
  - [tty](#tty)
  - [safe_unwrap](#safe_unwrap)
  - [color_text](#color_text)
- [tui (experimental)](#tui-experimental)
- [Stability](#stability)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

This library provides utility functions:

1. Thread safe asynchronous Redux library (uses Tokio to run subscribers and middleware in
   separate tasks). The reducer functions are run in sequence (not in Tokio tasks).
2. Declarative macros, and procedural macros (both function like and derive) to avoid
   having to write lots of boilerplate code for many common (and complex) tasks.
3. Non binary tree data structure inspired by memory arenas, that is thread safe and
   supports parallel tree walking.
4. Functions to unwrap deeply nested objects inspired by Kotlin scope functions.
5. Capabilities to make it easier to build TUIs (Text User Interface apps) in Rust. This
   is currently experimental and is being actively developed.

> 💡 To learn more about this library, please read how it was built on
> [developerlife.com](https://developerlife.com):
>
> 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
> 2. <https://developerlife.com/2022/03/12/rust-redux/>
> 3. <https://developerlife.com/2022/03/30/rust-proc-macro/>

> 💡 You can also read all the Rust content on
> [developerlife.com here](https://developerlife.com/category/Rust/). Also, the equivalent
> of this library is available for TypeScript and is called
> [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/).

## Usage

Please add the following to your `Cargo.toml` file:

```toml
[dependencies]
r3bl_rs_utils = "0.7.17"
```

## redux

`Store` is thread safe and asynchronous (using Tokio). You have to implement `async`
traits in order to use it, by defining your own reducer, subscriber, and middleware trait
objects.

- The middleware will be run asynchronously via Tokio tasks. However, they are run one
  after another (in the order in which they're added). If you need to dispatch an action
  after your task is completed then you are free to use `tokio::spawn` to run `async`
  blocks in a
  [separate thread](https://docs.rs/tokio/latest/tokio/task/index.html#spawning). A macro
  [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
  is provided so that you can easily spawn parallel blocks of code in your `async`
  functions.
- The subscribers will be run asynchronously via Tokio tasks. However, they are run one
  after another (in the order in which they're added).
- The reducer functions are also are `async` functions that are run in the tokio runtime.
  They're also run one after another in the order in which they're added.

> ⚡ **Any functions or blocks that you write which uses the Redux library will have to be
> marked `async` as well. And you will have to spawn the Tokio runtime by using the
> `#[tokio::main]` macro. If you use the default runtime then Tokio will use multiple
> threads and its task stealing implementation to give you parallel and concurrent
> behavior. You can also use the single threaded runtime; its really up to you.**

1. To create middleware you have to implement the `AsyncMiddleware` trait.

   - The `run()` method is passed two arguments: the `Store` and the `Action`. It returns
     nothing.
   - You can get the `State` from the `Store` using
     `store_ref.read().await.get_state_clone()` to perform any state dependent computation
     in your task.
   - If you want to dispatch an action when your task is done, you have to:
     1. Use the
        [`fire_and_forget!`](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/macro.fire_and_forget.html)
        macro to surround your code. Please read the
        [`AsyncMiddleware` docs](https://docs.rs/r3bl_rs_utils/latest/r3bl_rs_utils/redux/async_middleware/trait.AsyncMiddleware.html)
        to make sure that you are not deadlocking.
     2. Use the `Store` reference to dispatch an action
        `store_ref.write().await().dispatch_action(...)`.

2. To create reducers you have to implement the `AsyncReducer` trait.

   - These should be
     [pure functions](https://redux.js.org/understanding/thinking-in-redux/three-principles#changes-are-made-with-pure-functions)
     and simply return a new `State` object.
   - The `run()` method will be passed two arguments: a ref to `Action` and ref to
     `State`.

3. To create subscribers you have to implement the `AsyncSubscriber` trait.

   - The `run()` method will be passed a `State` object as an argument.
   - It returns nothing `()`.

Here's the gist of how to make & use one of these:

1. Create a struct. Make it derive `Default`. Or you can add your own properties / fields
   to this struct, and construct it yourself, or even provide a constructor function.
   - A default constructor function `new()` is provided for you by the trait.
   - Just follow that works for when you need to make your own constructor function for a
     struct w/ your own properties.
2. Implement the `AsyncMiddleware`, `AsyncReducer`, or `AsyncSubscriber` trait on your
   struct.
3. Register this struct w/ the store using one of the `add_middleware()`, `add_reducer()`,
   or `add_subscriber()` methods. You can register multiple of these.
   - If you have a struct w/ no properties, you can just use the default `::new()` method
     to create an instance and pass that to the `add_???()` methods.
   - If you have a struct w/ custom properties, you can either implement your own
     constructor function or use the following as an argument to the `add_???()` methods:
     `Arc::new(RwLock::new($YOUR_STRUCT))`.

> 💡 There are lots of examples in the
> [tests](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/redux_test.rs) for
> this library and in this
> [CLI application](https://github.com/nazmulidris/rust_scratch/blob/main/address-book-with-redux/)
> built using it.

Here's an example of how to use it. Let's start w/ the import statements.

```rust
/// Imports.
use async_trait::async_trait;
use r3bl_rs_utils::redux::{
  AsyncMiddleware, AsyncReducer, AsyncSubscriber, Store, StoreStateMachine,
};
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;
```

> 1. Make sure to have the `tokio` and `async-trait` crates installed as well as
>    `r3bl_rs_utils` in your `Cargo.toml` file.
> 2. Here's an example
>    [`Cargo.toml`](https://github.com/nazmulidris/rust_scratch/blob/main/address-book-with-redux/Cargo.toml).

Let's say we have the following action enum, and state struct.

```rust
/// Action enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
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
#[derive(Clone, Default, PartialEq, Debug, Hash)]
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

Here's an example of an async subscriber function (which are run in parallel after an
action is dispatched). The following example uses a lambda that captures a shared object.
This is a pretty common pattern that you might encounter when creating subscribers that
share state in your enclosing block or scope.

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

Here are two types of async middleware functions. One that returns an action (which will
get dispatched once this middleware returns), and another that doesn't return anything
(like a logger middleware that just dumps the current action to the console). Note that
both these functions share the `shared_object` reference from above.

```rust
/// This shared object is used to collect results from the subscriber
/// function & test it later.
#[derive(Default)]
struct MwReturnsNone {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwReturnsNone {
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
      Action::MwReturnsNone_Add(_, _) => stack.push(-1),
      Action::MwReturnsNone_AddPop(_) => stack.push(-2),
      Action::MwReturnsNone_Clear => stack.push(-3),
      _ => {}
    }
  }
}

let mw_returns_none = MwReturnsNone {
  shared_object_ref: shared_object_ref.clone(),
};

/// This shared object is used to collect results from the subscriber
/// function & test it later.
#[derive(Default)]
struct MwReturnsAction {
  pub shared_object_ref: Arc<Mutex<Vec<i32>>>,
}

#[async_trait]
impl AsyncMiddleware<State, Action> for MwReturnsAction {
  async fn run(
    &self,
    action: Action,
    store_ref: Arc<RwLock<StoreStateMachine<State, Action>>>,
  ) {
    fire_and_forget!(
      {
        let mut stack = self
          .shared_object_ref
          .lock()
          .unwrap();
        match action {
          Action::MwReturnsAction_SetState => stack.push(-4),
          _ => {}
        }

        store_ref
          .write()
          .await
          .dispatch_action(Action::Clear, store_ref.clone())
          .await;
      }
    );
  }
}

let mw_returns_action = MwReturnsAction {
  shared_object_ref: shared_object_ref.clone(),
};
```

Here's how you can setup a store with the above reducer, middleware, and subscriber
functions.

```rust
// Setup store.
let mut store = Store::<State, Action>::default();
store
  .add_reducer(MyReducer::new()) // Note the use of `new()` here.
  .await
  .add_subscriber(Arc::new(RwLock::new( // We aren't using `new()` here
    my_subscriber,                      // because the struct has properties.
  )))
  .await
  .add_middleware(Arc::new(RwLock::new( // We aren't using `new()` here
    mw_returns_action,                  // because the struct has properties.
  )))
  .await
  .add_middleware(Arc::new(RwLock::new( // We aren't using `new()` here
    mw_returns_none,                    // because the struct has properties.
  )))
  .await;
```

Finally here's an example of how to dispatch an action in a test. You can dispatch actions
asynchronously using `dispatch_spawn()` which is "fire and forget" meaning that the caller
won't block or wait for the `dispatch_spawn()` to return.

```rust
// Test reducer and subscriber by dispatching Add and AddPop actions asynchronously.
store.dispatch_spawn(Action::Add(10, 10)).await;
store.dispatch_spawn(&Action::Add(1, 2)).await;
assert_eq!(shared_object.lock().unwrap().pop(), Some(3));
store.dispatch_spawn(&Action::AddPop(1)).await;
assert_eq!(shared_object.lock().unwrap().pop(), Some(21));
store.clear_subscribers().await;

// Test async middleware: mw_returns_action.
shared_object.lock().unwrap().clear();
store
  .add_middleware(SafeMiddlewareFnWrapper::new(mw_returns_action))
  .dispatch_spawn(&Action::MiddlewareCreateClearAction)
  .await;
assert_eq!(store.get_state().stack.len(), 0);
assert_eq!(shared_object.lock().unwrap().pop(), Some(-4));
```

## Macros

### Declarative

There are quite a few declarative macros that you will find in the library. They tend to
be used internally in the implementation of the library itself. Here are some that are
actually externally exposed via `#[macro_export]`.

#### debug!

This is a really simple macro to make it effortless to use the color console logger. It
takes an identifier as an argument. It simply dumps an arrow symbol, followed by the
identifier (stringified) along with the value that it contains (using the `Debug`
formatter). All of the output is colorized for easy readability. You can use it like this.

```rust
let my_string = "Hello World!";
debug!(my_string);
```

### Procedural

All the procedural macros are organized in 3 crates
[using an internal or core crate](https://developerlife.com/2022/03/30/rust-proc-macro/#add-an-internal-or-core-crate):
the public crate, an internal or core crate, and the proc macro crate.

#### #[derive(Builder)]

This derive macro makes it easy to generate builders when annotating a `struct` or `enum`.
It generates It has full support for generics. It can be used like this.

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

This function like macro (with custom syntax) makes it easy to manage shareability and
interior mutability of a struct. We call this pattern the "manager" of "things").

> 🪄 You can read all about it
> [here](https://developerlife.com/2022/03/12/rust-redux/#of-things-and-their-managers).

1. This struct gets wrapped in a `RwLock` for thread safety.
2. That is then wrapped inside an `Arc` so we can share it across threads.
3. Additionally it works w/ Tokio so that it is totally async. It also fully supports
   generics and trait bounds w/ an optional `where` clause.

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

This function like macro (with custom syntax) makes it easy to share functions and lambdas
that are async. They should be safe to share between threads and they should support
either being invoked or spawned.

> 🪄 You can read all about how to write proc macros
> [here](https://developerlife.com/2022/03/30/rust-proc-macro/).

1. A struct is generated that wraps the given function or lambda in an `Arc<RwLock<>>` for
   thread safety and interior mutability.
2. A `get()` method is generated which makes it possible to share this struct across
   threads.
3. A `from()` method is generated which makes it easy to create this struct from a
   function or lambda.
4. A `spawn()` method is generated which makes it possible to spawn the enclosed function
   or lambda asynchronously using Tokio.
5. An `invoke()` method is generated which makes it possible to invoke the enclosed
   function or lambda synchronously.

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

[`Arena`] and [`MTArena`] types are the implementation of a
[non-binary tree](https://en.wikipedia.org/wiki/Binary_tree#Non-binary_trees) data
structure that is inspired by [memory arenas](https://en.wikipedia.org/wiki/Memory_arena).

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

> 📜 There are more complex ways of using [`Arena`] and [`MTArena`]. Please look at these
> extensive integration tests that put them thru their paces
> [here](https://github.com/r3bl-org/r3bl-rs-utils/blob/main/tests/tree_memory_arena_test.rs).

## utils

### LazyMemoValues

This struct allows users to create a lazy hash map. A function must be provided that
computes the values when they are first requested. These values are cached for the
lifetime this struct. Here's an example.

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

Functions that make it easy to unwrap a value safely. These functions are provided to
improve the ergonomics of using wrapped values in Rust. Examples of wrapped values are
`<Arc<RwLock<T>>`, and `<Option>`. These functions are inspired by Kotlin scope functions
& TypeScript expression based language library which can be found
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

ANSI colorized text <https://github.com/ogham/rust-ansi-term> helper methods. Here's an
example.

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

## tui (experimental)

🚧 WIP - This is an experimental module that isn’t ready yet. It is the first step towards
creating a TUI library that can be used to create sophisticated TUI applications. This is
similar to Ink library for Node.js & TypeScript (that uses React and Yoga). Or kinda like
`tui` built atop `crossterm` (and not `termion`).

## Stability

🧑‍🔬 This library is in early development.

1. There are extensive integration tests for code that is production ready.
2. Everything else is marked experimental in the source.

Please report any issues to the
[issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues). And if you have any
feature requests, feel free to add them there too 👍.
