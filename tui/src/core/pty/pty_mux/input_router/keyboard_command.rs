// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

#[allow(unused_imports, reason = "Allows short link ref defs in rustdocs")]
use crate::core::pty::pty_mux;
use crate::{KeyPress, LengthOps as _, ModifierKeysMask, OfsBufVT100,
            PAGE_UP_OR_DOWN_SCROLL_BY_FACTOR, PtyInputEvent, ScrollbackAmount,
            SpecialKey, key_press};

/// Represents the explicit command to take for a keyboard event.
#[derive(Debug)]
pub enum KeyboardCommand {
    /// Scroll the virtual terminal viewport history up (intercepted).
    ScrollHistoryBack(ScrollbackAmount),

    /// Scroll the virtual terminal viewport history down (intercepted).
    ScrollHistoryForward(ScrollbackAmount),

    /// Forward the keystroke to the child process (and reset scroll).
    ///
    /// "Scroll on keystroke" behavior: Snap back to the live output whenever the user
    /// types a key that is sent to the process.
    ForwardToProcess(PtyInputEvent),

    /// The keystroke is invalid/unsupported and should be dropped.
    Ignore,
}

impl From<(KeyPress, &OfsBufVT100)> for KeyboardCommand {
    /// Derives the appropriate [`KeyboardCommand`] by evaluating the keystroke against
    /// the state of the active [virtual terminal tab].
    ///
    /// # TL;DR Mental Model
    ///
    /// 1. **The Goal:** We need to hijack scroll inputs because most of the time we're
    ///    running inside a host terminal like `Wezterm`.
    /// 2. **The Boundaries:** The code doesn't actually depend on `Wezterm` or any host
    ///    terminal. [`PTYMux`] is just a headless engine doing math on keystrokes,
    ///    regardless of where they came from.
    /// 3. **The Line-Discipline Distinction:** This routing decision has nothing to do
    ///    with Raw/Cooked mode, it's strictly about Primary vs Alternate screen buffers.
    ///
    /// # The Environment (Host vs Virtual)
    ///
    /// To understand why we route keys this way, it is important to understand the
    /// environment this code runs in.
    ///
    /// For most applications, the [`PTYMux`] engine runs within a full TUI app inside a
    /// host terminal emulator (e.g., `Wezterm`, `iTerm2`). To justify our routing logic,
    /// we make the architectural assumption that the TUI app has placed this host
    /// terminal into the **Alternate Screen Buffer** (e.g., via
    /// [`crate::ansi_output::terminal_modes::enter_alternate_screen`]).
    ///
    /// > This routing decision is based entirely on the active screen buffer and is
    /// > completely unrelated to whether the terminal is in [raw or cooked mode].
    ///
    /// We cannot programmatically verify this, **nor do we need to**. It is entirely
    /// feasible that [`PTYMux`] is not running in a host terminal at all, and that
    /// keystrokes are being synthetically injected. It does not matter. We lay out these
    /// assumptions simply to explain the primary use case that drives this code path:
    /// when a host terminal is in the alternate screen, it disables its native scrollback
    /// UI and passes scroll keys (like `Shift+PageUp` / `Shift+PageDown`) directly down
    /// to our application as raw keystrokes.
    ///
    /// [`PTYMux`] acts as a virtual terminal emulator for the child processes running
    /// inside it. When it receives scroll keystrokes from the host (or synthetic
    /// keystrokes in case the [`PTYMux`] engine is not running in a full TUI app inside a
    /// terminal emulator like `Wezterm`), we must decide whether to consume them (to
    /// scroll our own UI) or pass them through to the active child process.
    ///
    /// # The Decision Matrix
    ///
    /// The decision to intercept or forward is based entirely on the state of the active
    /// [`OfsBufVT100`] virtual tab:
    ///
    /// - **Primary Screen Buffer (e.g., `bash`, `ls`)**: The child process is doing
    ///   normal line-by-line output and expects the terminal emulator to handle
    ///   scrollback history. If we see a scroll keystroke, we intercept it and return a
    ///   [`ScrollHistoryBack`] or [`ScrollHistoryForward`] command to move our internal
    ///   viewport.
    /// - **Alternate Screen Buffer (e.g., `vim`, `htop`)**: The child process has drawn a
    ///   fullscreen UI and wants absolute control over all user input. If we see a scroll
    ///   keystroke, we do **not** intercept it. Instead, we return a [`ForwardToProcess`]
    ///   command so the child process can handle it.
    ///
    /// [`Alacritty`]: https://alacritty.org/
    /// [`crate::ansi_output::terminal_modes::enter_alternate_screen`]:
    ///     crate::crate::ansi_output::terminal_modes::enter_alternate_screen
    /// [`ForwardToProcess`]: KeyboardCommand::ForwardToProcess
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`PTYMux`]: crate::PTYMux
    /// [`ScrollHistoryBack`]: KeyboardCommand::ScrollHistoryBack
    /// [`ScrollHistoryForward`]: KeyboardCommand::ScrollHistoryForward
    /// [raw or cooked mode]: crate::raw_mode::RawMode
    /// [virtual terminal tab]:
    ///     pty_mux#virtual-terminal-architecture-the-virtual-tab-mental-model
    fn from(args: (KeyPress, &OfsBufVT100)) -> Self {
        let (key_press, active_buffer) = args;

        let is_in_primary_screen = active_buffer.is_in_primary_screen();
        let row_height = active_buffer.ofs_buf.get_window_size().row_height;

        let kp_shift_page_up = key_press!(
            @special ModifierKeysMask::new().with_shift(), SpecialKey::PageUp
        );
        let kp_shift_page_down = key_press!(
            @special ModifierKeysMask::new().with_shift(), SpecialKey::PageDown
        );

        if is_in_primary_screen {
            let amount = (row_height / PAGE_UP_OR_DOWN_SCROLL_BY_FACTOR).clamp_to_min(1);
            let scrollback_amount = (amount.as_usize()).into();

            if key_press == kp_shift_page_up {
                return KeyboardCommand::ScrollHistoryBack(scrollback_amount);
            }

            if key_press == kp_shift_page_down {
                return KeyboardCommand::ScrollHistoryForward(scrollback_amount);
            }
        }

        let maybe_pty_input_event: Option<PtyInputEvent> = key_press.into();
        match maybe_pty_input_event {
            // Could not recognize the keypress into a PtyInputEvent. Eg:
            // key_press is KittyKeyboardProtocol.
            None => KeyboardCommand::Ignore,
            // Valid keypress detected. Eg: key_press is Character('a'), Enter, Backspace,
            // etc.
            Some(it) => KeyboardCommand::ForwardToProcess(it),
        }
    }
}
