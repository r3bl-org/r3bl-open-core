# Implement Bracketed Paste Support in PTY Mux

## Context
The `PTY` multiplexer currently has a `bracketed_paste` field in `OfsBufVT100` that is marked with `#[allow(dead_code)]`. The `VT-100` sequences for bracketed paste (`ESC [ ? 2004 h` / `ESC [ ? 2004 l`) are currently ignored by the parser in `vt_100_shim_mode_ops.rs`.

Because the multiplexer uses crossterm (which enables bracketed paste on the host terminal globally), all pasted text arrives at `PTYMux` as a single `Event::Paste(text)`. We need to dynamically route this paste event to the active child process, wrapping it in `\x1b[200~` and `\x1b[201~` only if the child process requested bracketed paste mode.

## [ ] Phase 1: Un-ignore the sequence and track state
- Modify `vt_100_shim_mode_ops.rs` to stop ignoring `ESC [ ? 2004 h` and `ESC [ ? 2004 l`.
- Wire these sequences to update `OfsBufVT100.bracketed_paste`.
- Remove the `#[allow(dead_code)]` from `BracketedPasteState` in `ofs_buf_vt_100.rs`.

## [ ] Phase 2: Route `Event::Paste` in `InputRouter`
- Update `InputRouter::handle_input` to properly handle `crossterm::event::Event::Paste(text)`.
- When a `Paste` event is received, query the active process's `terminal_state.bracketed_paste`.
- If `Enabled`, send `\x1b[200~` + `text` + `\x1b[201~` to the child PTY.
- If `Disabled`, simply send the raw `text` to the child PTY.

## [ ] Phase 3: Verification
- Add integration tests verifying that pasting text routes correctly based on the active process's bracketed paste mode.
- Update `prepare-v0.8.0-meta-task.md` and check this task off.
