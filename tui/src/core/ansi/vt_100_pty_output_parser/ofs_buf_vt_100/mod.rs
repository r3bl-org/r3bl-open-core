// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

#![rustfmt::skip]

// Attach (Private).

mod core;
mod accessors;
mod active_buffer_routing;
mod config;

// Re-export (Public).

pub use core::*;
pub use active_buffer_routing::*;
pub use config::*;