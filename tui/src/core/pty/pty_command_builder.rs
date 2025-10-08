// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! PTY command builder for constructing and configuring PTY commands.

use super::pty_core::pty_types::PtyCommand;
use std::path::PathBuf;

/// Configuration builder for PTY commands with sensible defaults.
///
/// # Summary
/// - Builder pattern API for constructing PTY commands with proper configuration
/// - Features: automatic working directory, environment variables, OSC sequence support,
///   command arguments chaining
/// - Prevents common PTY issues like spawning in wrong directory or missing terminal
///   environment settings
/// - Used to create [`PtyCommand`] instances for spawning child processes in PTY sessions
/// - Integrates with cargo, npm, and other CLI tools requiring terminal emulation
///
/// # Examples
///
/// Basic cargo command with OSC sequences:
///
/// ```rust
/// # use r3bl_tui::PtyCommandBuilder;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cmd = PtyCommandBuilder::new("cargo")
///     .args(["build", "--release"])
///     .enable_osc_sequences()
///     .build()?;
/// # Ok(())
/// # }
/// ```
///
/// Command with custom working directory:
///
/// ```rust
/// # use r3bl_tui::PtyCommandBuilder;
/// # use std::env;
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let cmd = PtyCommandBuilder::new("npm")
///     .args(["install"])
///     .cwd(env::temp_dir()) // Use temp dir instead of "/path/to/project"
///     .env("NODE_ENV", "production")
///     .build()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PtyCommandBuilder {
    command: String,
    args: Vec<String>,
    cwd: Option<PathBuf>,
    env_vars: Vec<(String, String)>,
}

impl PtyCommandBuilder {
    /// Creates a new PTY command builder for the specified command.
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
            args: Vec::new(),
            cwd: None,
            env_vars: Vec::new(),
        }
    }

    /// Adds arguments to the command.
    #[must_use]
    pub fn args(mut self, args: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Sets the working directory.
    ///
    /// If not called, defaults to the current directory when [`build()`](Self::build) is
    /// invoked.
    #[must_use]
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    /// Adds an environment variable to the command's environment.
    #[must_use]
    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.push((key.into(), value.into()));
        self
    }

    /// Enables OSC sequence emission by setting appropriate environment variables.
    ///
    /// Cargo requires specific terminal environment variables to emit OSC 9;4 progress
    /// sequences. This method automatically detects and configures the appropriate
    /// environment based on the current terminal:
    ///
    /// - **Windows Terminal**: Detected via `WT_SESSION` (no additional config needed)
    /// - **`ConEmu`**: Detected via `ConEmuANSI=ON` (no additional config needed)
    /// - **`WezTerm`**: Set via `TERM_PROGRAM=WezTerm` (fallback for all platforms)
    ///
    /// This approach ensures maximum compatibility across different terminals and
    /// operating systems, particularly on Windows where Windows Terminal is the
    /// default in Windows 11.
    ///
    /// Here is a link to the Cargo source code that emits these sequences:
    /// - <https://github.com/rust-lang/cargo/blob/master/src/cargo/core/shell.rs#L594-L600>
    #[must_use]
    pub fn enable_osc_sequences(self) -> Self {
        // Windows Terminal sets WT_SESSION automatically, so we don't need to override
        // it.
        if std::env::var("WT_SESSION").is_ok() {
            // Already in Windows Terminal, no need to set anything.
            self
        } else if std::env::var("ConEmuANSI").ok() == Some("ON".into()) {
            // Already in ConEmu with ANSI enabled.
            self
        } else {
            // Fall back to WezTerm which works on all platforms.
            self.env("TERM_PROGRAM", "WezTerm")
        }
    }

    /// Builds the final [`PtyCommand`] with all configurations applied.
    ///
    /// Always sets a working directory - uses the provided one or defaults to current
    /// directory. This is critical to ensure the PTY starts in the expected location,
    /// since by default it uses `$HOME`.
    ///
    /// # Returns
    /// * `Ok(PtyCommand)` - Configured command ready for PTY execution
    /// * `Err(miette::Error)` - If current directory cannot be determined
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined when no
    /// working directory was explicitly provided.
    ///
    /// # Panics
    /// Panics if `cwd` is `None` after attempting to set it to the current directory,
    /// which should be impossible in practice.
    pub fn build(mut self) -> miette::Result<PtyCommand> {
        // CRITICAL - Ensure working directory is always set - use current if not
        // specified. This prevents PTY from spawning in an unexpected location.
        if self.cwd.is_none() {
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("Failed to get current directory: {}", e))?;
            self = self.cwd(current_dir);
        }

        // Create the PtyCommand to return.
        let mut cmd_to_return = PtyCommand::new(&self.command);

        // Add all arguments.
        for arg in &self.args {
            cmd_to_return.arg(arg);
        }

        // Set the working directory. This is guaranteed to be Some at this point because
        // we ensure it's set above. Using unwrap_or_else with unreachable!() makes the
        // invariant explicit while avoiding clippy warnings.
        let cwd = self.cwd.unwrap_or_else(|| {
            unreachable!("Working directory must be set - we ensure this above")
        });
        cmd_to_return.cwd(cwd);

        // Apply all user-specified environment variables (these override defaults)
        for (key, value) in &self.env_vars {
            tracing::debug!("Applying user env var: {}={}", key, value);
            cmd_to_return.env(key, value);
        }

        Ok(cmd_to_return)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_pty_command_builder_new() {
        let builder = PtyCommandBuilder::new("test");
        assert_eq!(builder.command, "test");
        assert!(builder.args.is_empty());
        assert!(builder.cwd.is_none());
        assert!(builder.env_vars.is_empty());
    }

    #[test]
    fn test_pty_command_builder_args() {
        let builder = PtyCommandBuilder::new("test").args(["arg1", "arg2"]);

        assert_eq!(builder.args, vec!["arg1", "arg2"]);
    }

    #[test]
    fn test_pty_command_builder_cwd() {
        let path = env::temp_dir();
        let builder = PtyCommandBuilder::new("test").cwd(&path);

        assert_eq!(builder.cwd, Some(path));
    }

    #[test]
    fn test_pty_command_builder_env() {
        let builder = PtyCommandBuilder::new("test")
            .env("KEY1", "value1")
            .env("KEY2", "value2");

        assert_eq!(
            builder.env_vars,
            vec![
                ("KEY1".to_string(), "value1".to_string()),
                ("KEY2".to_string(), "value2".to_string())
            ]
        );
    }

    #[test]
    fn test_pty_command_builder_build() {
        let builder = PtyCommandBuilder::new("ls")
            .args(["-la", "-h"])
            .env("TEST_VAR", "test_value");

        let result = builder.build();
        assert!(result.is_ok());

        let _pty_command = result.unwrap();
        // PtyCommand doesn't expose get_program(), so we just verify build succeeds
    }

    #[test]
    fn test_pty_command_builder_build_with_cwd() {
        let temp_dir = env::temp_dir();
        let builder = PtyCommandBuilder::new("test").cwd(&temp_dir);

        let result = builder.build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_pty_command_builder_chaining() {
        let builder = PtyCommandBuilder::new("cargo")
            .args(["build", "--release"])
            .cwd(env::current_dir().unwrap())
            .env("CARGO_TERM_COLOR", "always")
            .enable_osc_sequences();

        let result = builder.build();
        assert!(result.is_ok());
    }
}
