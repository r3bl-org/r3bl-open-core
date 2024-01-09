<p align="center">
  <img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/r3bl-term.svg" height="128px">
</p>

> 🪷 If you are interested in contributing to this project, please read our [📒 contributing
> guide](CONTRIBUTING.md).

# Context

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

🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
development w/ a twist: taking concepts that work really well for the frontend mobile and web
development world and re-imagining them for TUI & Rust.

- Draw inspiration from declarative and reactive frameworks for web, mobile, and desktop, to
  create responsive TUIs.
- The idea here is not to rebuild tmux in Rust (separate processes mux'd onto a single terminal
  window). Rather it is to build a set of integrated "apps" (or "tasks") that run in the same
  process that renders to one terminal window.
- Inside of this terminal window, we can implement things like "app" switching, routing, tiling
  layout, stacking layout, etc. so that we can manage a lot of TUI apps (which are tightly
  integrated) that are running in the same process, in the same window. So you can imagine that
  all these "app"s have shared application state. As the application state is modified via
  user input events, API callbacks, and animator updates, this results in performant re-renders,
  and repaints of only parts of the UI that have changed.
- Here are some examples of the types of "app"s we are building using this engine:
  1. multi user text editors w/ syntax highlighting
  2. integrations w/ github issues
  3. integrations w/ calendar, email, contacts APIs


> 🦜 To learn more about this library, please read how it was built (on
> [developerlife.com](https://developerlife.com)):
>
> 1. <https://developerlife.com/2022/02/24/rust-non-binary-tree/>
> 2. <https://developerlife.com/2022/03/12/rust-redux/>
> 3. <https://developerlife.com/2022/03/30/rust-proc-macro/>
>
> 🦀 You can also find all the Rust related content on developerlife.com
> [here](https://developerlife.com/category/Rust/).

<hr/>

# Table of contents
<a id="markdown-table-of-contents" name="table-of-contents"></a>

<!-- TOC -->

- [Folder structure and code organization](#folder-structure-and-code-organization)
- [Issues, comments, feedback, PRs, and Discord](#issues-comments-feedback-prs-and-discord)
- [cmdr folder -> r3bl_cmdr crate](#cmdr-folder---r3bl_cmdr-crate)
- [tui folder -> r3bl_tui crate](#tui-folder---r3bl_tui-crate)
- [ansi_color folder -> r3bl_ansi_color crate](#ansi_color-folder---r3bl_ansi_color-crate)
- [tuify folder -> r3bl_tuify crate](#tuify-folder---r3bl_tuify-crate)

<!-- /TOC -->

## Folder structure and code organization
<a id="markdown-folder-structure-and-code-organization" name="folder-structure-and-code-organization"></a>

This repo is organized as a monorepo. Each folder is a Rust crate. There's a `Cargo.toml`
file at the top level which provides a workspace that allows us to build all the crates in
this repo at the same time.

## Issues, comments, feedback, PRs, and Discord
<a id="markdown-issues%2C-comments%2C-feedback%2C-prs%2C-and-discord" name="issues%2C-comments%2C-feedback%2C-prs%2C-and-discord"></a>

- To contribute please check out [this
  page](https://github.com/r3bl-org/r3bl-open-core/contribute).
- Please report any issues to the [issue
  tracker](https://github.com/r3bl-org/r3bl-rs-utils/issues).
- Check out our contributor guide
  [here](https://github.com/r3bl-org/r3bl_rs_utils/blob/main/CONTRIBUTING.md#commit-message-guidelines).
- And if you have any feature requests, feel free to add them there too 👍.
- And we have a [discord server](https://discord.gg/8M2ePAevaM) if you would like to chat
  about the issue or PR.

## cmdr folder -> r3bl_cmdr crate
<a id="markdown-cmdr-folder--%3E-r3bl_cmdr-crate" name="cmdr-folder--%3E-r3bl_cmdr-crate"></a>

Here's a video of a prototype of [R3BL
CMDR](https://github.com/r3bl-org/r3bl-open-core/tree/main/cmdr) app built using this TUI
engine. You can install the apps in this crate using `cargo install r3bl-cmdr`. This will
install:
- `giti` - Interactive git client.
- `edi` - Markdown editor.

![rc](https://user-images.githubusercontent.com/2966499/234949476-98ad595a-3b72-497f-8056-84b6acda80e2.gif)

## tui folder -> r3bl_tui crate
<a id="markdown-tui-folder--%3E-r3bl_tui-crate" name="tui-folder--%3E-r3bl_tui-crate"></a>

For more information please read the
[README](https://github.com/r3bl-org/r3bl-open-core/tree/main/tui/README.md#r3bl_tui-crate) for the [r3bl_tui
crate](https://docs.rs/r3bl_tui/latest/r3bl_tui/).

<!-- How to upload video: https://stackoverflow.com/a/68269430/2085356 -->

Here's a video of the demo in action:

![video-gif](https://user-images.githubusercontent.com/2966499/233799311-210b887e-0aa6-470a-bcea-ee8e0e3eb019.gif)

## ansi_color folder -> r3bl_ansi_color crate
<a id="markdown-ansi_color-folder--%3E-r3bl_ansi_color-crate" name="ansi_color-folder--%3E-r3bl_ansi_color-crate"></a>

Rust crate to generate formatted ANSI 256 (8-bit) and truecolor (24-bit) color output to stdout. On
macOS, the default Terminal.app does not support truecolor, so ANSI 256 colors are used instead.

[README](https://github.com/r3bl-org/r3bl-open-core/tree/main/ansi_color/README.md) for the
[r3bl_ansi_color crate](https://docs.rs/r3bl_ansi_color/latest/r3bl_ansi_color/).

## tuify folder -> r3bl_tuify crate
<a id="markdown-tuify-folder--%3E-r3bl_tuify-crate" name="tuify-folder--%3E-r3bl_tuify-crate"></a>

This crate can be used in two ways:

As a library. This is useful if you want to add simple interactivity to your CLI app
written in Rust. You can see an example of this in the `examples` folder in the
`main_interactive.rs` file. You can run it using `cargo run --example main_interactive`.

Here's a demo of the library target of this crate in action.

https://user-images.githubusercontent.com/2966499/266870250-9af806a6-9d2a-48b3-9c02-22d8a05cbdc3.mp4

As a binary. This is useful if you want to use this crate as a command line tool. The
binary target is called `rt`.

Here's a demo of the binary target of this crate in action.

https://github.com/r3bl-org/r3bl-open-core/assets/2966499/2b42db72-cd62-4ea2-80ae-ccc01008190c

For more information please read the
[README](https://github.com/r3bl-org/r3bl-open-core/tree/main/tuify/README.md) for the
[r3bl_tuify crate](https://docs.rs/r3bl_tuify/latest/r3bl_tuify/).
