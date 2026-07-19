// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Model Context Protocol (MCP) server subsystem for AI coding assistants.
//!
//! This module provides the `stdio` JSON-RPC 2.0 event loop, tool registry, tool
//! parameter parsing, execution dispatch, and diagnostics reporting.

mod protocol;
mod server;
mod tools;

pub use protocol::*;
pub use server::*;
pub use tools::*;
