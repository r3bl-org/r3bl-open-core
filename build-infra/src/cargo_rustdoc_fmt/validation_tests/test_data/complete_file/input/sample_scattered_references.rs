// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Test fixture: scattered reference-style links that should be aggregated at bottom

//! Module documentation with scattered references.
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`parser`] - Main routing entry point
//! [`parser`]: mod@super::parser
//! - ➡️ **Peer**: [`keyboard`], [`terminal_events`] - Other parsers
//! [`keyboard`]: mod@super::keyboard
//! [`terminal_events`]: mod@super::terminal_events
//!
//! ## Protocol Details
//!
//! See [`VT100MouseButton`] for button types.
//! [`VT100MouseButton`]: super::VT100MouseButton
//! And [`VT100MouseAction`] for actions.
//! [`VT100MouseAction`]: super::VT100MouseAction

/// Function with scattered reference-style links.
///
/// Uses [`DataType`] for storage.
/// [`DataType`]: crate::DataType
///
/// Also needs [`Config`] for setup.
/// [`Config`]: crate::Config
///
/// See [`Result`] for return type.
/// [`Result`]: std::result::Result
fn example() {}

/// Another function with references.
///
/// Links to [`alpha`] first.
/// [`alpha`]: crate::Alpha
///
/// Then references [`zebra`].
/// [`zebra`]: crate::Zebra
///
/// And finally [`middle`].
/// [`middle`]: crate::Middle
fn another_example() {}
