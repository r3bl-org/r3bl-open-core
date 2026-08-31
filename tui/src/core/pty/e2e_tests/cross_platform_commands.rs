// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

use crate::PtySessionBuilder;

/// Returns a [`PtySessionBuilder`] configured for a cross-platform [`cat`] command that
/// echoes [`stdin`] to [`stdout`] verbatim (line by line, preserving order).
///
/// [`cat`]: https://en.wikipedia.org/wiki/Cat_(Unix)
/// [`stdin`]: std::io::stdin
/// [`stdout`]: std::io::stdout
pub fn cat() -> PtySessionBuilder {
    #[cfg(unix)]
    {
        PtySessionBuilder::new("cat")
    }
    #[cfg(windows)]
    {
        // `findstr.exe "^"` echoes all stdin lines verbatim (unlike `sort` which
        // reorders lines alphabetically, breaking multi-line test assertions).
        PtySessionBuilder::new("findstr.exe").cli_arg("^")
    }
}

/// Returns a [`PtySessionBuilder`] configured for a cross-platform [`sleep`] command
/// that sleeps for the specified number of seconds.
///
/// [`sleep`]: https://en.wikipedia.org/wiki/Sleep_(Unix)
pub fn sleep(seconds: u64) -> PtySessionBuilder {
    #[cfg(unix)]
    {
        PtySessionBuilder::new("sleep").cli_arg(seconds.to_string())
    }
    #[cfg(windows)]
    {
        // `timeout.exe` fails in non-interactive or redirected console environments
        // ("ERROR: Input redirection is not supported, exiting the process
        // immediately."). Using PowerShell with `Start-Sleep` avoids this
        // restriction.
        let ps_cmd = format!("Start-Sleep -Seconds {seconds}");
        PtySessionBuilder::new("powershell.exe").cli_args([
            "-NoProfile",
            "-Command",
            &ps_cmd,
        ])
    }
}

/// Returns a [`PtySessionBuilder`] configured for a cross-platform shell ([`sh`] or
/// [`cmd`]).
///
/// [`cmd`]: https://en.wikipedia.org/wiki/Command_Prompt
/// [`sh`]: https://en.wikipedia.org/wiki/Bourne_shell
pub fn sh() -> PtySessionBuilder {
    #[cfg(unix)]
    {
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

// cspell:words findstr
