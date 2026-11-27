// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod jsonc_manifest_struct;
mod crate_distribution;
mod crate_metadata;
mod crate_name;
mod crate_release_date;
mod crate_version;
mod endpoint_url;
mod jsonc_scalar;

// Public barrel re-exports.
pub use jsonc_manifest_struct::*;
pub use crate_distribution::*;
pub use crate_metadata::*;
pub use crate_name::*;
pub use crate_release_date::*;
pub use crate_version::*;
pub use endpoint_url::*;
pub use jsonc_scalar::*;
