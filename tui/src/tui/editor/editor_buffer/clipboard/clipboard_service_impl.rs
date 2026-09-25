// Copyright (c) 2024-2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use super::{ClipboardResult, ClipboardService};
use crate::{ClipboardTarget, DEBUG_TUI_COPY_PASTE, OscSequence, ok};
use copypasta::{ClipboardContext, ClipboardProvider};
use std::io::Write;

/// In-band terminal clipboard service using [`OSC`] 52 escape sequences.
///
/// Writes clipboard copy commands directly to standard output via
/// [`OscSequence::ClipboardSet`]. When running in headless environments (e.g. remote SSH
/// sessions or Docker containers) where no display server (Wayland / macOS / Windows) is
/// available, this allows the host terminal emulator to capture the copied text.
///
/// Inbound clipboard data ([`try_to_get_content_from_clipboard`]) is intentionally not
/// supported via synchronous queries, because terminal pasting is driven asynchronously
/// by [`DEC`] Private Mode 2004 Bracketed Paste (`CSI 200 ~`).
///
/// [`DEC`]: https://en.wikipedia.org/wiki/Digital_Equipment_Corporation
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`try_to_get_content_from_clipboard`]:
///     ClipboardService::try_to_get_content_from_clipboard
#[derive(Debug, Default, Clone, Copy)]
pub struct Osc52Clipboard;

impl ClipboardService for Osc52Clipboard {
    fn try_to_put_content_into_clipboard(
        &mut self,
        content: String,
    ) -> ClipboardResult<()> {
        let seq = OscSequence::ClipboardSet {
            target: ClipboardTarget::System,
            data: content,
        };

        let mut stdout = std::io::stdout().lock();
        write!(stdout, "{seq}")?;
        stdout.flush()?;

        DEBUG_TUI_COPY_PASTE.then(|| {
            if let OscSequence::ClipboardSet { data, .. } = &seq {
                // % is Display, ? is Debug.
                tracing::debug!(
                    message = "📋📋📋 Selected Text was copied to clipboard via OSC 52",
                    copied = %data,
                );
            }
        });

        ok!()
    }

    fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
        Err(
            "OSC 52 clipboard reading via synchronous query is unsupported. \
             Inbound text arrives asynchronously via DEC Private Mode 2004 Bracketed Paste."
                .into(),
        )
    }
}

/// Standard system clipboard service with hybrid fallback.
///
/// Attempts to copy via the local desktop display server (`copypasta`). If that fails
/// (e.g. running in a remote SSH session or headless environment without `$DISPLAY` /
/// `$WAYLAND_DISPLAY`), it automatically falls back to [`Osc52Clipboard`].
#[derive(Debug, Default)]
pub struct SystemClipboard;

impl ClipboardService for SystemClipboard {
    /// Attempts to write clipboard content via the local desktop display server
    /// ([`copypasta`]). If that fails, it falls back to [`Osc52Clipboard`].
    ///
    /// # Performance impact of cloning vs fallback reliability
    ///
    /// Notice that `content.clone()` is passed to [`copypasta`]. We acknowledge that
    /// cloning can be expensive if `content` is very large. However, this clone is
    /// intentionally retained for full transaction lifecycle resilience:
    ///
    /// 1. **Consumed by value**: The third-party [`ClipboardProvider::set_contents`] API
    ///    consumes `content` as an owned [`String`] and returns `Result<(), Box<dyn
    ///    Error>>` without returning the string on error.
    /// 2. **Transaction fallback**: If [`set_contents`] fails (e.g. Wayland compositor
    ///    data device errors or OS clipboard mutex contention), `content` must remain
    ///    available to fall back to [`Osc52Clipboard`].
    /// 3. **Headless efficiency**: In remote SSH or Docker environments without a display
    ///    server, [`ClipboardContext::new`] returns an error immediately. Because
    ///    `and_then` is short-circuiting, `content.clone()` is never evaluated in those
    ///    headless environments.
    ///
    /// # Errors
    ///
    /// Returns an error if both the desktop display server and [`Osc52Clipboard`] fail to
    /// write the clipboard content.
    ///
    /// [`ClipboardContext::new`]: copypasta::ClipboardContext::new
    /// [`ClipboardProvider::set_contents`]: copypasta::ClipboardProvider::set_contents
    /// [`copypasta`]: copypasta
    /// [`set_contents`]: copypasta::ClipboardProvider::set_contents
    fn try_to_put_content_into_clipboard(
        &mut self,
        content: String,
    ) -> ClipboardResult<()> {
        match ClipboardContext::new()
            .and_then(|mut ctx| ctx.set_contents(content.clone()))
        {
            Ok(()) => {
                DEBUG_TUI_COPY_PASTE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "📋📋📋 Selected Text was copied to clipboard via copypasta",
                        copied = %content,
                    );
                });
                ok!()
            }
            Err(copypasta_err) => {
                DEBUG_TUI_COPY_PASTE.then(|| {
                    // % is Display, ? is Debug.
                    tracing::debug!(
                        message = "📋📋📋 copypasta failed, falling back to OSC 52",
                        error = %copypasta_err,
                    );
                });
                // Fallback to in-band terminal emulator OSC 52 clipboard.
                Osc52Clipboard.try_to_put_content_into_clipboard(content)
            }
        }
    }

    fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
        let mut ctx = ClipboardContext::new()?;
        let content = ctx.get_contents()?;
        Ok(content)
    }
}

#[cfg(any(test, doc))]
pub mod clipboard_test_fixtures {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    #[derive(Debug, Default)]
    pub struct TestClipboard {
        pub content: String,
    }

    impl ClipboardService for TestClipboard {
        fn try_to_put_content_into_clipboard(
            &mut self,
            content: String,
        ) -> ClipboardResult<()> {
            self.content = content;
            ok!()
        }

        fn try_to_get_content_from_clipboard(&mut self) -> ClipboardResult<String> {
            ok!(self.content.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc52_clipboard_get_unsupported() {
        let mut clipboard = Osc52Clipboard;
        let result = clipboard.try_to_get_content_from_clipboard();
        assert!(result.is_err());
    }

    #[test]
    fn test_osc52_clipboard_put() {
        let mut clipboard = Osc52Clipboard;
        let result =
            clipboard.try_to_put_content_into_clipboard("Hello OSC 52".to_string());
        assert!(result.is_ok());
    }
}
