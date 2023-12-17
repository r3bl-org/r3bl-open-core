<p align="center">
  <img src="r3bl-term.svg" height="128px">
</p>

# r3bl-cmdr
<a id="markdown-r3bl-cmdr" name="r3bl-cmdr"></a>


<!-- TOC -->

- [Context](#context)
- [This binary crate: r3bl-cmdr](#this-binary-crate-r3bl-cmdr)
- [Building & running locally](#building--running-locally)
- [Contributing](#contributing)

<!-- /TOC -->

## Context
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

## This binary crate: r3bl-cmdr
<a id="markdown-this-binary-crate%3A-r3bl-cmdr" name="this-binary-crate%3A-r3bl-cmdr"></a>


`r3bl-cmdr` is the second thing that's described above. It contains a set of apps for developers by
developers. It is engineered to enhance your:

- ❯ 🚀 productivity
- ❯ 🌍 efficiency
- ❯ 📖 knowledge capture & sharing
- ❯ 🛣️ workflow management

Our goal is to put a smile on your face every time you use this product.

## Building & running locally
<a id="markdown-building-%26-running-locally" name="building-%26-running-locally"></a>


You can run it using `cargo run`. There are 2 other ways to launch it if you install it on your
machine both that involve a little less typing.

```sh
cargo install --path .
r3bl-cmdr # this is the same as `cargo run` or `cargo run --bin r3bl-cmdr`
rc # this is an alias for `r3bl-cmdr`, equivalent to `cargo run --bin rc``
```

The `Cargo.toml` file contains a `[dependencies]` section which lists all the dependencies that this
crate has, one of which is a
[path dependency](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#multiple-locations)
on the `r3bl_rs_utils` crate. The creates.io version uses the `r3bl_rs_utils` crate from the
crates.io repository, but your local copy will use the folder specified in the path.

> We plan to remove the `path` dependency when this crate is ready to be used by end users.
> Currently it early WIP so it has this dependency for ease of development.

So, to build and run this locally, you have to clone the
[r3bl_rs_utils](https://github.com/r3bl-org/r3bl_rs_utils) repo so that it shares the same parent as
this crate. Here's a sample folder structure.

```text
├── github
│     ├── r3bl-cmdr
│     └── r3bl-rs-utils
```

## Contributing
<a id="markdown-contributing" name="contributing"></a>


This binary crate is being developed as a set of examples. The actual product will emerge as these
examples are evolved into features of the actual product, which is intended to be released to
developers.

Please read our [community contributing guidelines here](./CONTRIBUTING.md).
