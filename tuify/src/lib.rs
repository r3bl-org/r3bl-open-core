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
//! This crate can be used in two ways:
//! 1. As a library. This is useful if you want to add simple interactivity to your CLI
//!    app written in Rust. You can see an example of this in the `examples` folder in the
//!    `main_interactive.rs` file. You can run it using `cargo run --example
//!    main_interactive`.
//! 1. As a binary. This is useful if you want to use this crate as a command line tool.
//!    The binary target is called `rt`.
//!
//! ## How to use it as a library?
//!
//! Here's a demo of the library target of this crate in action.
//!
//! <https://github-production-user-asset-6210df.s3.amazonaws.com/2966499/266504562-c6717052-780f-4ae0-8ecf-e57beca49929.mp4>
//!
//! The following example illustrates how you can use this as a library. The function that
//! does the work of rendering the UI is called
//! [`select_from_list`](crate::select_from_list). It takes a list of items, and returns
//! the selected item or items (depending on the selection mode). If the user does not
//! select anything, it returns `None`. The function also takes the maximum height and
//! width of the display, and the selection mode (single select or multiple select).
//!
//! It works on macOS, Linux, and Windows. And is aware
//! of terminal color output limitations of each. For eg, it uses Windows API on Windows for
//! keyboard input. And on macOS Terminal.app it restricts color output to a 256 color palette.
//!
//! > Currently only single selection is implemented. An issue is open to add this
//! > feature: <https://github.com/r3bl-org/r3bl_rs_utils/issues> if you would like to
//! > [contribute](https://github.com/r3bl-org/r3bl_rs_utils/contribute).
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
//! ## How to use it as a binary?
//!
//! Here's a demo of the binary target of this crate in action.
//!
//! <https://user-images.githubusercontent.com/2966499/266860855-dce05d87-327d-48f7-b063-45987177159c.mp4>
//!
//! You can install the binary using `cargo install r3bl_tuify` (from crates.io). Or
//! `cargo install --path .` from source. Once installed, you can `rt` is a command line
//! tool that allows you to select one of the options from the list that is passed into it
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
//! > Currently only single selection is implemented. An issue is open to add this
//! > feature: <https://github.com/r3bl-org/r3bl_rs_utils/issues> if you would like to
//! > [contribute](https://github.com/r3bl-org/r3bl_rs_utils/contribute).
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
//! ### Docs
//!
//! - [clap docs](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html)
//! - [clap print help programmatically](https://github.com/clap-rs/clap/issues/672#issuecomment-1159332810)
//! - [clap print help declaratively](https://docs.rs/clap/latest/clap/struct.Command.html#method.arg_required_else_help)
//! - [clap cookbook git example](https://docs.rs/clap/latest/clap/_derive/_cookbook/git_derive/index.html)
//! - [Pipe detection](https://developerlife.com/2022/03/02/rust-grep-cli-app/)

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

pub mod components;
pub mod event_loop;
pub mod keypress;
pub mod public_api;
pub mod react;
pub mod scroll;
pub mod state;
pub mod term;

pub use components::*;
pub use event_loop::*;
pub use keypress::*;
pub use public_api::*;
pub use react::*;
pub use scroll::*;
pub use state::*;
pub use term::*;

/// Enable file logging. You can use `tail -f log.txt` to watch the logs.
pub const TRACE: bool = true;
