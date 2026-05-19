// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{ControllerReader, ControllerWriter, PtyPair, PtyTestChild};
use std::io::BufReader;

/// A bundle of [`PTY`] resources passed to integration test controllers.
///
/// This context is automatically prepared by the [`generate_pty_test!`] macro.
///
/// [`generate_pty_test!`]: crate::generate_pty_test
/// [`PTY`]: mod@crate::core::pty
#[allow(missing_debug_implementations)]
pub struct PtyTestContext {
    /// The [`PTY`] pair wrapper.
    ///
    /// [`PTY`]: mod@crate::core::pty
    pub pty_pair: PtyPair,

    /// The controlled child process wrapped in a safety guard.
    pub child: PtyTestChild,

    /// A buffered reader for the [`PTY`] controller side.
    ///
    /// [`PTY`]: mod@crate::core::pty
    pub buf_reader: BufReader<ControllerReader>,

    /// A writer for sending input to the [`PTY`] controller side.
    ///
    /// On Windows, this writer has already performed the mandatory [`ConPTY`] [`DSR`]
    /// handshake.
    ///
    /// [`ConPTY`]:
    ///     https://learn.microsoft.com/en-us/windows/console/creating-a-pseudoconsole-session
    /// [`DSR`]: crate::DsrSequence
    /// [`PTY`]: mod@crate::core::pty
    pub writer: ControllerWriter,
}

/// Result of reading [`PTY`] output lines until a marker.
///
/// [`PTY`]: crate::core::pty::pty_engine::pty_pair#what-is-a-pty
#[derive(Debug)]
pub struct ReadLinesResult {
    /// Collected output lines (normalized, filtered).
    pub lines: Vec<String>,
    /// Whether the marker string was found in the output.
    pub found_marker: bool,
}
