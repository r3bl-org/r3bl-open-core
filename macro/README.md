<p align="center">
  <img src="r3bl-term.svg" height="128px">
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

# r3bl_rs_utils_macro
<a id="markdown-r3bl_rs_utils_macro" name="r3bl_rs_utils_macro"></a>


<!-- TOC depthfrom:2 updateonsave:true orderedlist:false insertanchor:true -->

- [Macros](#macros)
  - [Procedural](#procedural)
    - [style! macro](#style-macro)
    - [Builder derive macro](#builder-derive-macro)
    - [make_struct_safe_to_share_and_mutate!](#make_struct_safe_to_share_and_mutate)
    - [make_safe_async_fn_wrapper!](#make_safe_async_fn_wrapper)
- [Other crates that depend on this](#other-crates-that-depend-on-this)
- [Issues, comments, feedback, and PRs](#issues-comments-feedback-and-prs)

<!-- /TOC -->

## Macros
<a id="markdown-macros" name="macros"></a>


### Procedural
<a id="markdown-procedural" name="procedural"></a>


All the procedural macros are organized in 3 crates
[using an internal or core crate](https://developerlife.com/2022/03/30/rust-proc-macro/#add-an-internal-or-core-crate):
the public crate, an internal or core crate, and the proc macro crate.

#### style! macro
<a id="markdown-style!-macro" name="style!-macro"></a>


Here's an example of the `style!` macro:

```rust
style! {
  id: "my_style",          /* Optional. */
  attrib: [dim, bold]      /* Optional. */
  padding: 10,             /* Optional. */
  color_fg: TWColor::Blue, /* Optional. */
  color_bg: TWColor::Red,  /* Optional. */
}
```

`color_fg` and `color_bg` can take any of the following:

1. Color enum value.
2. Rgb value.
3. Variable holding either of the above.

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

## Other crates that depend on this
<a id="markdown-other-crates-that-depend-on-this" name="other-crates-that-depend-on-this"></a>


This crate is a dependency of [`r3bl_rs_utils`](https://crates.io/crates/r3bl_rs_utils) crate (the
"main" library).

## Issues, comments, feedback, and PRs
<a id="markdown-issues%2C-comments%2C-feedback%2C-and-prs" name="issues%2C-comments%2C-feedback%2C-and-prs"></a>


Please report any issues to the [issue tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
And if you have any feature requests, feel free to add them there too 👍.
