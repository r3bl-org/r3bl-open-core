// Copyright (c) 2026 R3BL LLC. Licensed under Apache License, Version 2.0.

/// Target output channel for upgrade messages, spinners, and progress reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserFeedbackChannel {
    /// Write upgrade output to [`stdout`] (default for standard CLI and TUI tools).
    ///
    /// [`stdout`]: std::io::stdout
    Stdout,

    /// Write upgrade output exclusively to [`stderr`] (mandatory for
    /// `rust-analyzer-mcp-server` and [`stdout`]-piped utilities like `env-source` so
    /// [`stdout`] stays 100% uncorrupted).
    ///
    /// [`stderr`]: std::io::stderr
    /// [`stdout`]: std::io::stdout
    Stderr,

    /// Suppress terminal output and emit structured `tracing` events only.
    Quiet,
}

impl UserFeedbackChannel {
    /// Prints a line to the configured feedback channel.
    pub fn println(&self, msg: impl AsRef<str>) {
        match self {
            Self::Stdout => println!("{}", msg.as_ref()),
            Self::Stderr => eprintln!("{}", msg.as_ref()),
            Self::Quiet => {
                // % is Display, ? is Debug.
                tracing::info!(message = %msg.as_ref());
            }
        }
    }

    /// Prints an error line to the configured feedback channel ([`stderr`] for both
    /// `Stdout` and `Stderr` channels, `tracing::error!` for `Quiet`).
    ///
    /// [`stderr`]: std::io::stderr
    pub fn eprintln(&self, msg: impl AsRef<str>) {
        match self {
            Self::Stdout | Self::Stderr => eprintln!("{}", msg.as_ref()),
            Self::Quiet => {
                // % is Display, ? is Debug.
                tracing::error!(message = %msg.as_ref());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_feedback_channel_dispatch() {
        let channels = [
            UserFeedbackChannel::Stdout,
            UserFeedbackChannel::Stderr,
            UserFeedbackChannel::Quiet,
        ];

        for channel in channels {
            // Exercises Stdout (println!), Stderr (eprintln!), and Quiet
            // (tracing::info!).
            channel.println("test line");
            // Exercises Stdout|Stderr (eprintln!) and Quiet (tracing::error!).
            channel.eprintln("test error line");
        }
    }
}
