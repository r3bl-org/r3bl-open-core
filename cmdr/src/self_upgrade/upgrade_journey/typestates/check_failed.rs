// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::StateMarker;
use crate::{FetchManifestError, ThreadRunnerError};

/// Control Plane failure reason recorded in [`CheckFailed::error`] when a manifest check
/// fails.
///
/// Composes [Execution Plane] manifest fetch/lookup failures ([`FetchManifestError`])
/// with [Control Plane] thread/runtime failures ([`ThreadRunnerError`]).
///
/// [`FetchManifestError`]: crate::self_upgrade::FetchManifestError
/// [`ThreadRunnerError`]: crate::self_upgrade::ThreadRunnerError
/// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
/// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
pub enum CheckFailedError {
    /// Manifest fetch, HTTP status, JSONC parse, or crate lookup failed in the
    /// [Execution Plane].
    ///
    /// [Execution Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    #[error(transparent)]
    #[diagnostic(transparent)]
    FetchManifest(#[from] FetchManifestError),

    /// Dedicated or scoped OS thread failed to spawn, initialize its Tokio runtime, or
    /// panicked in the [Control Plane].
    ///
    /// [Control Plane]: mod@crate::self_upgrade#control-plane-vs-execution-plane
    #[error(transparent)]
    #[diagnostic(transparent)]
    ThreadRunner(#[from] ThreadRunnerError),
}

/// Typestate when the manifest check encountered a network, parse, or thread/runtime
/// failure.
#[derive(Debug)]
pub struct CheckFailed {
    /// Structured [`CheckFailedError`] describing why the check failed.
    ///
    /// [`CheckFailedError`]: crate::self_upgrade::CheckFailedError
    pub error: CheckFailedError,
}
impl StateMarker for CheckFailed {}
