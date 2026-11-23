// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Private submodules.
mod fetch_manifest;
mod remote_version_check_gate;

// Public barrel re-exports.
pub use fetch_manifest::*;
pub use remote_version_check_gate::*;
