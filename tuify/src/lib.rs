/*
 *   Copyright (c) 2023 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

//! # r3bl-tuify
//!
//! r3bl_tuify is a Rust crate that allows you to add simple interactivity to your CLI app.
//!
//! r3bl_tuify crate can be used in two ways:
//! 1. As a library. This is useful if you want to add simple interactivity to your CLI
//!    app written in Rust. You can see an example of this in the `examples` folder in the
//!    `main_interactive.rs` file. You can run it using `cargo run --example
//!    main_interactive`.
//! 1. As a binary. This is useful if you want to use this crate as a command line tool.
//!    The binary target is called `rt`.
//!
//! Please check out the
//! [changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl_tuify)
//! to see how the library has evolved over time.
//!
//! ## How to use it as a library?
//!
//! Here's a demo of the library target of this crate in action.
//!
//! <video width="100%" controls>
//!   <source src="https://github.com/r3bl-org/r3bl-open-core/assets/22040032/46850043-4973-49fa-9824-58f32f21e96e" type="video/mp4"/>
//! </video>
//!
//! To install the crate as a library, add the following to your `Cargo.toml` file:
//!
//! ```toml
//! [dependencies]
//! r3bl_tuify = "0.1.24" # Get the latest version at the time you get this.
//! r3bl_rs_utils_core = "0.9.11" # Get the latest version at the time you get this.
//! ```
//!
//! The following example illustrates how you can use this as a library. The function that
//! does the work of rendering the UI is called
//! [`select_from_list`]. It takes a list of items and returns
//! the selected item or items (depending on the selection mode). If the user does not
//! select anything, it returns `None`. The function also takes the maximum height and
//! width of the display, and the selection mode (single select or multiple select).
//!
//! It works on macOS, Linux, and Windows. And is aware
//! of the terminal color output limitations of each. For eg, it uses Windows API on Windows for
//! keyboard input. And on macOS Terminal.app it restricts color output to a 256 color palette.
//!
//! ```rust
//! use r3bl_rs_utils_core::*;
//! use r3bl_tuify::*;
//! use std::io::Result;
//!
//! fn main() -> Result<()> {
//!     // Get display size.
//!     let max_width_col_count: usize = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
//!     let max_height_row_count: usize = 5;
//!
//!     let user_input = select_from_list(
//!         "Select an item".to_string(),
//!         [
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ]
//!         .iter()
//!         .map(|it| it.to_string())
//!         .collect(),
//!         max_height_row_count,
//!         max_width_col_count,
//!         SelectionMode::Single,
//!         StyleSheet::default(),
//!     );
//!
//!     match &user_input {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//! ## APIs
//!
//! We provide 2 APIs:
//!
//! - [`select_from_list`]: Use this API if you want to display a list of items with a single line header.
//! - [`select_from_list_with_multi_line_header`]: Use this API if you want to display a list of items
//!   with a multi line header.
//!
//! ### select_from_list
//!
//! Use this API if you want to display a list of items with a single line header.
//!
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/0ae722bb-8cd1-47b1-a293-1a96e84d24d0)
//!
//! #### `select_from_list` signature:
//!
//! [select_from_list]
//!
//! ```rust
//! use r3bl_rs_utils_core::*;
//! use r3bl_tuify::*;
//! use std::io::Result;
//!
//! fn main() -> Result<()> {
//!     // Get display size.
//!     let max_height_row_count: usize = 5;
//!     let user_input = select_from_list(
//!         "Select an item".to_string(),
//!         [
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ]
//!         .iter()
//!         .map(|it| it.to_string())
//!         .collect(),
//!         max_height_row_count,
//!         0,
//!         SelectionMode::Single,
//!         StyleSheet::default(),
//!     );
//!
//!     match &user_input {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### select_from_list_with_multi_line_header
//!
//! Use the `select_from_list_with_multi_line_header` API if you want to display a list of items with a
//! multi line header. The first 5 lines are all part of the multi line header.
//!
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/2f82a42c-f720-4bcb-925d-0d5ad0b0a3c9)
//!
//! #### `select_from_list_with_multi_line_header` signature:
//!
//! [select_from_list_with_multi_line_header]
//!
//! ```rust
//! use std::{io::Result, vec};
//!
//! use r3bl_ansi_color::{AnsiStyledText, Color, Style as RStyle};
//! use r3bl_rs_utils_core::*;
//! use r3bl_tuify::{
//!     components::style::StyleSheet,
//!     select_from_list_with_multi_line_header,
//!     SelectionMode,
//! };
//!
//! fn multi_select_instructions() -> Vec<Vec<AnsiStyledText<'static>>> {
//!     let up_and_down = AnsiStyledText {
//!         text: " Up or down:",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(9, 238, 211)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!     let navigate = AnsiStyledText {
//!         text: "     navigate",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(94, 103, 111)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!
//!     let line_1 = vec![up_and_down, navigate];
//!
//!     let space = AnsiStyledText {
//!         text: " Space:",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(255, 216, 9)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!     let select = AnsiStyledText {
//!         text: "          select or deselect item",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(94, 103, 111)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!
//!     let line_2 = vec![space, select];
//!
//!     let esc = AnsiStyledText {
//!         text: " Esc or Ctrl+C:",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(255, 132, 18)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!     let exit = AnsiStyledText {
//!         text: "  exit program",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(94, 103, 111)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!
//!     let line_3 = vec![esc, exit];
//!     let return_key = AnsiStyledText {
//!         text: " Return:",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(234, 0, 196)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!     let confirm = AnsiStyledText {
//!         text: "         confirm selection",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(94, 103, 111)),
//!             RStyle::Background(Color::Rgb(14, 17, 23)),
//!         ],
//!     };
//!     let line_4 = vec![return_key, confirm];
//!     vec![line_1, line_2, line_3, line_4]
//! }
//!
//! fn main() -> Result<()> {
//!    let header = AnsiStyledText {
//!         text: " Please select one or more items. This is a really long heading that just keeps going and if your terminal viewport is small enough, this heading will be clipped",
//!         style: &[
//!             RStyle::Foreground(Color::Rgb(171, 204, 242)),
//!             RStyle::Background(Color::Rgb(31, 36, 46)),
//!         ],
//!     };
//!
//!     let mut instructions_and_header: Vec<Vec<AnsiStyledText>> = multi_select_instructions();
//!     instructions_and_header.push(vec![header]);
//!
//!     let user_input = select_from_list_with_multi_line_header(
//!         instructions_and_header,
//!         [
//!             "item 1 of 13",
//!             "item 2 of 13",
//!             "item 3 of 13",
//!             "item 4 of 13",
//!             "item 5 of 13",
//!             "item 6 of 13",
//!             "item 7 of 13",
//!             "item 8 of 13",
//!             "item 9 of 13",
//!             "item 10 of 13",
//!             "item 11 of 13",
//!             "item 12 of 13",
//!             "item 13 of 13",
//!         ]
//!         .iter()
//!         .map(|it| it.to_string())
//!         .collect(),
//!         Some(6),
//!         None,
//!         SelectionMode::Multiple,
//!         StyleSheet::default(),
//!     );
//!     match &user_input {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!    Ok(())
//! }
//! ```
//!
//! ## How to use it as a binary?
//! <a id="markdown-how-to-use-it-as-a-binary%3F" name="how-to-use-it-as-a-binary%3F"></a>
//!
//! Here's a demo of the binary target of this crate in action.
//!
//! <video width="100%" controls>
//!   <source src="https://github-production-user-asset-6210df.s3.amazonaws.com/2966499/267427392-2b42db72-cd62-4ea2-80ae-ccc01008190c.mp4" type="video/mp4"/>
//! </video>
//!
//! You can install the binary using `cargo install r3bl_tuify` (from crates.io). Or
//! `cargo install --path .` from source. `rt` is a command line tool that allows you to select one of the options from the list that is passed into it
//! via `stdin`. It supports both `stdin` and `stdout` piping.
//!
//! Here are the command line arguments that it accepts:
//! 1. `-s` or `--selection-mode` - Allows you to select the selection mode. There are two
//!    options: `single` and `multiple`.
//! 1. `-c` or `--command-to-run-with-selection` - Allows you to specify the command to
//!    run with the selected item. For example `"echo foo \'%\'"` simply prints each
//!    selected item.
//! 1. `-t` or `--tui-height` - Optionally allows you to set the height of the TUI. The
//!    default is 5.
//!
//! ### Interactive user experience
//! <a id="markdown-interactive-user-experience" name="interactive-user-experience"></a>
//!
//! Typically a CLI app is not interactive. You can pass commands, subcommands, options, and
//! arguments to it, but if you get something wrong, then you get an error and have to start
//! all over again. This "conversation" style interface might require a lot of trial and error
//! to get the desired result.
//!
//! The following is an example of using the binary with many subcommands, options, and arguments.
//!
//! ```shell
//! cat TODO.todo | cargo run -- select-from-list \
//!     --selection-mode single \
//!     --command-to-run-with-each-selection "echo %"
//! ```
//!
//! Here's a video of this in action.
//!
//! <!-- tuify-long-command -->
//! <video width="100%" controls>
//!   <source src="https://github.com/r3bl-org/r3bl-open-core/assets/2966499/c9b49bfb-b811-460e-a844-fe260eaa860a" type="video/mp4"/>
//! </video>
//!
//! What does this do?
//!
//! 1. `cat TODO.todo` - prints the contents of the `TODO.todo` file to `stdout`.
//! 1. `|` - pipes the output of the previous command to the next command, which is `rt` (ie,
//!    the binary target of this crate).
//! 1. `cargo run --` - runs the `rt` debug binary in the target folder.
//! 1. `select-from-list` - runs the `rt` binary with the `select-from-list`
//!    subcommand. This subcommand requires 2 arguments: `--selection-mode` and
//!    `--command-to-run-with-each-selection`. Whew! This is getting long!
//! 1. `--selection-mode single` - sets the selection mode to `single`. This means that the
//!     user can only select one item from the list. What list? The list that is piped in from
//!     the previous command (ie, `cat TODO.todo`).
//! 1. `--command-to-run-with-each-selection "echo %"` - sets the command to run with each
//!     selection. In this case, it is `echo %`. The `%` is a placeholder for the selected
//!     item. So if the user selects `item 1`, then the command that will be run is `echo item
//!     1`. The `echo` command simply prints the selected item to `stdout`.
//!
//! Now that is a lot to remember. It is helpful to use `clap` to provide nice command line help but
//! that are still quite a few things that you have to get right for this command to work.
//!
//! It doesn't have to be this way. The binary can be interactive along with
//! the use of `clap` to specify some of the subcommands, and arguments. It doesn't
//! have to be an all or nothing approach. We can have the best of both worlds. The following
//! videos illustrate what happens when:
//!
//! 1. `--selection-mode` and `--command-to-run-with-each-selection` are *not* passed in the
//!    command line.
//!    ```shell
//!    cat TODO.todo | cargo run -- select-from-list
//!    ```
//!
//!    Here are the 3 scenarios that can happen:
//!
//!    - The user first chooses `single` selection mode (using a list selection component),
//!      and then types in `echo %` in the terminal, as the command to run with each
//!      selection. This is an
//!      interactive scenario since the user has to provide 2 pieces of information:  the selection mode, and the command to run with each
//!      selection. They didn't provide this upfront when they ran the command.
//!      <!-- tuify-interactive-happy-path -->
//!      <video width="100%" controls>
//!        <source src="https://github.com/r3bl-org/r3bl-open-core/assets/2966499/51de8867-513b-429f-aff2-63dd25d71c82" type="video/mp4"/>
//!      </video>
//!
//!    - Another scenario is that the user does not provide the required information even when
//!      prompted interactively. In this scenario, the program exits with an error and help
//!      message.
//!
//!      Here they don't provide what `selection-mode` they want. And they don't provide what
//!      `command-to-run-with-each-selection` they want. Without this information the program
//!      can't continue, so it exits and provides some help message.
//!      <!-- tuify-interactive-unhappy-path -->
//!      <video width="100%" controls>
//!        <source src="https://github.com/r3bl-org/r3bl-open-core/assets/2966499/664d0367-90fd-4f0a-ad87-3f4745642ad0" type="video/mp4"/>
//!      </video>
//!
//! 1. `--selection-mode` is *not* passed in the command line. So it only interactively
//!    prompts the user for this piece of information. Similarly, if the user does not provide
//!    this information, the app exits and provides a help message.
//!    ```shell
//!    cat TODO.todo | cargo run -- select-from-list --command-to-run-with-each-selection "echo %"
//!    ```
//!    <!-- tuify-interactive-selection-mode-not-provided -->
//!      <video width="100%" controls>
//!        <source src="https://github.com/r3bl-org/r3bl-open-core/assets/2966499/be65d9b2-575b-47c0-8291-110340bd2fe7" type="video/mp4"/>
//!      </video>
//!
//! 1. `--command-to-run-with-each-selection` is *not* passed in the command line. So it only
//!    interactively prompts the user for this piece of information. Similarly, if the user
//!    does not provide this information, the app exits and provides a help message.
//!    ```shell
//!    cat TODO.todo | cargo run -- select-from-list --selection-mode single
//!    ```
//!    <!-- tuify-interactive-command-to-run-with-selection-not-provided -->
//!      <video width="100%" controls>
//!        <source src="https://github.com/r3bl-org/r3bl-open-core/assets/2966499/d8d7d419-c85e-4c10-bea5-345aa31a92a3" type="video/mp4"/>
//!      </video>
//!
//! ### Paths
//!
//! There are a lot of different execution paths that you can take with this relatively
//! simple program. Here is a list.
//!
//! - Happy paths:
//!   1. `rt` - prints help.
//!   1. `cat Cargo.toml | rt -s single -c "echo foo \'%\'"` - `stdin` is piped
//!      in, and it prints the user selected option to `stdout`.
//!   1. `cat Cargo.toml | rt -s multiple -c "echo foo \'%\'"` - `stdin` is piped
//!      in, and it prints the user selected option to `stdout`.
//!
//! - Unhappy paths (`stdin` is _not_ piped in and, or `stdout` _is_ piped out):
//!   1. `rt -s single` - expects `stdin` to be piped in, and prints help.
//!   1. `rt -s multiple` - expects `stdin` to be piped in, and prints help.
//!   1. `ls -la | rt -s single | xargs -0` - does not expect `stdout` to be piped out,
//!      and prints help.
//!   1. `ls -la | rt -s multiple | xargs -0` - does not expect `stdout` to be piped out,
//!      and prints help.
//!
//! > Due to how unix pipes are implemented, it is not possible to pipe the
//! > `stdout` of this command to anything else. Unix pipes are non-blocking. So there is no
//! > way to stop the pipe "midway". This is why `rt` displays an error when the `stdout` is
//! > piped out. It is not possible to pipe the `stdout` of `rt` to another command. Instead,
//! > the `rt` binary simply takes a command that will run after the user has made their
//! > selection. Using the selected item(s) and applying them to this command.
//!
//! ## Style the components
//!
//! ### Choose one of the 3 built-in styles
//!
//! Built-in styles are called `default`, `sea_foam_style`, and `hot_pink_style`. You can find them in the `style.rs` file (tuify/src/components/style.rs).
//!
//! ### default style
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/eaf990a4-1c33-4783-9f39-82af42568183)
//!
//! ### sea_foam_style
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/fc414f56-2f72-4d3a-86eb-bfd732b66bd1)
//!
//! ### hot_pink_style
//! ![image](https://github.com/r3bl-org/r3bl-open-core/assets/22040032/06c155f9-11a9-416d-8056-cb4c741ac3d7)
//!
//! To use one of the built-in styles, simply pass it as an argument to the `select_from_list` function.
//!
//! ```rust
//! use r3bl_rs_utils_core::*;
//! use r3bl_tuify::*;
//! use std::io::Result;
//!
//! fn main() -> Result<()> {
//!     // 🎨 Uncomment the lines below to choose the other 2 built-in styles.
//!     // let default_style = StyleSheet::default();
//!     // let hot_pink_style = StyleSheet::hot_pink_style();
//!     let sea_foam_style = StyleSheet::sea_foam_style();
//!
//!     let max_width_col_count: usize = get_size().map(|it| it.col_count).unwrap_or(ch!(80)).into();
//!     let max_height_row_count: usize = 5;
//!
//!     let user_input = select_from_list(
//!         "Select an item".to_string(),
//!         [
//!             "item 1", "item 2", "item 3", "item 4", "item 5", "item 6", "item 7", "item 8",
//!             "item 9", "item 10",
//!         ]
//!         .iter()
//!         .map(|it| it.to_string())
//!         .collect(),
//!         max_height_row_count,
//!         max_width_col_count,
//!         SelectionMode::Single,
//!         sea_foam_style,  // 🖌️ or default_style or hot_pink_style
//!     );
//!
//!     match &user_input {
//!         Some(it) => {
//!             println!("User selected: {:?}", it);
//!         }
//!         None => println!("User did not select anything"),
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Create your style
//!
//! To create your style, you need to create a `StyleSheet` struct and pass it as an argument to the `select_from_list` function.
//!
//! ```rust
//! use std::io::Result;
//! use r3bl_ansi_color::{AnsiStyledText, Color};
//! use r3bl_tuify::{components::style::{Style, StyleSheet},
//!                 select_from_list,
//!                 SelectionMode};
//!
//! fn main() -> Result<()> {
//!    // This is how you can define your custom style.
//!    // For each Style struct, you can define different style overrides.
//!    // Please take a look at the Style struct to see what you can override.
//!    let my_custom_style = StyleSheet {
//!       focused_and_selected_style: Style {
//!             fg_color: Color::Rgb(255, 244, 0),
//!             bg_color: Color::Rgb(15, 32, 66),
//!             ..Style::default()
//!       },
//!       focused_style: Style {
//!             fg_color: Color::Rgb(255, 244, 0),
//!             ..Style::default()
//!       },
//!       unselected_style: Style { ..Style::default() },
//!       selected_style: Style {
//!             fg_color: Color::Rgb(203, 170, 250),
//!             bg_color: Color::Rgb(15, 32, 66),
//!             ..Style::default()
//!       },
//!       header_style: Style {
//!             fg_color: Color::Rgb(171, 204, 242),
//!             bg_color: Color::Rgb(31, 36, 46),
//!             ..Style::default()
//!       },
//!    };
//!
//!    // Then pass `my_custom_style` as the last argument to the `select_from_list` function.
//!    let user_input = select_from_list(
//!       "Multiple select".to_string(),
//!       ["item 1 of 3", "item 2 of 3", "item 3 of 3"]
//!          .iter()
//!          .map(|it| it.to_string())
//!          .collect(),
//!       6, // max_height_row_count
//!       80, // max_width_col_count
//!       SelectionMode::Multiple,
//!       my_custom_style,
//!    );
//!
//!    match &user_input {
//!       Some(it) => {
//!          println!("User selected: {:?}", it);
//!       }
//!       None => println!("User did not select anything"),
//!    }
//!    Ok(())
//! }
//! ```
//!
//! ## Build, run, test tasks
//!
//! ### Prerequisites
//!
//! 🌠 For these to work you have to install the Rust toolchain, `nu`, `cargo-watch`,
//! `bat`, and `flamegraph` on your system. Here are the instructions:
//!
//! 1. Install the Rust toolchain using `rustup` by following the instructions
//!    [here](https://rustup.rs/).
//! 1. Install `cargo-watch` using `cargo install cargo-watch`.
//! 1. Install `flamegraph` using `cargo install flamegraph`.
//! 1. Install `bat` using `cargo install bat`.
//! 1. Install [`nu`](https://crates.io/crates/nu) shell on your system using `cargo install
//!    nu`. It is available for Linux, macOS, and Windows.
//!
//! ### Nu shell scripts to build, run, test, etc.
//!
//! Go to the `tuify` folder and run the commands below. These commands are defined in the `./run` folder.
//!
//! | Command                                | Description                                |
//! | -------------------------------------- | ------------------------------------------ |
//! | `nu run examples`                      | Run examples in the `./examples` folder    |
//! | `nu run piped`                         | Run binary with piped input                |
//! | `nu run build`                         | Build                                      |
//! | `nu run clean`                         | Clean                                      |
//! | `nu run all`                           | All                                        |
//! | `nu run examples-with-flamegraph-profiling` | Run examples with flamegraph profiling |
//! | `nu run test`                          | Run tests                                  |
//! | `nu run clippy`                        | Run clippy                                 |
//! | `nu run docs`                          | Build docs                                 |
//! | `nu run serve-docs`                    | Serve docs over VSCode Remote SSH session. |
//! | `nu run upgrade-deps`                  | Upgrade deps                               |
//! | `nu run rustfmt`                       | Run rustfmt                                |
//!
//! The following commands will watch for changes in the source folder and re-run:
//!
//! | Command                                             | Description                        |
//! | --------------------------------------------------- | ---------------------------------- |
//! | `nu run watch-examples`                             | Watch run examples                 |
//! | `nu run watch-all-tests`                            | Watch all test                     |
//! | `nu run watch-one-test <test_name>`                 | Watch one test                     |
//! | `nu run watch-clippy`                               | Watch clippy                       |
//! | `nu run watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
//!
//! There's also a `run` script at the **top level folder** of the repo. It is intended to
//! be used in a CI/CD environment w/ all the required arguments supplied or in
//! interactive mode, where the user will be prompted for input.
//!
//! | Command                       | Description                        |
//! | ----------------------------- | ---------------------------------- |
//! | `nu run all`                  | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD |
//! | `nu run build-full`           | This will build all the crates in the Rust workspace. It will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
//! | `nu run install-cargo-tools`  | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`,and `flamegraph` will all be installed in one go) |
//! | `nu run check-licenses`       | Use `cargo-deny` to audit all licenses used in the Rust workspace |
//!
//!
//! ## References
//!
//! CLI UX guidelines:
//!
//! - <https://rust-cli-recommendations.sunshowers.io/handling-arguments.html>
//! - <https://rust-cli-recommendations.sunshowers.io/configuration.html>
//! - <https://rust-cli-recommendations.sunshowers.io/hierarchical-config.html>
//! - <https://rust-cli-recommendations.sunshowers.io/hierarchical-config.html>
//! - <https://docs.rs/clap/latest/clap/_derive/#overview>
//! - <https://clig.dev/#foreword>
//!
//! ANSI escape codes:
//!
//! - <https://notes.burke.libbey.me/ansi-escape-codes/>
//! - <https://en.wikipedia.org/wiki/ANSI_escape_code>
//! - <https://www.asciitable.com/>
//! - <https://commons.wikimedia.org/wiki/File:Xterm_256color_chart.svg>
//! - <https://www.ditig.com/256-colors-cheat-sheet>
//! - <https://stackoverflow.com/questions/4842424/list-of-ansi-color-escape-sequences>
//! - <https://www.compuphase.com/cmetric.htm>

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

pub mod components;
pub mod constants;
pub mod event_loop;
pub mod function_component;
pub mod keypress;
pub mod public_api;
pub mod scroll;
pub mod state;
pub mod term;
pub mod test_utils;

pub use components::*;
pub use constants::*;
pub use event_loop::*;
pub use function_component::*;
pub use keypress::*;
pub use public_api::*;
pub use scroll::*;
pub use state::*;
pub use term::*;
pub use test_utils::*;

/// Enable file logging. You can use `tail -f log.txt` to watch the logs.
pub const DEVELOPMENT_MODE: bool = false;
