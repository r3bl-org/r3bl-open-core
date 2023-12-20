<p align="center">
  <img src="r3bl-term.svg" height="128px">
</p>

# r3bl-cmdr
<a id="markdown-r3bl-cmdr" name="r3bl-cmdr"></a>

<!-- TOC -->

- [Context](#context)
- [Run giti binary target](#run-giti-binary-target)
- [Run r3bl-cmdr or rc binary target](#run-r3bl-cmdr-or-rc-binary-target)
- [Build, run, test tasks](#build-run-test-tasks)
  - [Prerequisites](#prerequisites)
  - [Nu shell scripts to build, run, test etc.](#nu-shell-scripts-to-build-run-test-etc)

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

## Run `giti` binary target
<a id="markdown-run-giti-binary-target" name="run-giti-binary-target"></a>

1. Go to the `cmdr` folder in your terminal
2. Run `nu run install` to install `giti` locally to `~/.cargo/bin`
3. Run `giti` from anywhere on your system
4. To delete one or more branches in your repo run `giti branch delete`.
5. If you want to generate log output for `giti`, run `giti -l`. For example, `giti -l
   branch delete`.

[![asciicast](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg.svg)](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg)

## Run `r3bl-cmdr` or `rc` binary target
<a id="markdown-run-r3bl-cmdr-or-rc-binary-target" name="run-r3bl-cmdr-or-rc-binary-target"></a>


`r3bl-cmdr` is the second thing that's described above. It contains a set of apps for developers by
developers. It is engineered to enhance your:

- ❯ 🚀 productivity
- ❯ 🌍 efficiency
- ❯ 📖 knowledge capture & sharing
- ❯ 🛣️ workflow management

Our goal is to put a smile on your face every time you use this product.

## Build, run, test tasks
<a id="markdown-build%2C-run%2C-test-tasks" name="build%2C-run%2C-test-tasks"></a>

### Prerequisites
<a id="markdown-prerequisites" name="prerequisites"></a>

🌠 In order for these to work you have to install the Rust toolchain, `nu`, `cargo-watch`, `bat`,
and `flamegraph` on your system. Here are the instructions:

1. Install the Rust toolchain using `rustup` by following the instructions
   [here](https://rustup.rs/).
1. Install `cargo-watch` using `cargo install cargo-watch`.
1. Install `flamegraph` using `cargo install flamegraph`.
1. Install `bat` using `cargo install bat`.
1. Install [`nu`](https://crates.io/crates/nu) shell on your system using `cargo install nu`. It is
   available for Linux, macOS, and Windows.

### Nu shell scripts to build, run, test etc.
<a id="markdown-nu-shell-scripts-to-build%2C-run%2C-test-etc." name="nu-shell-scripts-to-build%2C-run%2C-test-etc."></a>

| Command                                   | Description                                       |
| ----------------------------------------- | ------------------------------------------------- |
| `nu run.nu run`                           | Run the default binary target                     |
| `nu run.nu install`                       | Install `giti` and `rc` locally to `~/.cargo/bin` |
| `nu run.nu build`                         | Build                                             |
| `nu run.nu clean`                         | Clean                                             |
| `nu run.nu all`                           | All                                               |
| `nu run.nu run-with-flamegraph-profiling` | Run examples with flamegraph profiling            |
| `nu run.nu test`                          | Run tests                                         |
| `nu run.nu clippy`                        | Run clippy                                        |
| `nu run.nu docs`                          | Build docs                                        |
| `nu run.nu serve-docs`                    | Serve docs over VSCode Remote SSH session.        |
| `nu run.nu upgrade-deps`                  | Upgrade deps                                      |
| `nu run.nu rustfmt`                       | Run rustfmt                                       |

The following commands will watch for changes in the source folder and re-run:

| Command                                                | Description                        |
| ------------------------------------------------------ | ---------------------------------- |
| `nu run.nu watch-run`                                  | Watch run                          |
| `nu run.nu watch-all-tests`                            | Watch all test                     |
| `nu run.nu watch-one-test <test_name>`                 | Watch one test                     |
| `nu run.nu watch-clippy`                               | Watch clippy                       |
| `nu run.nu watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
