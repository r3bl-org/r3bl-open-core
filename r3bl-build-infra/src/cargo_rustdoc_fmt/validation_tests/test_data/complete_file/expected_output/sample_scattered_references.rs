// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// Test fixture: scattered reference-style links that should be aggregated at bottom

//! Module documentation with scattered references.
//!
//! **Navigate**:
//! - ⬆️ **Up**: [`parser`] - Main routing entry point
//! - ➡️ **Peer**: [`keyboard`], [`terminal_events`] - Other parsers
//!
//! ## Protocol Details
//!
//! See [`VT100MouseButton`] for button types.
//! And [`VT100MouseAction`] for actions.
//!
//! [`VT100MouseAction`]: super::VT100MouseAction
//! [`VT100MouseButton`]: super::VT100MouseButton
//! [`keyboard`]: mod@super::keyboard
//! [`parser`]: mod@super::parser
//! [`terminal_events`]: mod@super::terminal_events

/// Function with scattered reference-style links.
///
/// Uses [`DataType`] for storage.
///
/// Also needs [`Config`] for setup.
///
/// See [`Result`] for return type.
///
/// [`Config`]: crate::Config
/// [`DataType`]: crate::DataType
/// [`Result`]: std::result::Result
fn example() {}

/// Another function with references.
///
/// Links to [`alpha`] first.
///
/// Then references [`zebra`].
///
/// And finally [`middle`].
///
/// [`alpha`]: crate::Alpha
/// [`middle`]: crate::Middle
/// [`zebra`]: crate::Zebra
fn another_example() {}
