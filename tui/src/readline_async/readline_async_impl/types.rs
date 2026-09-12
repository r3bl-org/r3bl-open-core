// Copyright (c) 2024-2026 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{KeyPress, VPSize};
use miette::Report as ErrorReport;
use std::{io, num::NonZeroU8, time::Duration};
use thiserror::Error;

/// Events emitted by [`Readline::readline()`].
///
/// [`Readline::readline()`]: crate::Readline::readline
#[derive(Debug, PartialEq, Clone)]
pub enum ReadlineEvent {
    /// The user entered a line of text.
    Line(String),

    /// The user pressed `Ctrl+D`.
    Eof,

    /// The user pressed `Ctrl+C`.
    Interrupted,

    /// The user pressed `Tab`.
    Tab,

    /// The user pressed `Shift+Tab` (`BackTab`).
    BackTab,

    /// The user pressed `Page Up`.
    PageUp,

    /// The user pressed `Page Down`.
    PageDown,

    /// The user pressed `Insert`.
    Insert,

    /// The user pressed a function key (`F1`-`F12`).
    ///
    /// The value is 1-12 (not 0-11), matching the key labels.
    FnKey(NonZeroU8),

    /// A key that readline doesn't handle internally.
    ///
    /// This allows consumers to handle application-specific keys without
    /// requiring changes to the readline library.
    UnhandledKey(KeyPress),

    /// The terminal was resized.
    Resized(VPSize),
}

/// This is an artificial delay amount that is added to hide the jank of displaying the
/// cursor to the terminal when the prompt is first printed, after the terminal is put
/// into raw mode.
pub const READLINE_ASYNC_INITIAL_PROMPT_DISPLAY_CURSOR_SHOW_DELAY: Duration =
    Duration::from_millis(66);

/// Internal control flow for the [`readline()`] method. This is used primarily to make
/// testing easier.
///
/// # Result Conversion
///
/// This type supports implicit conversion from [`Result<Option<T>, E>`] via [`.into()`],
/// allowing for a fluid functional style when working with locks and loops.
///
/// # Usage Guidance
///
/// To maintain high readability and low cognitive load, follow these conventions:
/// 1. **Errors**: Prefer `ReturnError(E)` directly for early exits with errors.
/// 2. **Success**: Prefer `ReturnOk(T)` directly for successful completion.
/// 3. **Early Returns**: Use [`Self::Continue`] directly for early returns in a state
///    machine loop.
///
/// [`.into()`]: Into::into
/// [`readline()`]: crate::Readline::readline
#[derive(Debug, PartialEq, Clone)]
pub enum ReadlineControlFlow<T, E> {
    ReturnOk(T),
    ReturnError(E),
    Continue,
}

impl<T, E> From<Result<Option<T>, E>> for ReadlineControlFlow<T, E> {
    fn from(result: Result<Option<T>, E>) -> ReadlineControlFlow<T, E> {
        match result {
            Ok(Some(val)) => Self::ReturnOk(val),
            Ok(None) => Self::Continue,
            Err(err) => Self::ReturnError(err),
        }
    }
}

/// Error returned from [`readline()`]. Such errors generally require specific procedures
/// to recover from.
///
/// # High-Fidelity Diagnostics
///
/// This type implements [`miette::Diagnostic`], which allows for high-fidelity error
/// reporting with help text and error codes. Use [`.into_diagnostic()`] to convert this
/// into a [`miette::Report`].
///
/// # Implicit Conversions
///
/// - **From Report**: Supports implicit conversion from [`miette::Report`] (via
///   [`From<ErrorReport>`]).
///
/// [`.into_diagnostic()`]: miette::IntoDiagnostic::into_diagnostic
/// [`readline()`]: crate::Readline::readline
#[derive(Debug, Error, miette::Diagnostic)]
pub enum ReadlineError {
    /// An internal I/O error occurred.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// `readline()` was called after the [`SharedWriter`] was dropped and everything
    /// written to the `SharedWriter` was already output.
    ///
    /// [`SharedWriter`]: crate::SharedWriter
    #[error("line writers closed")]
    Closed,
}

/// For convenience, convert [`ErrorReport`] to [`ReadlineError`], so that
/// [`into_diagnostic()`] works.
///
/// [`into_diagnostic()`]: miette::IntoDiagnostic::into_diagnostic
impl From<ErrorReport> for ReadlineError {
    fn from(report: ErrorReport) -> ReadlineError {
        ReadlineError::IO(io::Error::other(format!("{report}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use miette::miette;

    #[test]
    fn test_readline_control_flow_from_result() {
        // Branch 1: Ok(Some(val)) -> ReturnOk.
        let ok_some: Result<Option<i32>, &str> = Ok(Some(42));
        assert_eq!(
            ReadlineControlFlow::from(ok_some),
            ReadlineControlFlow::ReturnOk(42)
        );

        // Branch 2: Ok(None) -> Continue.
        let ok_none: Result<Option<i32>, &str> = Ok(None);
        assert_eq!(
            ReadlineControlFlow::from(ok_none),
            ReadlineControlFlow::Continue
        );

        // Branch 3: Err(err) -> ReturnError.
        let err: Result<Option<i32>, &str> = Err("test error");
        assert_eq!(
            ReadlineControlFlow::from(err),
            ReadlineControlFlow::ReturnError("test error")
        );
    }

    #[test]
    fn test_readline_error_from_error_report() {
        let report = miette!("custom diagnostic error");
        let error = ReadlineError::from(report);
        match error {
            ReadlineError::IO(io_err) => {
                assert!(io_err.to_string().contains("custom diagnostic error"));
            }
            ReadlineError::Closed => panic!("expected IO error variant"),
        }
    }
}
