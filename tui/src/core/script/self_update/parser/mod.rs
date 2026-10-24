// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod crate_metadata;
mod crate_version;
mod jsonc_manifest;

// Public barrel re-exports.
pub use crate_metadata::*;
pub use crate_version::*;
pub use jsonc_manifest::*;
