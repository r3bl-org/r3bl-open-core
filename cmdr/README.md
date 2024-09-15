# r3bl-cmdr

## Why R3BL?

<img src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/r3bl-cmdr-eap.svg?raw=true" height="256px">

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

We are working on building command line apps in Rust which have rich text user interfaces (TUI).
We want to lean into the terminal as a place of productivity, and build all kinds of awesome
apps for it.

1. 🔮 Instead of just building one app, we are building a library to enable any kind of rich TUI
   development w/ a twist: taking concepts that work really well for the frontend mobile and web
   development world and re-imagining them for TUI & Rust.

   - Taking inspiration from things like [React](https://react.dev/),
     [SolidJS](https://www.solidjs.com/),
     [Elm](https://guide.elm-lang.org/architecture/),
     [iced-rs](https://docs.rs/iced/latest/iced/), [Jetpack
     Compose](https://developer.android.com/compose),
     [JSX](https://ui.dev/imperative-vs-declarative-programming),
     [CSS](https://www.w3.org/TR/CSS/#css), but making everything async (so they can
     be run in parallel & concurrent via [Tokio](https://crates.io/crates/tokio)).
   - Even the thread running the main event loop doesn't block since it is async.
   - Using proc macros to create DSLs to implement something inspired by
     [CSS](https://www.w3.org/TR/CSS/#css) &
     [JSX](https://ui.dev/imperative-vs-declarative-programming).

2. 🌎 We are building apps to enhance developer productivity & workflows.

   - The idea here is not to rebuild `tmux` in Rust (separate processes mux'd onto a
     single terminal window). Rather it is to build a set of integrated "apps" (or
     "tasks") that run in the same process that renders to one terminal window.
   - Inside of this terminal window, we can implement things like "app" switching,
     routing, tiling layout, stacking layout, etc. so that we can manage a lot of TUI
     apps (which are tightly integrated) that are running in the same process, in the
     same window. So you can imagine that all these "app"s have shared application
     state. Each "app" may also have its own local application state.
   - Here are some examples of the types of "app"s we plan to build (for which this
     infrastructure acts as the open source engine):
     1. Multi user text editors w/ syntax highlighting.
     2. Integrations w/ github issues.
     3. Integrations w/ calendar, email, contacts APIs.

All the crates in the `r3bl-open-core`
[repo](https://github.com/r3bl-org/r3bl-open-core/) provide lots of useful
functionality to help you build TUI (text user interface) apps, along w/ general
niceties & ergonomics that all Rustaceans 🦀 can enjoy 🎉.

## Table of contents

<!-- TOC -->

- [Installation](#installation)
- [Changelog](#changelog)
- [Learn how these crates are built, provide feedback](#learn-how-these-crates-are-built-provide-feedback)
- [Run giti binary target](#run-giti-binary-target)
- [Run edi binary target](#run-edi-binary-target)
- [Build, run, test tasks](#build-run-test-tasks)
  - [Prerequisites](#prerequisites)
  - [Nushell scripts to build, run, test etc.](#nushell-scripts-to-build-run-test-etc)

<!-- /TOC -->

## Installation

The two apps, `edi` and `giti`, that comprise `r3bl-cmdr` will put a smile on your face and
make you more productive. These apps are currently available as early access preview 🐣.

- 😺 `giti` - an interactive git CLI app designed to give you more confidence and a better
  experience when working with git.
- 🦜 `edi` - a TUI Markdown editor that lets you edit Markdown files in your terminal in
  style.

To install `r3bl-cmdr` on your system, run the following command, assuming you have
`cargo` on your system:

```bash
cargo install r3bl-cmdr
```

If you don't have `cargo` on your system, you can either:

1. Follow these [instructions](https://rustup.rs/) to install `cargo` on your system
   first. Then run `cargo install r3bl-cmdr` to install this crate.
2. Build the binaries from the crate's source code. First clone this
   [repo](https://github.com/r3bl-org/r3bl-open-core/). Then, run `cd
   r3bl-open-core/cmdr && cargo install`.

## Changelog

Please check out the
[changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl-cmdr) to
see how the library has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate, please take a look at the following resources.
- If you like consuming video content, here's our [YT channel](https://www.youtube.com/@developerlifecom). Please consider [subscribing](https://www.youtube.com/channel/CHANNEL_ID?sub_confirmation=1).
- If you like consuming written content, here's our developer [site](https://developerlife.com/). Please consider subscribing to our [newsletter](https://developerlife.com/subscribe.html).
- If you have questions, please join our [discord server](https://discord.gg/8M2ePAevaM).

## Run `giti` binary target

<!--
giti branch video
Source: https://github.com/nazmulidris/developerlife.com/issues/5
Source mp4: https://github.com/nazmulidris/developerlife.com/assets/2966499/262f59d1-a95c-4af3-accf-c3d6cac6e586
-->
![giti video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/giti.gif?raw=true)

To run from binary:
- Run `cargo install r3bl-cmdr` (detailed instructions above). This will install `giti`
  locally to `~/.cargo/bin`.
- Run `giti` from anywhere on your system.
- Try `giti --help` to see the available commands.
- To delete one or more branches in your repo run `giti branch delete`.
- To checkout a branch run `giti branch checkout`.
- To create a new branch run `giti branch new`.

To run from source:
- Clone the `r3bl-open-core` repo.
- Go to the `cmdr` folder in your terminal.
- Run `nu run install` to install `giti` locally to `~/.cargo/bin`.
- Run `giti` from anywhere on your system.
- Try `giti --help` to see the available commands.
- To delete one or more branches in your repo run `giti branch delete`.
- To checkout a branch run `giti branch checkout`.
- To create a new branch run `giti branch new`.
- If you want to generate log output for `giti`, run `giti -l`. For example,
  `giti -l branch delete`. To view this log output run `nu run log`.

## Run `edi` binary target

<!--
edi video
Source: https://github.com/nazmulidris/developerlife.com/issues/6
Source mp4: https://github.com/nazmulidris/developerlife.com/assets/2966499/f2c4b07d-b5a2-4f41-af7a-06d1b6660c41
-->
![edi video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/edi.gif?raw=true)

To run from binary:
- Run `cargo install r3bl-cmdr` (detailed instructions above). This will install `giti`
  locally to `~/.cargo/bin`.
- Run `edi` from anywhere on your system.
- Try `edi --help` to see the available commands.
- To open an existing file, run `edi <file_name>`. For example, `edi README.md`.

To run from source:
- Clone the `r3bl-open-core` repo.
- Go to the `cmdr` folder in your terminal.
- Run `nu run install` to install `edi` locally to `~/.cargo/bin`.
- Run `edi` from anywhere on your system.
- Try `edi --help` to see the available commands.
- To open an existing file, run `edi <file_name>`. For example, `edi README.md`.
- If you want to generate log output for `edi`, run `edi -l`. For example,
  `edi -l README.md`. To view this log output run `nu run log`.

## Build, run, test tasks

### Prerequisites

🌠 In order for these to work you have to install the Rust toolchain, `nu`, `cargo-watch`, `bat`,
and `flamegraph` on your system. Here are the instructions:

1. Install the Rust toolchain using `rustup` by following the instructions
   [here](https://rustup.rs/).
1. Install `cargo-watch` using `cargo install cargo-watch`.
1. Install `flamegraph` using `cargo install flamegraph`.
1. Install `bat` using `cargo install bat`.
1. Install [`nu`](https://crates.io/crates/nu) shell on your system using `cargo install nu`. It is
   available for Linux, macOS, and Windows.

### Nushell scripts to build, run, test etc.

| Command             | Description                                                                                                          |
| ------------------- | -------------------------------------------------------------------------------------------------------------------- |
| `nu run help`       | See all the commands you can pass to the `run` script                                                                |
| `nu run install`    | Install `giti`, `edi`, `rc` to `~/.cargo/bin`                                                                        |
| `nu run build`      | Build                                                                                                                |
| `nu run clean`      | Clean                                                                                                                |
| `nu run test`       | Run tests                                                                                                            |
| `nu run clippy`     | Run clippy                                                                                                           |
| `nu run log`        | View the log output. This [video](https://www.youtube.com/watch?v=Sy26IMkOEiM) has a walkthrough of how to use this. |
| `nu run docs`       | Build docs                                                                                                           |
| `nu run serve-docs` | Serve docs over VSCode Remote SSH session                                                                            |
| `nu run rustfmt`    | Run rustfmt                                                                                                          |

The following commands will watch for changes in the source folder and re-run:

| Command                                             | Description                        |
| --------------------------------------------------- | ---------------------------------- |
| `nu run watch-all-tests`                            | Watch all test                     |
| `nu run watch-one-test <test_name>`                 | Watch one test                     |
| `nu run watch-clippy`                               | Watch clippy                       |
| `nu run watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |

There's also a `run` script at the **top level folder** of the repo. It is intended to be used in a
CI/CD environment w/ all the required arguments supplied or in interactive mode, where the user will
be prompted for input.

| Command                      | Description                                                                                                                                                                                                                                            |
| ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `nu run all`                 | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD                                                                                                                                                                                  |
| `nu run build-full`          | This will build all the crates in the Rust workspace. And it will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
| `nu run install-cargo-tools` | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`, `flamegraph` will all be installed in one go)                                                                                         |
| `nu run check-licenses`      | Use `cargo-deny` to audit all licenses used in the Rust workspace                                                                                                                                                                                      |

License: Apache-2.0
