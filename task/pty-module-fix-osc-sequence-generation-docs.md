# Task: Improve rustdoc for `enable_osc_sequences()` and example file

## Context

The `enable_osc_sequences()` method on `PtySessionBuilder` sets two env vars
(`CARGO_TERM_PROGRESS_WHEN=always` and `TERM=xterm-256color`) but the docs only list them
without explaining **why** each is needed or how they work together. The example file
`spawn_pty_output_capture.rs` also lacks this rationale.

The goal: explain that both env vars are needed so that cargo reliably emits OSC 9;4 progress
sequences, which the PTY infrastructure captures and turns into `OscEvent` variants for
real-time progress/status bar updates.

## Files to modify

### 1. `tui/src/core/pty/pty_session/pty_session_builder.rs` (lines 97-105)

Expand the `enable_osc_sequences()` doc comment to explain:

- **Opening line**: Purpose — enables real-time build progress capture from cargo/rustup
  commands by setting env vars that force OSC 9;4 sequence emission.
- **`CARGO_TERM_PROGRESS_WHEN=always`**: Forces cargo to always emit progress, bypassing its
  own heuristics about when to show a progress bar. Without this, cargo may suppress progress
  output even inside a PTY.
- **`TERM=xterm-256color`**: Signals that the terminal supports modern escape sequences.
  Specifically, this value passes the terminal capability exclusion list in
  `examine_env_vars_to_determine_hyperlink_support()` (where plain `"xterm"` is excluded).
  It also ensures cargo trusts the terminal enough to emit OSC sequences.
- **Downstream pipeline**: Mention that the emitted OSC bytes are parsed by `OscBuffer` into
  `OscEvent` variants like `ProgressUpdate(u8)`, delivered via the PTY session's MPSC channel.
- **Reference-style intra-doc links** at bottom (codebase convention):
  - `` [`cargo`] `` → `https://github.com/rust-lang/cargo`
  - `` [`OSC`] `` → `crate::osc_codes::OscSequence`
  - `` [`examine_env_vars_to_determine_hyperlink_support()`] `` → `crate::examine_env_vars_to_determine_hyperlink_support`
  - `` [`OscBuffer`] `` → `crate::OscBuffer`
  - `` [`OscEvent`] `` → `crate::OscEvent`
  - `` [`ProgressUpdate`] `` → `crate::OscEvent::ProgressUpdate`

### 2. `tui/examples/spawn_pty_output_capture.rs` (lines 3-28)

**DRY principle**: Don't duplicate the env var rationale here. Instead, add a brief sentence
with an intra-doc link to `PtySessionBuilder::enable_osc_sequences()` as the single source
of truth. Something like: "See `PtySessionBuilder::enable_osc_sequences()` for the
environment variables required to trigger OSC emission."

Also fix the usage command on line 24: `spawn_pty_read_channel` → `spawn_pty_output_capture`.

## Not changing

- No logic changes — documentation only.
- Not modifying `detect_color_support.rs` or `upgrade_check.rs` — they are link targets, not
  targets for editing.

## Verification

1. `./check.fish --doc` — verify all intra-doc links resolve.
2. `./check.fish --clippy` — check for style issues.
