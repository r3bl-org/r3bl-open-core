// Copyright (c) 2023-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! Clipboard integration and abstraction services.
//!
//! See [`ClipboardService`] for details on the hybrid clipboard architecture.
//!
//! [`ClipboardService`]: crate::ClipboardService

// Attach.
mod clipboard_service;
mod clipboard_service_impl;

// Re-export.
pub use clipboard_service::*;
pub use clipboard_service_impl::*;
