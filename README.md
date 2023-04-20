<p align="center">
  <img src="https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/r3bl-term.svg" height="128px">
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

# r3bl_rs_utils
<a id="markdown-r3bl_rs_utils" name="r3bl_rs_utils"></a>


This crate is related to the first thing that's described above. It provides lots of useful
functionality to help you build TUI (text user interface) apps, along w/ general niceties &
ergonomics that all Rustaceans 🦀 can enjoy 🎉:

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
> 🤷‍♂️ Fun fact: before we built this crate, we built a library that is similar in spirit for
> TypeScript (for TUI apps on Node.js) called
> [r3bl-ts-utils](https://github.com/r3bl-org/r3bl-ts-utils/). We have since switched to Rust 🦀🎉.

<hr/>

Table of contents:

<!-- TOC depthfrom:2 updateonsave:true orderedlist:false insertanchor:true -->

- [tui and tui_core](#tui-and-tui_core)
- [redux](#redux)
- [Macros](#macros)
  - [Declarative](#declarative)
  - [Procedural](#procedural)
- [tree_memory_arena non-binary tree data structure](#tree_memory_arena-non-binary-tree-data-structure)
- [utils](#utils)
  - [LazyField](#lazyfield)
  - [LazyMemoValues](#lazymemovalues)
  - [tty](#tty)
  - [safe_unwrap](#safe_unwrap)
  - [color_text](#color_text)
- [Notes](#notes)
- [Issues, comments, feedback, and PRs](#issues-comments-feedback-and-prs)

<!-- /TOC -->

<hr/>

## tui and tui_core
<a id="markdown-tui-and-tui_core" name="tui-and-tui_core"></a>


For more information please read the README for the
[r3bl_tui crate](https://docs.rs/r3bl_tui/latest/r3bl_tui/).

<!-- How to upload video: https://stackoverflow.com/a/68269430/2085356 -->

Here's a video of the demo in action:

https://user-images.githubusercontent.com/2966499/233481838-b6da884f-f73d-4e1f-adef-94beb9761c46.mp4


## redux
<a id="markdown-redux" name="redux"></a>


For more information please read the README for the
[r3bl_redux crate](https://docs.rs/r3bl_redux/latest/r3bl_redux/).

## Macros
<a id="markdown-macros" name="macros"></a>


### Declarative
<a id="markdown-declarative" name="declarative"></a>


For more information please read the README for the
[r3bl_rs_utils_core crate](https://docs.rs/r3bl_rs_utils_core/latest/r3bl_rs_utils_core/).

### Procedural
<a id="markdown-procedural" name="procedural"></a>


For more information please read the README for the
[r3bl_rs_utils_macro crate](https://docs.rs/r3bl_rs_utils_macro/latest/r3bl_rs_utils_macro/).

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

// Access node.
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

// Node does not exist.
{
  let node_id_dne = 200 as usize;
  assert!(arena.get_node_arc(&node_id_dne).is_none());
}

// Walk tree.
{
  let node_1_id = 0 as usize;
  let node_list = dbg!(arena.tree_walk_dfs(&node_1_id).unwrap());
  assert_eq!(node_list.len(), 1);
  assert_eq!(node_list, vec![0]);
}

// Mutate node.
{
  let node_1_id = 0_usize;
  {
    let node_1_ref = dbg!(arena.get_node_arc(node_1_id).unwrap());
    node_1_ref.write().unwrap().payload = 100;
  }
  assert_eq2!(
    arena.get_node_arc(node_1_id).unwrap().read().unwrap().payload,
    100
  );
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

## Issues, comments, feedback, and PRs
<a id="markdown-issues%2C-comments%2C-feedback%2C-and-prs" name="issues%2C-comments%2C-feedback%2C-and-prs"></a>


Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.
