// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::{DefaultPtySize, VPSize};
use std::ops::{Add, AddAssign};

/// Whether to capture a particular data stream from the [`PTY`].
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureFlag {
    Capture,
    NoCapture,
}

/// Configuration for a [`PTY`] session.
///
/// This struct holds the final resolved state of all configuration options. These
/// settings govern event routing and terminal capabilities in the [Session Layer].
/// While this struct is `pub`, it is **not** intended to be constructed manually.
/// Instead, either:
/// 1. Compose the desired [`PtySessionConfigToken`]s using the `+` operator from scratch.
/// 2. Start with [`DefaultPtySessionConfig`] and use `+` operator to override any default
///    options.
///
/// # Examples
///
/// ```rust
/// # use r3bl_tui::{
/// #     DefaultPtySessionConfig, PtySessionConfig, PtySessionConfigToken,
/// #     PtySessionConfigToken::CaptureOsc, PtySessionConfigToken::CaptureOutput
/// # };
/// let config_1: PtySessionConfig = DefaultPtySessionConfig + CaptureOsc;
/// let config_2: PtySessionConfig = CaptureOsc + CaptureOutput;
/// let config_3: PtySessionConfig = DefaultPtySessionConfig + CaptureOsc + CaptureOutput;
/// let config_4: PtySessionConfig = DefaultPtySessionConfig.into();
/// ```
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [Session Layer]: mod@crate::pty_session
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySessionConfig {
    /// Whether to capture **[`OSC`]** sequences.
    ///
    /// See [`PtySessionConfigToken::CaptureOsc`] for details.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    pub capture_osc: CaptureFlag,

    /// Whether to capture raw terminal output.
    ///
    /// See [`PtySessionConfigToken::CaptureOutput`] for details.
    pub capture_output: CaptureFlag,

    /// The initial window size for the [`PTY`].
    ///
    /// See [`PtySessionConfigToken::Size`] for details.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    pub pty_size: VPSize,
}

/// Marker struct that provides the default [`PtySessionConfig`].
///
/// This zero-sized type acts as the starting point for **composing** configuration
/// options. It implements [`Into<PtySessionConfig>`] and supports the `+` operator for
/// applying [`PtySessionConfigToken`]s.
///
/// # Default Values
///
/// See the [`From<DefaultPtySessionConfig>`] implementation for the default field values.
///
/// [`DefaultPtySessionConfig::default()`]: DefaultPtySessionConfig::default()
#[derive(Debug, Clone, Copy)]
pub struct DefaultPtySessionConfig;

mod impl_default_pty_session_config {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl DefaultPtySessionConfig {
        /// Generate the default [`PtySessionConfig`] with sensible config options.
        #[must_use]
        #[allow(clippy::should_implement_trait)]
        pub fn default() -> PtySessionConfig {
            PtySessionConfig {
                capture_osc: CaptureFlag::NoCapture,
                capture_output: CaptureFlag::Capture,
                pty_size: DefaultPtySize.into(),
            }
        }
    }

    /// Convert [`DefaultPtySessionConfig`] marker to [`PtySessionConfig`].
    impl From<DefaultPtySessionConfig> for PtySessionConfig {
        fn from(_: DefaultPtySessionConfig) -> PtySessionConfig {
            DefaultPtySessionConfig::default()
        }
    }
}

/// Configuration tokens for a [`PTY`] session.
///
/// Tokens exist so they can be composed to build a [`PtySessionConfig`], using the `+`
/// operator. The operator follows a "last write wins" for each field.
///
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PtySessionConfigToken {
    /// Enable capture of **[`OSC`]** (Operating System Command) escape sequences.
    ///
    /// This is required to receive structured events like **`OSC 9;4`** for progress
    /// reporting or **`OSC 0`** for terminal title updates. When enabled, the [`PTY`]
    /// parser intercepts these sequences and emits them as [`PtyOutputEvent::Osc`].
    ///
    /// See [`enable_osc_sequences()`] for more details.
    ///
    /// [`enable_osc_sequences()`]: crate::PtySessionBuilder::enable_osc_sequences
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`PtyOutputEvent::Osc`]: crate::PtyOutputEvent::Osc
    CaptureOsc,

    /// Disable **[`OSC`]** sequence capture.
    ///
    /// When disabled, **[`OSC`]** sequences are treated as raw output bytes and
    /// delivered via [`PtyOutputEvent::Output`], rather than being parsed into
    /// structured events.
    ///
    /// [`OSC`]: crate::osc_codes::OscSequence
    /// [`PtyOutputEvent::Output`]: crate::PtyOutputEvent::Output
    NoCaptureOsc,

    /// Enable capture of raw terminal output.
    ///
    /// [`stdout`] and [`stderr`] outputs from the child process are delivered as
    /// [`PtyOutputEvent::Output`] events. This is the default behavior and is
    /// required for displaying the process's output in a terminal.
    ///
    /// [`PtyOutputEvent::Output`]: crate::PtyOutputEvent::Output
    /// [`stderr`]: std::io::stderr
    /// [`stdout`]: std::io::stdout
    CaptureOutput,

    /// Disable raw output capture.
    ///
    /// Useful for background tasks where you only care about structured events
    /// (like [`CaptureOsc`]) or process lifecycle events (like [`Exit`]), and
    /// want to avoid the overhead of processing large volumes of raw text.
    ///
    /// [`CaptureOsc`]: Self::CaptureOsc
    /// [`Exit`]: crate::PtyOutputEvent::Exit
    NoCaptureOutput,

    /// Specify the initial window size ([`rows`] and [`columns`]) for the [`PTY`].
    ///
    /// Correct sizing is essential for **`TUI`** applications like `htop` or
    /// `vim` to render their interface properly within the available terminal
    /// area.
    ///
    /// [`columns`]: crate::VPWidth
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`rows`]: crate::VPHeight
    Size(VPSize),
}

/// Backwards compatibility alias for [`PtySessionConfigToken`].
pub type PtySessionConfigOption = PtySessionConfigToken;

// XMARK: Elegant Constructor DSL.

/// This module implements the "heavy lifting" for the Elegant Constructor DSL Pattern.
///
/// This enables an elegant, type-safe, and ergonomic DSL for configuring a [`PTY`]
/// session. By leveraging [`impl Into<PtySessionConfig>`] and operator overloading (`+`),
/// callers can progressively disclose their configuration needs.
///
/// It implements [`From`] traits to convert various configuration types
/// ([`DefaultPtySessionConfig`], [`PtySessionConfig`], and [`PtySessionConfigToken`])
/// into [`PtySessionConfig`], and [`Add`] traits to combine them with `+`. This is what
/// allows [`PtySessionBuilder::with_config`] to accept multiple types of inputs via
/// [`impl Into<PtySessionConfig>`].
///
/// # Architecture: Constructor DSL Tokens vs Storage Types
///
/// 1. **Constructor DSL Tokens / Inputs** ([`PtySessionConfigToken`],
///    [`DefaultPtySessionConfig`]):
///    - Token types passed to configure session options or compose with `+`.
///
/// 2. **Canonical Storage Struct** ([`PtySessionConfig`]):
///    - Aggregates resolved [`PTY`] session flags, size, and settings.
///
/// See [`PtySessionBuilder`] docs for a full usage example and the [Session Layer]
/// documentation for the architectural context.
///
/// [`impl Into<PtySessionConfig>`]: PtySessionConfig
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`PtySessionBuilder::with_config`]: crate::PtySessionBuilder::with_config
/// [`PtySessionBuilder`]: crate::PtySessionBuilder
/// [Session Layer]: mod@crate::pty_session
mod impl_elegant_constructor_dsl_pattern {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    impl PtySessionConfig {
        fn apply(&mut self, token: PtySessionConfigToken) {
            match token {
                PtySessionConfigToken::CaptureOsc => {
                    self.capture_osc = CaptureFlag::Capture;
                }
                PtySessionConfigToken::NoCaptureOsc => {
                    self.capture_osc = CaptureFlag::NoCapture;
                }
                PtySessionConfigToken::CaptureOutput => {
                    self.capture_output = CaptureFlag::Capture;
                }
                PtySessionConfigToken::NoCaptureOutput => {
                    self.capture_output = CaptureFlag::NoCapture;
                }

                PtySessionConfigToken::Size(size) => self.pty_size = size,
            }
        }
    }

    /// Start from [`DefaultPtySessionConfig`] and apply one token with `+`.
    impl Add<PtySessionConfigToken> for DefaultPtySessionConfig {
        type Output = PtySessionConfig;

        fn add(self, rhs: PtySessionConfigToken) -> PtySessionConfig {
            let mut config = PtySessionConfig::from(self);
            config.apply(rhs);
            config
        }
    }

    /// Combine two [`PtySessionConfigToken`]s into a [`PtySessionConfig`] using `+`.
    impl Add<PtySessionConfigToken> for PtySessionConfigToken {
        type Output = PtySessionConfig;

        fn add(self, rhs: PtySessionConfigToken) -> PtySessionConfig {
            let mut config = PtySessionConfig::from(DefaultPtySessionConfig);
            config.apply(self);
            config.apply(rhs);
            config
        }
    }

    /// Add an option to an existing [`PtySessionConfig`].
    impl Add<PtySessionConfigToken> for PtySessionConfig {
        type Output = PtySessionConfig;

        fn add(mut self, rhs: PtySessionConfigToken) -> PtySessionConfig {
            self.apply(rhs);
            self
        }
    }

    /// Implement [`AddAssign`] for `+=` operator on [`PtySessionConfig`].
    impl AddAssign<PtySessionConfigToken> for PtySessionConfig {
        fn add_assign(&mut self, rhs: PtySessionConfigToken) { self.apply(rhs); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{vp_height, vp_width};

    #[test]
    fn test_default_config() {
        let config = PtySessionConfig::from(DefaultPtySessionConfig);
        assert_eq!(config.capture_osc, CaptureFlag::NoCapture);
        assert_eq!(config.capture_output, CaptureFlag::Capture);
    }

    #[test]
    fn test_option_combination() {
        // Token + Token.
        let config =
            PtySessionConfigToken::CaptureOsc + PtySessionConfigToken::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);

        // DefaultPtySessionConfig + one token.
        let config = DefaultPtySessionConfig + PtySessionConfigToken::CaptureOsc;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::Capture); // Default

        // DefaultPtySessionConfig + two tokens.
        let config = DefaultPtySessionConfig
            + PtySessionConfigToken::CaptureOsc
            + PtySessionConfigToken::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);

        // DefaultPtySessionConfig + three tokens.
        let sz = vp_width(80) + vp_height(24);
        let config = DefaultPtySessionConfig
            + PtySessionConfigToken::CaptureOsc
            + PtySessionConfigToken::CaptureOutput
            + PtySessionConfigToken::Size(sz);
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::Capture);
        assert_eq!(config.pty_size, sz);
    }

    #[test]
    fn test_add_assign_and_chaining() {
        let mut config: PtySessionConfig = DefaultPtySessionConfig.into();
        config += PtySessionConfigToken::CaptureOsc;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);

        let sz = vp_width(100) + vp_height(50);
        let config = config
            + PtySessionConfigToken::Size(sz)
            + PtySessionConfigToken::NoCaptureOutput;
        assert_eq!(config.capture_osc, CaptureFlag::Capture);
        assert_eq!(config.capture_output, CaptureFlag::NoCapture);
        assert_eq!(config.pty_size, sz);
    }

    #[test]
    fn test_default_with_size() {
        let sz = vp_width(120) + vp_height(60);
        let config = DefaultPtySessionConfig + PtySessionConfigToken::Size(sz);
        assert_eq!(config.pty_size, sz);
        assert_eq!(config.capture_output, CaptureFlag::Capture); // Default
        assert_eq!(config.capture_osc, CaptureFlag::NoCapture); // Default
    }
}
