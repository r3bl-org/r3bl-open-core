/*
 *   Copyright (c) 2024 R3BL LLC
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

//! <p align="center">
//!   <img src="r3bl-term.svg" height="128px">
//! </p>
//!
//! # r3bl-cmdr: Suite of TUI apps focused on developer productivity
//! <a id="markdown-r3bl-cmdr%3A-suite-of-tui-apps-focused-on-developer-productivity" name="r3bl-cmdr%3A-suite-of-tui-apps-focused-on-developer-productivity"></a>
//!
//! <!-- TOC -->
//!
//! - [Install the apps on your system using cargo](#install-the-apps-on-your-system-using-cargo)
//! - [Run giti binary target](#run-giti-binary-target)
//! - [Run edi binary target](#run-edi-binary-target)
//! - [Build, run, test tasks](#build-run-test-tasks)
//!   - [Prerequisites](#prerequisites)
//!   - [Nu shell scripts to build, run, test etc.](#nu-shell-scripts-to-build-run-test-etc)
//!
//! <!-- /TOC -->
//!
//! ## Install the apps on your system using cargo
//! <a id="markdown-install-the-apps-on-your-system-using-cargo" name="install-the-apps-on-your-system-using-cargo"></a>
//!
//! Two apps, `edi` and `giti`, that comprise `r3bl-cmdr` will put a smile on your face and
//! make you more productive. These apps are currently available as early access preview 🐣.
//!
//! - 😺 `giti` - an interactive git CLI app designed to give you more confidence and a better
//!   experience when working with git.
//! - 🦜 `edi` - a TUI Markdown editor that lets you edit Markdown files in your terminal in
//!   style.
//!
//! To install `r3bl-cmdr` on your system, run the following command:
//!
//! ```bash
//! cargo install r3bl-cmdr
//! ```
//!
//! > You will need `cargo` installed on your system. If you don't have it, follow the instructions
//! > [here](https://rustup.rs/).
//!
//! ## Run `giti` binary target
//! <a id="markdown-run-giti-binary-target" name="run-giti-binary-target"></a>
//!
//! <!--
//! giti branch video
//! Source: https://github.com/nazmulidris/developerlife.com/issues/5
//! -->
//! <video width="100%" controls>
//!   <source src="https://github.com/nazmulidris/developerlife.com/assets/2966499/262f59d1-a95c-4af3-accf-c3d6cac6e586" type="video/mp4"/>
//! </video>
//!
//! To run from binary:
//! - Run `cargo install r3bl-cmdr` (detailed instructions above). This will install `giti`
//!   locally to `~/.cargo/bin`.
//! - Run `giti` from anywhere on your system.
//! - Try `giti --help` to see the available commands.
//! - To delete one or more branches in your repo run `giti branch delete`.
//! - To checkout a branch run `giti branch checkout`.
//! - To create a new branch run `giti branch new`.
//!
//! To run from source:
//! - Clone the `r3bl-open-core` repo.
//! - Go to the `cmdr` folder in your terminal.
//! - Run `nu run install` to install `giti` locally to `~/.cargo/bin`.
//! - Run `giti` from anywhere on your system.
//! - Try `giti --help` to see the available commands.
//! - To delete one or more branches in your repo run `giti branch delete`.
//! - To checkout a branch run `giti branch checkout`.
//! - To create a new branch run `giti branch new`.
//! - If you want to generate log output for `giti`, run `giti -l`. For example,
//!   `giti -l branch delete`. To view this log output run `nu run log`.
//!
//! ## Run `edi` binary target
//! <a id="markdown-run-edi-binary-target" name="run-edi-binary-target"></a>
//!
//! <!--
//! edi video
//! Source: https://github.com/nazmulidris/developerlife.com/issues/6
//! -->
//! <video width="100%" controls>
//!   <source src="https://github.com/nazmulidris/developerlife.com/assets/2966499/f2c4b07d-b5a2-4f41-af7a-06d1b6660c41" type="video/mp4"/>
//! </video>
//!
//!
//! To run from binary:
//! - Run `cargo install r3bl-cmdr` (detailed instructions above). This will install `giti`
//!   locally to `~/.cargo/bin`.
//! - Run `edi` from anywhere on your system.
//! - Try `edi --help` to see the available commands.
//! - To open an existing file, run `edi <file_name>`. For example, `edi README.md`.
//!
//! To run from source:
//! - Clone the `r3bl-open-core` repo.
//! - Go to the `cmdr` folder in your terminal.
//! - Run `nu run install` to install `edi` locally to `~/.cargo/bin`.
//! - Run `edi` from anywhere on your system.
//! - Try `edi --help` to see the available commands.
//! - To open an existing file, run `edi <file_name>`. For example, `edi README.md`.
//! - If you want to generate log output for `edi`, run `edi -l`. For example,
//!   `edi -l README.md`. To view this log output run `nu run log`.
//!
//! ## Build, run, test tasks
//! <a id="markdown-build%2C-run%2C-test-tasks" name="build%2C-run%2C-test-tasks"></a>
//!
//! ### Prerequisites
//! <a id="markdown-prerequisites" name="prerequisites"></a>
//!
//!
//! 🌠 In order for these to work you have to install the Rust toolchain, `nu`, `cargo-watch`, `bat`,
//! and `flamegraph` on your system. Here are the instructions:
//!
//! 1. Install the Rust toolchain using `rustup` by following the instructions
//!    [here](https://rustup.rs/).
//! 1. Install `cargo-watch` using `cargo install cargo-watch`.
//! 1. Install `flamegraph` using `cargo install flamegraph`.
//! 1. Install `bat` using `cargo install bat`.
//! 1. Install [`nu`](https://crates.io/crates/nu) shell on your system using `cargo install nu`. It is
//!    available for Linux, macOS, and Windows.
//!
//! ### Nu shell scripts to build, run, test etc.
//! <a id="markdown-nu-shell-scripts-to-build%2C-run%2C-test-etc." name="nu-shell-scripts-to-build%2C-run%2C-test-etc."></a>
//!
//!
//! | Command             | Description                                                                                                          |
//! | ------------------- | -------------------------------------------------------------------------------------------------------------------- |
//! | `nu run help`       | See all the commands you can pass to the `run` script                                                                |
//! | `nu run install`    | Install `giti`, `edi`, `rc` to `~/.cargo/bin`                                                                        |
//! | `nu run build`      | Build                                                                                                                |
//! | `nu run clean`      | Clean                                                                                                                |
//! | `nu run test`       | Run tests                                                                                                            |
//! | `nu run clippy`     | Run clippy                                                                                                           |
//! | `nu run log`        | View the log output. This [video](https://www.youtube.com/watch?v=Sy26IMkOEiM) has a walkthrough of how to use this. |
//! | `nu run docs`       | Build docs                                                                                                           |
//! | `nu run serve-docs` | Serve docs over VSCode Remote SSH session                                                                            |
//! | `nu run rustfmt`    | Run rustfmt                                                                                                          |
//!
//! The following commands will watch for changes in the source folder and re-run:
//!
//! | Command                                             | Description                        |
//! | --------------------------------------------------- | ---------------------------------- |
//! | `nu run watch-all-tests`                            | Watch all test                     |
//! | `nu run watch-one-test <test_name>`                 | Watch one test                     |
//! | `nu run watch-clippy`                               | Watch clippy                       |
//! | `nu run watch-macro-expansion-one-test <test_name>` | Watch macro expansion for one test |
//!
//! There's also a `run` script at the **top level folder** of the repo. It is intended to be used in a
//! CI/CD environment w/ all the required arguments supplied or in interactive mode, where the user will
//! be prompted for input.
//!
//! | Command                      | Description                                                                                                                                                                                                                                            |
//! | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
//! | `nu run all`                 | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD                                                                                                                                                                                  |
//! | `nu run build-full`          | This will build all the crates in the Rust workspace. And it will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
//! | `nu run install-cargo-tools` | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`, `flamegraph` will all be installed in one go)                                                                                         |
//! | `nu run check-licenses`      | Use `cargo-deny` to audit all licenses used in the Rust workspace                                                                                                                                                                                      |

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

pub const DEVELOPMENT_MODE: bool = true;

// Attach sources.
pub mod analytics_client;
pub mod color_constants;
pub mod edi;
pub mod giti;
pub mod rc;

// Re-export.
pub use analytics_client::*;
