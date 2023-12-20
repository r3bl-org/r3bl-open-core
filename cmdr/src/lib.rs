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
//! 1. Go to the `cmdr` folder in your terminal
//! 2. Run `nu run install` to install `giti` locally to `~/.cargo/bin`
//! 3. Run `giti` from anywhere on your system
//! 4. To delete one or more branches in your repo run `giti branch delete`.
//! 5. If you want to generate log output for `giti`, run `giti -l`. For example, `giti -l
//!    branch delete`.
//!
//! [![asciicast](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg.svg)](https://asciinema.org/a/14V8v3OKKYvDkUDkRFiMDsCNg)

// https://github.com/rust-lang/rust-clippy
// https://rust-lang.github.io/rust-clippy/master/index.html
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]

pub const DEVELOPMENT_MODE: bool = true;

pub mod giti;
pub mod rc;

pub use giti::*;
pub use rc::*;
