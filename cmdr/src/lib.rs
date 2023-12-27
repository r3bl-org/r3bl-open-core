/*
 *   Copyright (c) 2022 R3BL LLC
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

//! ## Run `giti` binary target
//! <a id="markdown-run-giti-binary-target" name="run-giti-binary-target"></a>
//!
//! 1. Go to the `cmdr` folder in your terminal.
//! 2. Run `nu run install` to install `giti` locally to `~/.cargo/bin`.
//! 3. Run `giti` from anywhere on your system.
//! 4. To delete one or more branches in your repo run `giti branch delete`.
//! 5. If you want to generate log output for `giti`, run `giti -l`. For example, `giti -l
//!    branch delete`. To view this log output run `nu run log`.
//!
//! [![asciicast](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg.svg)](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg)
//!
//! ## Build, run, test tasks
//! <a id="markdown-build%2C-run%2C-test-tasks" name="build%2C-run%2C-test-tasks"></a>
//!
//! ### Prerequisites
//! <a id="markdown-prerequisites" name="prerequisites"></a>
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
//! | Command                                | Description                                       |
//! | -------------------------------------- | ------------------------------------------------- |
//! | `nu run install`                       | Install `giti`, `edi`, `rc`  to `~/.cargo/bin`    |
//! | `nu run build`                         | Build                                             |
//! | `nu run clean`                         | Clean                                             |
//! | `nu run all`                           | All                                               |
//! | `nu run test`                          | Run tests                                         |
//! | `nu run clippy`                        | Run clippy                                        |
//! | `nu run docs`                          | Build docs                                        |
//! | `nu run serve-docs`                    | Serve docs over VSCode Remote SSH session         |
//! | `nu run rustfmt`                       | Run rustfmt                                       |
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
//! There's also a `run` script at the **top level folder** of the repo. It is intended to
//! be used in a CI/CD environment w/ all the required arguments supplied or in
//! interactive mode, where the user will be prompted for input.
//!
//! | Command                       | Description                        |
//! | ----------------------------- | ---------------------------------- |
//! | `nu run all`                  | Run all the tests, linting, formatting, etc. in one go. Used in CI/CD |
//! | `nu run build-full`           | This will build all the crates in the Rust workspace. And it will install all the required pre-requisite tools needed to work with this crate (what `install-cargo-tools` does) and clear the cargo cache, cleaning, and then do a really clean build. |
//! | `nu run install-cargo-tools`  | This will install all the required pre-requisite tools needed to work with this crate (things like `cargo-deny`, `flamegraph` will all be installed in one go) |
//! | `nu run check-licenses`       | Use `cargo-deny` to audit all licenses used in the Rust workspace |

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(clippy::unwrap_in_result)]
#![warn(rust_2018_idioms)]

pub const DEVELOPMENT_MODE: bool = true;

// Attach sources.
pub mod edi;
pub mod giti;
pub mod rc;
