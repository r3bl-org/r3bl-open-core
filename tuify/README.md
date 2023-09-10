# r3bl_tuify
<a id="markdown-r3bl_tuify" name="r3bl_tuify"></a>

<!-- TOC -->

- [What does it do?](#what-does-it-do)
- [How to use it as a library?](#how-to-use-it-as-a-library)
- [How to use it as a binary?](#how-to-use-it-as-a-binary)
  - [Paths](#paths)
- [Build, run, test tasks](#build-run-test-tasks)
  - [Prerequisites](#prerequisites)
  - [Just commands](#just-commands)
- [References](#references)

<!-- /TOC -->

## What does it do?
<a id="markdown-what-does-it-do%3F" name="what-does-it-do%3F"></a>

This crate can be used in two ways:
1. As a library. This is useful if you want to add simple interactivity to your CLI
   app written in Rust. You can see an example of this in the `examples` folder in the
   `main_interactive.rs` file. You can run it using `cargo run --example main_interactive`.
1. As a binary. This is useful if you want to use this crate as a command line tool.
   The binary target is called `rt`.

## How to use it as a library?
<a id="markdown-how-to-use-it-as-a-library%3F" name="how-to-use-it-as-a-library%3F"></a>

Here's a demo of the library target of this crate in action.

https://user-images.githubusercontent.com/2966499/266870250-9af806a6-9d2a-48b3-9c02-22d8a05cbdc3.mp4

The following example illustrates how you can use this as a library. The function that
does the work of rendering the UI is called
[`select_from_list`](crate::select_from_list). It takes a list of items, and returns
the selected item or items (depending on the selection mode). If the user does not
select anything, it returns `None`. The function also takes the maximum height and
width of the display, and the selection mode (single select or multiple select).

It works on macOS, Linux, and Windows. And is aware
of terminal color output limitations of each. For eg, it uses Windows API on Windows for
keyboard input. And on macOS Terminal.app it restricts color output to a 256 color palette.

> Currently only single selection is implemented. An issue is open to add this
> feature: <https://github.com/r3bl-org/r3bl_rs_utils/issues> if you would like to
> [contribute](https://github.com/r3bl-org/r3bl_rs_utils/contribute).

```rust
use r3bl_rs_utils_core::*;
use r3bl_tuify::*;
use std::io::Result;

fn main() -> Result<()> {
    // Get display size.
    let max_width_col_count: usize = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
    let max_height_row_count: usize = 5;

    let user_input = select_from_list(
        [
            "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
            "item 9", "item 10",
        ]
        .iter()
        .map(|it| it.to_string())
        .collect(),
        max_height_row_count,
        max_width_col_count,
        SelectionMode::Single,
    );

    match &user_input {
        Some(it) => {
            println!("User selected: {:?}", it);
        }
        None => println!("User did not select anything"),
    }

    Ok(())
}
```

## How to use it as a binary?
<a id="markdown-how-to-use-it-as-a-binary%3F" name="how-to-use-it-as-a-binary%3F"></a>

Here's a demo of the binary target of this crate in action.

https://user-images.githubusercontent.com/2966499/266860855-dce05d87-327d-48f7-b063-45987177159c.mp4

You can install the binary using `cargo install r3bl_tuify` (from crates.io). Or
`cargo install --path .` from source. Once installed, you can `rt` is a command line
tool that allows you to select one of the options from the list that is passed into it
via `stdin`. It supports both `stdin` and `stdout` piping.

Here are the command line arguments that it accepts:
1. `-s` or `--selection-mode` - Allows you to select the selection mode. There are two
  options: `single` and `multiple`.
1. `-c` or `--command-to-run-with-selection` - Allows you to specify the command to
  run with the selected item. For example `"echo foo \'%\'"` simply prints each
  selected item.
1. `-t` or `--tui-height` - Optionally allows you to set the height of the TUI. The
  default is 5.

> Currently only single selection is implemented. An issue is open to add this
> feature: <https://github.com/r3bl-org/r3bl_rs_utils/issues> if you would like to
> [contribute](https://github.com/r3bl-org/r3bl_rs_utils/contribute).

### Paths
<a id="markdown-paths" name="paths"></a>

There are a lot of different execution paths that you can take with this relatively
simple program. Here is a list.

- Happy paths:
  1. `rt` - prints help.
  1. `cat Cargo.toml | rt -s single -c "echo foo \'%\'"` - `stdin` is piped
    in, and it prints the user selected option to `stdout`.
  1. `cat Cargo.toml | rt -s multiple -c "echo foo \'%\'"` - `stdin` is piped
    in, and it prints the user selected option to `stdout`.

- Unhappy paths (`stdin` is _not_ piped in and, or `stdout` _is_ piped out):
  1. `rt -s single` - expects `stdin` to be piped in, and prints help.
  1. `rt -s multiple` - expects `stdin` to be piped in, and prints help.
  1. `ls -la | rt -s single | xargs -0` - does not expect `stdout` to be piped out,
    and prints help.
  1. `ls -la | rt -s multiple | xargs -0` - does not expect `stdout` to be piped out,
    and prints help.

> Due to the way in which unix pipes are implemented, it is not possible to pipe the
> `stdout` of this command to anything else. Unix pipes are non blocking. So there is no
> way to stop the pipe "mid way". This is why `rt` displays an error when the `stdout` is
> piped out. It is not possible to pipe the `stdout` of `rt` to another command. Instead,
> the `rt` binary simply takes a command that it will run after the user has made their
> selection. Using the selected item(s) and applying them to this command.

## Build, run, test tasks
<a id="markdown-build%2C-run%2C-test-tasks" name="build%2C-run%2C-test-tasks"></a>

### Prerequisites
<a id="markdown-prerequisites" name="prerequisites"></a>

🌠 In order for these to work you have to install the Rust toolchain and `just` and
`cargo-watch`:

1. Install the Rust toolchain using `rustup` by following the instructions
   [here](https://rustup.rs/).
1. Install `cargo-watch` using `cargo install cargo-watch`.
1. Install `flamegraph` using `cargo install flamegraph`.
1. Install [`just`](https://just.systems/man/en/chapter_4.html) `just` on your system using
   `cargo install just`. It is available for Linux, macOS, and Windows.
   - If you want shell completions for `just` you can follow [these
     instructions](https://github.com/casey/just#shell-completion-scripts).
   - If you install `just` using `cargo install just` or `brew install just` you will
     not get shell completions without doing one extra configuration step. So on Linux
     it is best to use `sudo apt install -y just` if you want them.

### Just commands
<a id="markdown-just-commands" name="just-commands"></a>

> Note to run a just command named `all` on Windows, you have to use the following:
> `just --shell powershell.exe --shell-arg -c all`

- Build: `just build`
- Clean: `just clean`
- Run examples: `just run`
- Run examples with release flag: `just run-release`
- Run examples with flamegraph profiling: `just run-flamegraph`
- Run tests: `just test`
- Run clippy: `just clippy`
- Build docs: `just docs`
- Serve docs: `just serve-docs`. This is only useful if you SSH into a remote machine via
  VSCode (where you build and serve the docs) and want to view the docs in a browser on
  your local machine.
- Upgrade deps: `just upgrade-deps`
- Run rustfmt: `just rustfmt`

The following commands will watch for changes in the source folder and re-run:

- Watch run: `just watch-run`
- Watch all test: `just watch-all-tests`
- Watch one test: `just watch-one-test <test_name>`
- Watch clippy: `just watch-clippy`
- Watch macro expansion for one test: `just watch-macro-expansion-one-test <test_name>`

## References
<a id="markdown-references" name="references"></a>

- https://notes.burke.libbey.me/ansi-escape-codes/
- https://en.wikipedia.org/wiki/ANSI_escape_code
- https://www.asciitable.com/
- https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg
- https://www.ditig.com/256-colors-cheat-sheet
- https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences
- https://www.compuphase.com/cmetric.htm
