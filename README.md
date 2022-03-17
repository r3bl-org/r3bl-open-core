# r3bl_rs_utils

This library provides utility functions:

1. Thread safe asynchronous Redux library (uses `tokio` to run subscribers and middleware in
   parallel). The reducer functions are run single threaded.
2. Non binary tree data structure inspired by memory arenas, that is thread safe and supports
   parallel tree walking.
3. Functions to unwrap deeply nested objects inspired by Kotlin scope functions.
4. Capabilities to make it easier to build TUIs (Text User Interface apps) in Rust.
5. And more.

> 💡 To learn more about this library, please read how it was built on
> [developerlife.com](https://developerlife.com):
>
> 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>.
> 2. <https://developerlife.com/2022/03/12/rust-redux/>
>
> You can also read all the Rust content on
> [developerlife.com here](https://developerlife.com/category/Rust/). And the equivalent of this
> library is available for TypeScript and is called
> [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/).

## Usage

Please add the following to your `Cargo.toml` file:

```toml
[dependencies]
r3bl_rs_utils = "0.6.7"
```

## redux

`Store` is a thread safe asynchronous Redux library that uses `tokio` to run subscribers and
middleware in parallel. The reducer functions are run single threaded. Here's an example of how to
use it. Let's say we have the following action enum, and state struct.

```rust
/// Action enum.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum Action {
  Add(i32, i32),
  AddPop(i32),
  Clear,
  MiddlewareCreateClearAction,
}

/// State.
#[derive(Clone, Default, PartialEq, Debug, Hash)]
pub struct State {
  pub stack: Vec<i32>,
}
```

Here's an example of the reducer function.

```rust
// Reducer function (pure).
let reducer_fn = |state: &State, action: &Action| match action {
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
};
```

Here's an example of an async subscriber function (which are run in parallel after an action is
dispatched). The following example uses a lambda that captures a shared object. This is a pretty
common pattern that you might encounter when creating subscribers that share state in your enclosing
block or scope.

```rust
// This shared object is used to collect results from the subscriber function & test it later.
let shared_object = Arc::new(Mutex::new(Vec::<i32>::new()));
// This subscriber function is curried to capture a reference to the shared object.
let subscriber_fn = with(shared_object.clone(), |it| {
  let curried_fn = move |state: State| {
    let mut stack = it.lock().unwrap();
    stack.push(state.stack[0]);
  };
  curried_fn
});
```

Here are two types of async middleware functions. One that returns an action (which will get
dispatched once this middleware returns), and another that doesn't return anything (like a logger
middleware that just dumps the current action to the console). Note that both these functions share
the `shared_object` reference from above.

```rust
// This middleware function is curried to capture a reference to the shared object.
let mw_returns_none = with(shared_object.clone(), |it| {
  let curried_fn = move |action: Action| {
    let mut stack = it.lock().unwrap();
    match action {
      Action::Add(_, _) => stack.push(-1),
      Action::AddPop(_) => stack.push(-2),
      Action::Clear => stack.push(-3),
      _ => {}
    }
    None
  };
  curried_fn
});

// This middleware function is curried to capture a reference to the shared object.
let mw_returns_action = with(shared_object.clone(), |it| {
  let curried_fn = move |action: Action| {
    let mut stack = it.lock().unwrap();
    match action {
      Action::MiddlewareCreateClearAction => stack.push(-4),
      _ => {}
    }
    Some(Action::Clear)
  };
  curried_fn
});
```

Here's how you can setup a store with the above reducer, middleware, and subscriber functions.

```rust
// Setup store.
let mut store = Store::<State, Action>::new();
store
  .add_reducer(ReducerFnWrapper::new(reducer_fn))
  .await
  .add_subscriber(SafeSubscriberFnWrapper::new(subscriber_fn))
  .await
  .add_middleware(SafeMiddlewareFnWrapper::new(mw_returns_none))
  .await;
```

Finally here's an example of how to dispatch an action in a test. You can dispatch actions
asynchronously using `dispatch_spawn()` which is "fire and forget" meaning that the caller won't
block or wait for the `dispatch_spawn()` to return. Then you can dispatch actions synchronously if
that's what you would like using `dispatch()`.

```rust
// Test reducer and subscriber by dispatching Add and AddPop actions asynchronously.
store.dispatch_spawn(Action::Add(10, 10)).await;
store.dispatch(&Action::Add(1, 2)).await;
assert_eq!(shared_object.lock().unwrap().pop(), Some(3));
store.dispatch(&Action::AddPop(1)).await;
assert_eq!(shared_object.lock().unwrap().pop(), Some(21));
store.clear_subscribers().await;

// Test async middleware: mw_returns_action.
shared_object.lock().unwrap().clear();
store
  .add_middleware(SafeMiddlewareFnWrapper::new(mw_returns_action))
  .dispatch(&Action::MiddlewareCreateClearAction)
  .await;
assert_eq!(store.get_state().stack.len(), 0);
assert_eq!(shared_object.lock().unwrap().pop(), Some(-4));
```

## tree_memory_arena (non-binary tree data structure)

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

### LazyMemoValues

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

## tui (experimental)

🚧 WIP - This is an experimental module that isn’t ready yet. It is the first step towards creating
a TUI library that can be used to create sophisticated TUI applications. This is similar to Ink
library for Node.js & TypeScript (that uses React and Yoga). Or kinda like `tui` built atop
`crossterm` (and not `termion`).

## Stability

🧑‍🔬 This library is in early development.

1. There are extensive integration tests for code that is production ready.
2. Everything else is marked experimental in the source.

Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.
