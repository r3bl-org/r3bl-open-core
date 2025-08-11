/*
 *   Copyright (c) 2025 R3BL LLC
 *   All rights reserved.
 *
 *   Licensed under the Apache License, Version 2.0 (the "License");
 *   you may not use this file except in compliance with the License.
 *   You may obtain a copy of the License at
 *
 *   http://www.apache.org/licenses/LICENSE-2.0
 *
 *   Unless required by applicable law or agreed to in writing, software
 *   distributed under the License is distributed on an "AS IS" BASIS,
 *   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *   See the License for the specific language governing permissions and
 *   limitations under the License.
 */

use std::path::PathBuf;

use portable_pty::{CommandBuilder, MasterPty, SlavePty};

use super::OscEvent;

// Buffer size for reading PTY output.
pub const READ_BUFFER_SIZE: usize = 4096;

// Type aliases for better readability.
pub type Controlled = Box<dyn SlavePty + Send>;
pub type Controller = Box<dyn MasterPty>;
pub type ControlledChild = Box<dyn portable_pty::Child>;

/// Unified event type for PTY output that can contain both OSC sequences and raw output
/// data.
#[derive(Debug, Clone)]
pub enum PtyEvent {
    /// OSC sequence event (if OSC capture is enabled).
    Osc(OscEvent),
    /// Raw output data (stdout/stderr combined).
    Output(Vec<u8>),
    /// Process exited with status.
    Exit(portable_pty::ExitStatus),
}

/// Configuration builder for PTY commands with sensible defaults.
///
/// This builder ensures critical settings are not forgotten when creating PTY commands:
/// - Automatically sets the current working directory if not specified
/// - Provides methods for common terminal environment variables
/// - Ensures commands spawn in the correct context (not in `$HOME`)
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

    /// Builds the final [`CommandBuilder`] with all configurations applied.
    ///
    /// Always sets a working directory - uses the provided one or defaults to current
    /// directory. This is critical to ensure the PTY starts in the expected location,
    /// since by default it uses `$HOME`.
    ///
    /// # Returns
    /// * `Ok(CommandBuilder)` - Configured command ready for PTY execution
    /// * `Err(miette::Error)` - If current directory cannot be determined
    ///
    /// # Errors
    /// Returns an error if the current directory cannot be determined when no
    /// working directory was explicitly provided.
    ///
    /// # Panics
    /// Panics if `cwd` is `None` after attempting to set it to the current directory,
    /// which should be impossible in practice.
    pub fn build(mut self) -> miette::Result<CommandBuilder> {
        // Ensure working directory is always set - use current if not specified. This
        // prevents PTY from spawning in an unexpected location.
        if self.cwd.is_none() {
            let current_dir = std::env::current_dir()
                .map_err(|e| miette::miette!("Failed to get current directory: {}", e))?;
            self = self.cwd(current_dir);
        }

        // Create the command to return.
        let mut cmd_to_return = CommandBuilder::new(&self.command);

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

        // Apply all environment variables.
        for (key, value) in &self.env_vars {
            cmd_to_return.env(key, value);
        }

        Ok(cmd_to_return)
    }
}
