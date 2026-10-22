// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod build;
mod download;
mod prompt;
mod release;
mod strategy;
pub mod strings;
mod swap;
mod target;

// Public barrel re-exports.
pub use build::*;
pub use download::*;
pub use prompt::*;
pub use release::*;
pub use strategy::*;
pub use strings::*;
pub use strings as ui_strings;
pub use swap::*;
pub use target::*;
