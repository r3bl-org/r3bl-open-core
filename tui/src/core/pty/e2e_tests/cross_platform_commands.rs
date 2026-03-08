// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words findstr

use crate::PtySessionBuilder;
use std::path::Path;

/// Returns a [`PtySessionBuilder`] configured for a cross-platform [`cat`] command that
/// echoes [`stdin`] to [`stdout`] verbatim (line by line, preserving order).
///
/// [`cat`]: https://en.wikipedia.org/wiki/Cat_(Unix)
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
pub fn cat() -> PtySessionBuilder {
    #[cfg(unix)]
    {
        let paths = ["/usr/bin/cat", "/bin/cat"];
        for path in paths {
            if Path::new(path).exists() {
                return PtySessionBuilder::new(path);
            }
        }
        PtySessionBuilder::new("cat")
    }
    #[cfg(windows)]
    {
        // `findstr "^"` echoes all stdin lines verbatim (unlike `sort` which
        // reorders lines alphabetically, breaking multi-line test assertions).
        PtySessionBuilder::new("cmd.exe").cli_args(["/c", "findstr \"^\""])
    }
}

/// Returns a [`PtySessionBuilder`] configured for a cross-platform [`sleep`] command
/// that sleeps for the specified number of seconds.
///
/// [`sleep`]: https://en.wikipedia.org/wiki/Sleep_(Unix)
pub fn sleep(seconds: u64) -> PtySessionBuilder {
    #[cfg(unix)]
    {
        let paths = ["/usr/bin/sleep", "/bin/sleep"];
        for path in paths {
            if Path::new(path).exists() {
                return PtySessionBuilder::new(path).cli_arg(seconds.to_string());
            }
        }
        PtySessionBuilder::new("sleep").cli_arg(seconds.to_string())
    }
    #[cfg(windows)]
    {
        PtySessionBuilder::new("timeout.exe").cli_args([
            "/t",
            &seconds.to_string(),
            "/nobreak",
        ])
    }
}

/// Returns a [`PtySessionBuilder`] configured for a cross-platform shell ([`sh`] or
/// [`cmd`]).
///
/// [`cmd`]: https://en.wikipedia.org/wiki/Command_Prompt
/// [`sh`]: https://en.wikipedia.org/wiki/Bourne_shell
pub fn bash_or_cmd() -> PtySessionBuilder {
    #[cfg(unix)]
    {
        let paths = ["/usr/bin/sh", "/bin/sh"];
        for path in paths {
            if Path::new(path).exists() {
                return PtySessionBuilder::new(path);
            }
        }
        PtySessionBuilder::new("sh")
    }
    #[cfg(windows)]
    {
        PtySessionBuilder::new("cmd.exe")
    }
}

/// Returns a [`PtySessionBuilder`] configured to emit the given [`OSC`] sequence to
/// [`stdout`].
///
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`stdout`]: std::io::stdout
pub fn printf(osc_sequence: &str) -> PtySessionBuilder {
    #[cfg(unix)]
    {
        let paths = ["/usr/bin/printf", "/bin/printf"];
        for path in paths {
            if Path::new(path).exists() {
                return PtySessionBuilder::new(path).cli_arg(osc_sequence);
            }
        }
        PtySessionBuilder::new("printf").cli_arg(osc_sequence)
    }
    #[cfg(windows)]
    {
        // On Windows, use PowerShell to emit ESC sequences.
        use crate::ESC_STR;
        let ps_cmd = format!(
            "Write-Host -NoNewline \"{}\"",
            osc_sequence.replace(ESC_STR, "$([char]27)")
        );
        PtySessionBuilder::new("powershell.exe").cli_args([
            "-NoProfile",
            "-Command",
            &ps_cmd,
        ])
    }
}
