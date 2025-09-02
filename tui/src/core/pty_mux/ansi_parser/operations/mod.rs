// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! ANSI/VT sequence operation modules.
//!
//! This module organizes all the different types of ANSI operations into
//! logical groups for better maintainability and discoverability.

pub mod char_ops;
pub mod cursor_ops;
pub mod dsr_ops;
pub mod line_ops;
pub mod margin_ops;
pub mod mode_ops;
pub mod scroll_ops;
pub mod sgr_ops;
pub mod terminal_ops;

// Re-export all operations for easier access.
pub use char_ops::*;
pub use cursor_ops::*;
pub use dsr_ops::*;
pub use line_ops::*;
pub use margin_ops::*;
pub use mode_ops::*;
pub use scroll_ops::*;
pub use sgr_ops::*;
pub use terminal_ops::*;
