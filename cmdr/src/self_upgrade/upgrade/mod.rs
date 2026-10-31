// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod upgrade_entry_point;
mod bin_name;
mod build;
mod download;
mod feedback_device;
mod prompt;
mod release;
mod swap;
mod target;
mod ui_strings;
mod upgrade_source;

// Public barrel re-exports.
pub use upgrade_entry_point::*;
pub use bin_name::*;
pub use build::*;
pub use download::*;
pub use feedback_device::*;
pub use prompt::*;
pub use release::*;
pub use swap::*;
pub use target::*;
pub use ui_strings::*;
pub use upgrade_source::*;
