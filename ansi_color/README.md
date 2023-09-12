# r3bl_ansi_color crate
<a id="markdown-r3bl_ansi_color-crate" name="r3bl_ansi_color-crate"></a>

<!-- TOC -->

- [What does it do?](#what-does-it-do)
- [How to use it?](#how-to-use-it)
- [Build, run, test tasks](#build-run-test-tasks)
  - [Prerequisites](#prerequisites)
  - [Commands to run](#commands-to-run)
- [References](#references)
- [Why make a new crate for this?](#why-make-a-new-crate-for-this)

<!-- /TOC -->

## What does it do?
<a id="markdown-what-does-it-do%3F" name="what-does-it-do%3F"></a>

Rust crate to generate formatted ANSI 256 (8-bit) and truecolor (24-bit) color output to stdout. On
macOS, the default Terminal.app does not support truecolor, so ANSI 256 colors are used instead.

> This crate performs its own detection of terminal color capability heuristically. And does not
> use other crates to perform this function.

Here's a screenshot of running the `main` example on various operating systems:

| ![Linux screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/ansi_color/docs/screenshot_linux.png?raw=true) |
|:--:|
| *Running on Linux Tilix* |

| ![Windows screenshot](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/ansi_color/docs/screenshot_windows.png?raw=true) |
|:--:|
| *Running on Windows Terminal* |

| ![macOS screenshot Terminal app](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/ansi_color/docs/screenshot_macos_terminal_app.png?raw=true) |
|:--:|
| *Running on macOS Terminal app (note ANSI 256 runtime detection)* |

| ![macOS screenshot iTerm app](https://raw.githubusercontent.com/r3bl-org/r3bl_rs_utils/main/ansi_color/docs/screenshot_macos_iterm_app.png?raw=true) |
|:--:|
| *Running on macOS iTerm app (note Truecolor runtime detection)* |

## How to use it?
<a id="markdown-how-to-use-it%3F" name="how-to-use-it%3F"></a>

The main struct that we have to consider is `AnsiStyledText`. It has two fields:

- `text` - the text to print.
- `style` - a list of styles to apply to the text.

Here's an example.

```rust
AnsiStyledText {
    text: "Print a formatted (bold, italic, underline) string w/ ANSI color codes.",
    style: &[
        Style::Bold,
        Style::Italic,
        Style::Underline,
        Style::Foreground(Color::Rgb(50, 50, 50)),
        Style::Background(Color::Rgb(100, 200, 1)),
    ],
}
.println();
```

Please a look at the
[`main` example](https://github.com/r3bl-org/r3bl_ansi_color/blob/main/examples/main.rs) to get a
better idea of how to use this crate.

## Build, run, test tasks
<a id="markdown-build%2C-run%2C-test-tasks" name="build%2C-run%2C-test-tasks"></a>

### Prerequisites
<a id="markdown-prerequisites" name="prerequisites"></a>

🌠 In order for these to work you have to install the Rust toolchain and the following
crates `cargo-cache`, `cargo-watch`, `flamegraph`, and `nu` using these instructions:

1. Install the Rust toolchain using `rustup` by following the instructions
   [here](https://rustup.rs/).
1. Install `cargo-cache` using `cargo install cargo-cache`.
1. Install `cargo-watch` using `cargo install cargo-watch`.
1. Install `flamegraph` using `cargo install flamegraph`.
1. Install `nu` using `cargo install nu`.

### Commands to run
<a id="markdown-commands-to-run" name="commands-to-run"></a>

The script to run is `run.nu`. It works on Linux, macOS, and Windows. Please make sure
that the [pre-requisites](#prerequisites) are installed.

- Build: `nu run.nu build`
- Clean: `nu run.nu clean`
- Run examples: `nu run.nu run`
- Run examples with release flag: `nu run.nu run-release`
- Run examples with flamegraph profiling: `nu run.nu run-flamegraph`
- Run tests: `nu run.nu test`
- Run clippy: `nu run.nu clippy`
- Build docs: `nu run.nu docs`
- Serve docs: `nu run.nu serve-docs`. This is only useful if you SSH into a remote machine via
  VSCode (where you build and serve the docs) and want to view the docs in a browser on
  your local machine.
- Upgrade deps: `nu run.nu upgrade-deps`
- Run rustfmt: `nu run.nu rustfmt`

The following commands will watch for changes in the source folder and re-run:

- Watch run: `nu run.nu watch-run`
- Watch all test: `nu run.nu watch-all-tests`
- Watch one test: `nu run.nu watch-one-test <test_name>`
- Watch clippy: `nu run.nu watch-clippy`
- Watch macro expansion for one test: `nu run.nu watch-macro-expansion-one-test <test_name>`

## References
<a id="markdown-references" name="references"></a>

- https://notes.burke.libbey.me/ansi-escape-codes/
- https://en.wikipedia.org/wiki/ANSI_escape_code
- https://www.asciitable.com/
- https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg
- https://www.ditig.com/256-colors-cheat-sheet
- https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences
- https://www.compuphase.com/cmetric.htm

## Why make a new crate for this?
<a id="markdown-why-make-a-new-crate-for-this%3F" name="why-make-a-new-crate-for-this%3F"></a>

- There are a few crates on crates.io that do similar things but they don't amenable
  licenses.
- Other crates simply ignore ANSI 256 colors and only support truecolor, even when they
  claim that they support it.
- And there are other crates which don't correctly report that macOS Terminal.app does not
  support truecolor and only supports ANSI 256 color.

Here are some links:

1. <https://github.com/rust-cli/concolor/issues/47>
1. <https://docs.rs/anstream/latest/anstream/>
1. <https://docs.rs/colorchoice/latest/colorchoice/>
1. <https://docs.rs/colorchoice-clap/latest/colorchoice_clap/>
1. <https://docs.rs/anstyle-query/latest/anstyle_query/fn.term_supports_ansi_color.html>
1. <https://crates.io/crates/anstyle-query>
1. <https://docs.rs/supports-color/2.0.0/supports_color/>
1. <https://crates.io/crates/ansi_colours>
1. <https://crates.io/crates/colored>
