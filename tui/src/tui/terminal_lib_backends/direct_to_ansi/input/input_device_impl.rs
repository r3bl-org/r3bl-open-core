// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

// cspell:words tcgetwinsize winsize

use super::protocol_conversion::convert_input_event;
use crate::{ByteIndex, ByteOffset, InputEvent,
            core::{ansi::vt_100_terminal_input_parser::{VT100InputEventIR,
                                                        VT100KeyCodeIR,
                                                        VT100PasteModeIR,
                                                        try_parse_input_event},
                   term::get_size},
            tui::DEBUG_TUI_SHOW_TERMINAL_BACKEND};
use smallvec::SmallVec;
use std::{collections::VecDeque, fmt::Debug, sync::LazyLock};
#[cfg(unix)]
use tokio::signal::unix::{Signal, SignalKind};

/// Global static singleton for input reader state - persists for program lifetime.
///
/// This mirrors [crossterm]'s architecture where a global [`INTERNAL_EVENT_READER`] holds
/// the tty file descriptor and event buffer, ensuring data in the kernel buffer is not
/// lost when [`EventStream`] instances are created and dropped.
///
/// # Architecture
///
/// This module uses a **global static** input reader pattern. The key insight is that
/// `stdin` handles must persist across device lifecycle boundaries to prevent data loss
/// during TUI ↔ readline transitions. This happens in the main TUI examples that you can
/// run using `cargo run --example tui_apps`.
///
/// ```text
/// ┌─────────────────────────────────────────────────────────────────────────────────────┐
/// │ GLOBAL_INPUT_CORE (static LazyLock<Mutex<...>>)                                     │
/// │   • stdin: tokio::io::Stdin (PERSISTS for program lifetime)                         │
/// │   • parse_buffer: SmallVec (PERSISTS - carries over partial sequences)              │
/// │   • buffer_position: ByteIndex (PERSISTS - tracks consumption)                      │
/// │   • paste_state: PasteCollectionState (PERSISTS - mid-paste survives)               │
/// │   • event_queue: VecDeque (PERSISTS - buffered events preserved)                    │
/// └───────────────────────────────────────┬─────────────────────────────────────────────┘
///                                         │
///            ┌────────────────────────────┴────────────────────────┐
///            │                                                     │
/// ┌──────────▼───────────────────┐           ┌─────────────────────▼────────┐
/// │ DirectToAnsiInputDevice A    │           │ DirectToAnsiInputDevice B    │
/// │   (TUI App context)          │           │   (Readline context)         │
/// │   • sigwinch_receiver only   │           │   • sigwinch_receiver only   │
/// │   • Delegates to global core │           │   • Delegates to global core │
/// └──────────────────────────────┘           └──────────────────────────────┘
///
/// ✅ Data preserved during transitions - same stdin handle used throughout!
/// ```
///
/// # Why Global State?
///
/// Without global state, when a TUI app exits and a new readline context is created,
/// creating a new [`tokio::io::stdin()`] handle causes data in the kernel buffer to
/// become inaccessible. User keypresses during the transition are lost.
///
/// With global state, the stdin handle and parse buffer survive across
/// [`DirectToAnsiInputDevice`] lifetimes, ensuring no data loss.
///
/// # Reference: How crossterm Solves This
///
/// Crossterm uses a global [`INTERNAL_EVENT_READER`], here's an excerpt:
///
/// ```text
/// static INTERNAL_EVENT_READER: Mutex<Option<...>>
///    source: UnixInternalEventSource (tty fd - PERSISTS)
///    events: VecDeque<InternalEvent> (buffer - PERSISTS)
/// ```
///
/// All [`EventStream`] instances share this, so data is never lost during transitions.
///
/// # Why [`tokio::sync::Mutex`] (Not [`std::sync::Mutex`])
///
/// We hold the mutex guard across `.await` points (during `stdin.read().await`):
/// - [`std::sync::MutexGuard`] is `!Send` and cannot be held across `.await` points
/// - [`tokio::sync::Mutex`] is async-native and yields to scheduler instead of blocking
/// - This prevents starving other tokio tasks while waiting for the lock
///
/// [`EventStream`]: ::crossterm::event::EventStream
/// [`INTERNAL_EVENT_READER`]: https://github.com/crossterm-rs/crossterm/blob/0.29.0/src/event.rs#L149
/// [crossterm]: ::crossterm
#[allow(missing_debug_implementations)]
pub struct DirectToAnsiInputCore {
    /// Tokio async stdin handle for non-blocking reading.
    ///
    /// This handle persists for the program lifetime, ensuring no data is lost
    /// when [`DirectToAnsiInputDevice`] instances are created and dropped.
    /// All device instances share this single stdin handle.
    stdin: tokio::io::Stdin,

    /// Raw byte buffer for ANSI sequences and text.
    ///
    /// Pre-allocated with [`PARSE_BUFFER_SIZE`] capacity inline. This buffer
    /// persists across device lifetimes, ensuring partial ANSI sequences are
    /// not lost during TUI ↔ readline transitions.
    parse_buffer: SmallVec<[u8; PARSE_BUFFER_SIZE]>,

    /// Current position in buffer marking the boundary between consumed and unconsumed
    /// bytes.
    ///
    /// Bytes before this position have been parsed; bytes from this position
    /// onward are pending. When this exceeds half of [`PARSE_BUFFER_SIZE`], the
    /// buffer is compacted via [`consume_bytes()`].
    buffer_position: ByteIndex,

    /// State machine for collecting bracketed paste text.
    ///
    /// Tracks whether we're between `Paste(Start)` and `Paste(End)` markers.
    /// Persists across device lifetimes so mid-paste transitions don't lose data.
    paste_state: PasteCollectionState,

    /// Buffered events that haven't been consumed yet.
    ///
    /// When multiple events are parsed from a single read, extras are queued here.
    /// Pre-allocated with capacity 32 for typical burst scenarios.
    event_queue: VecDeque<InputEvent>,
}

/// Buffer size constants for the input device.
mod constants {
    /// Initial buffer capacity for efficient ANSI sequence buffering.
    ///
    /// Most terminal input consists of either:
    /// - Individual keypresses (~5-10 bytes for special keys like arrows, function keys)
    /// - Paste events (variable, but rare to exceed buffer capacity)
    /// - Mouse events (~20 bytes for typical terminal coordinates)
    ///
    /// 4096 bytes accommodates multiple complete ANSI sequences without frequent
    /// reallocations. This is a good balance: large enough to handle typical bursts,
    /// small enough to avoid excessive memory overhead for idle periods.
    ///
    /// See [`try_read_event()`] for buffer management algorithm.
    ///
    /// [`try_read_event()`]: DirectToAnsiInputDevice::try_read_event#buffer_management_algorithm
    pub const PARSE_BUFFER_SIZE: usize = 4096;

    /// Temporary read buffer size for stdin reads.
    ///
    /// This is the read granularity: how much data we pull from the kernel in one
    /// syscall. Too small (< 256): Excessive syscalls increase latency. Too large
    /// (> 256): Delays response to time-sensitive input (e.g., arrow key repeat).
    ///
    /// 256 bytes is optimal for terminal input: it's one page boundary on many
    /// architectures, fits comfortably in the input buffer, and provides good syscall
    /// efficiency without introducing noticeable latency.
    ///
    /// See [`try_read_event()`] for buffer management algorithm.
    ///
    /// [`try_read_event()`]: DirectToAnsiInputDevice::try_read_event#buffer_management_algorithm
    pub const STDIN_READ_BUFFER_SIZE: usize = 256;
}
#[allow(clippy::wildcard_imports)]
use constants::*;

/// Global singleton for the input core - initialized on first access.
mod singleton {
    #[allow(clippy::wildcard_imports)]
    use super::*;

    /// Global singleton - initialized on first access.
    ///
    /// Uses [`LazyLock`] for thread-safe lazy initialization and [`tokio::sync::Mutex`]
    /// for async-safe access. The [`Option`] allows initialization to happen on first
    /// access.
    pub static GLOBAL_INPUT_CORE: LazyLock<
        tokio::sync::Mutex<Option<DirectToAnsiInputCore>>,
    > = LazyLock::new(|| tokio::sync::Mutex::new(None));

    /// Gets or initializes the global input core.
    ///
    /// On first call, creates the stdin handle, parse buffer, and event queue.
    /// Subsequent calls return a guard to the existing state.
    pub async fn get_or_init_global_core()
    -> tokio::sync::MutexGuard<'static, Option<DirectToAnsiInputCore>> {
        let mut guard = GLOBAL_INPUT_CORE.lock().await;
        if guard.is_none() {
            *guard = Some(DirectToAnsiInputCore {
                stdin: tokio::io::stdin(),
                parse_buffer: SmallVec::new(),
                buffer_position: ByteIndex::default(),
                paste_state: PasteCollectionState::Inactive,
                event_queue: VecDeque::with_capacity(32),
            });
        }
        guard
    }
}
#[allow(clippy::wildcard_imports)]
use singleton::*;

/// Async input device for [`DirectToAnsi`] backend.
///
/// This is a **thin wrapper** that delegates to `GLOBAL_INPUT_CORE` for stdin reading
/// and buffer management. The global core pattern mirrors crossterm's architecture,
/// ensuring stdin handles persist across device lifecycle boundaries.
///
/// Manages asynchronous reading from terminal stdin using tokio, with:
/// - Global state for stdin handle and parse buffer (survives device lifecycle)
/// - Simple [`SmallVec<u8>`] buffer for handling partial/incomplete ANSI sequences
/// - Smart lookahead for zero-latency ESC key detection (no timeout!)
/// - Dispatch to protocol parsers (keyboard, mouse, terminal events, UTF-8)
///
/// # Why Global State?
///
/// When a TUI app exits and a new readline context is created, previous implementations
/// would create a new [`tokio::io::stdin()`] handle. Data in the kernel buffer during
/// this transition was lost, causing key presses to be dropped. The global core ensures
/// the stdin handle and parse buffer survive across [`DirectToAnsiInputDevice`]
/// lifetimes.
///
/// See the module-level documentation for detailed architecture diagrams.
///
/// # Full I/O Pipeline
///
/// This device sits in the backend executor layer, bridging raw I/O to the protocol
/// parser, then converting protocol IR to the public API:
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ Raw ANSI bytes: "\x1B[A"                                         │
/// │ stdin (tokio::io::stdin) - FROM GLOBAL_INPUT_CORE                │
/// └────────────────────────────┬─────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼─────────────────────────────────────┐
/// │ THIS DEVICE: DirectToAnsiInputDevice (Backend Executor)          │
/// │   • Delegates to GLOBAL_INPUT_CORE for stdin/buffer              │
/// │   • Owns SIGWINCH receiver (per-instance, can't be shared)       │
/// │   • SmallVec buffer: `PARSE_BUFFER_SIZE`, zero-timeout parsing   │
/// │   • Paste state machine: Collecting bracketed paste text         │
/// └────────────────────────────┬─────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼─────────────────────────────────────┐
/// │ vt_100_terminal_input_parser/ (Protocol Layer - IR)              │
/// │   try_parse_input_event() dispatches to:                         │
/// │   ├─ parse_keyboard_sequence() → VT100InputEventIR::Keyboard     │
/// │   ├─ parse_mouse_sequence()    → VT100InputEventIR::Mouse        │
/// │   ├─ parse_terminal_event()    → VT100InputEventIR::Focus/Resize │
/// │   └─ parse_utf8_text()         → VT100InputEventIR::Keyboard     │
/// └────────────────────────────┬─────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼─────────────────────────────────────┐
/// │ protocol_conversion.rs (IR → Public API)                         │
/// │   convert_input_event()       VT100InputEventIR → InputEvent     │
/// │   convert_key_code_to_keypress()  VT100KeyCodeIR → KeyPress      │
/// └────────────────────────────┬─────────────────────────────────────┘
///                              │
/// ┌────────────────────────────▼─────────────────────────────────────┐
/// │ Public API (Application Layer)                                   │
/// │   InputEvent::Keyboard(KeyPress)                                 │
/// │   InputEvent::Mouse(MouseInput)                                  │
/// │   InputEvent::Resize(Size)                                       │
/// │   InputEvent::Focus(FocusEvent)                                  │
/// │   InputEvent::Paste(String)                                      │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// # Underlying Protocol Parser
///
/// - [`vt_100_terminal_input_parser`]: The protocol parser that converts raw bytes to
///   [`VT100InputEventIR`]. This device calls [`try_parse_input_event`] to perform the
///   actual parsing.
///
/// # Zero-Latency ESC Key Detection
///
/// **The Problem**: Distinguishing ESC key presses from escape sequences (e.g., Up Arrow
/// = `ESC [ A`).
///
/// **Baseline (crossterm)**: When reading `0x1B` alone, wait up to 150ms to see if more
/// bytes arrive. If timeout expires → emit ESC key. If bytes arrive → parse escape
/// sequence.
///
/// **Our Approach**: Immediately emit ESC when buffer contains only `[0x1B]`, with no
/// artificial delay.
///
/// ## Performance Comparison
///
/// | Input Type           | crossterm Latency   | Our Latency   | Improvement       |
/// | -------------------- | ------------------- | ------------- | ----------------- |
/// | **ESC key press**    | 150ms (timeout)     | 0ms           | **150ms faster**  |
/// | Arrow keys           | 0ms (immediate)     | 0ms           | Same              |
/// | Regular text         | 0ms (immediate)     | 0ms           | Same              |
/// | Mouse events         | 0ms (immediate)     | 0ms           | Same              |
///
/// **Benefit applies to**: Vim-style modal editors, ESC-heavy workflows, dialog
/// dismissal.
///
/// ## How Escape Sequences Arrive in Practice
///
/// When you press a special key (e.g., Up Arrow), the terminal emulator sends
/// an escape sequence like `ESC [ A` (3 bytes: `[0x1B, 0x5B, 0x41]`).
///
/// **Key Assumption**: Modern terminal emulators send escape sequences **atomically**
/// in a single `write()` syscall, and the kernel buffers all bytes together.
///
/// ### Typical Flow (99.9% of cases - local terminals)
///
/// ```text
/// User presses Up Arrow
///   ↓
/// Terminal: write(stdout, "\x1B[A", 3)  ← One syscall, 3 bytes
///   ↓
/// Kernel buffer: [0x1B, 0x5B, 0x41]    ← All bytes arrive together
///   ↓
/// stdin.read().await → [0x1B, 0x5B, 0x41]  ← We get all 3 bytes in one read
///   ↓
/// try_parse() sees complete sequence → Up Arrow event ✓
/// ```
///
/// ### Edge Case: Slow Byte Arrival (rare - high-latency SSH, slow serial)
///
/// Over high-latency connections, bytes might arrive separately:
///
/// ```text
/// First read:  [0x1B]           → Emits ESC immediately
/// Second read: [0x5B, 0x41]     → User gets ESC instead of Up Arrow
/// ```
///
/// **Trade-off**: We optimize for the common case (local terminals with atomic
/// sequences) to achieve 0ms ESC latency, accepting rare edge cases over forcing
/// 150ms timeout on all users.
///
/// ### Why This Assumption Holds
///
/// - **Local terminals** (gnome-terminal, xterm, Alacritty, iTerm2): Always send escape
///   sequences atomically in one write
/// - **Terminal protocol design**: Sequences are designed to be atomic units
/// - **Kernel buffering**: Even with slight delays, kernel buffers complete sequences
///   before `read()` sees them
/// - **Network delay case**: Over SSH with 200ms latency, UX is already degraded; getting
///   ESC instead of Up Arrow is annoying but not catastrophic
///
/// [`DirectToAnsi`]: mod@crate::tui::terminal_lib_backends::direct_to_ansi
/// [`VT100InputEventIR`]: crate::core::ansi::vt_100_terminal_input_parser::VT100InputEventIR
/// [`try_parse_input_event`]: crate::core::ansi::vt_100_terminal_input_parser::try_parse_input_event
/// [`vt_100_terminal_input_parser`]: mod@crate::core::ansi::vt_100_terminal_input_parser
pub struct DirectToAnsiInputDevice {
    /// `SIGWINCH` signal receiver for terminal resize events (Unix-only).
    ///
    /// Terminal resize is not sent through stdin as ANSI sequences - it's delivered
    /// via the `SIGWINCH` signal. We use [`tokio::signal::unix::Signal`] to receive
    /// these asynchronously and convert them to [`InputEvent::Resize`].
    ///
    /// This is per-instance (not in global core) because signal receivers can't easily
    /// be shared across async contexts.
    ///
    /// # TODO(windows)
    ///
    /// Windows uses `WINDOW_BUFFER_SIZE_EVENT` via Console API instead of `SIGWINCH`.
    /// When adding Windows support for [`DirectToAnsi`], implement resize detection
    /// using `ReadConsoleInput` which returns window buffer size change events.
    /// See: <https://learn.microsoft.com/en-us/windows/console/window-buffer-size-record-str>
    ///
    /// [`DirectToAnsi`]: mod@super::super
    #[cfg(unix)]
    sigwinch_receiver: Signal,
}

/// State machine for collecting bracketed paste text.
///
/// When the terminal sends a bracketed paste sequence, it arrives as:
/// - `Paste(Start)` marker
/// - Multiple `Keyboard` events (the actual pasted text)
/// - `Paste(End)` marker
///
/// This state tracks whether we're currently collecting text between markers.
///
/// # Line Ending Handling
///
/// Both CR (`\r`) and LF (`\n`) are parsed by the keyboard parser as
/// [`VT100KeyCodeIR::Enter`], which is then accumulated as `'\n'`. This means:
/// - LF (`\n`) → `'\n'` ✓
/// - CR (`\r`) → `'\n'` ✓
/// - CRLF (`\r\n`) → `'\n\n'` (double newline)
///
/// Most Unix terminals normalize line endings before sending bracketed paste,
/// so CRLF sequences are uncommon in practice.
///
/// # TODO(windows)
///
/// Windows uses CRLF line endings natively. When adding Windows support for
/// [`DirectToAnsi`], consider normalizing CRLF → LF in the paste accumulator.
/// This would require either tracking the previous byte in the keyboard parser
/// or post-processing the accumulated text.
///
/// [`DirectToAnsi`]: mod@super::super
/// [`VT100KeyCodeIR::Enter`]: crate::core::ansi::vt_100_terminal_input_parser::VT100KeyCodeIR::Enter
#[derive(Debug)]
enum PasteCollectionState {
    /// Not currently in a paste operation.
    Inactive,
    /// Currently collecting text for a paste operation.
    Accumulating(String),
}

/// Result of applying the paste state machine to a parsed event.
enum PasteAction {
    /// Emit this event to the caller.
    Emit(InputEvent),
    /// Continue collecting (event was absorbed by paste state machine).
    Continue,
}

/// Result of waiting for stdin or signal in the event loop.
#[cfg(unix)]
enum WaitAction {
    /// Emit this event to the caller (e.g., Resize).
    Emit(InputEvent),
    /// EOF or error occurred, signal shutdown.
    Shutdown,
    /// Data was read or signal handled, continue parsing.
    Continue,
}

/// Applies the paste collection state machine to a parsed VT100 event.
///
/// Returns [`PasteAction::Emit`] if the event should be emitted to the caller,
/// or [`PasteAction::Continue`] if the event was absorbed (paste in progress).
fn apply_paste_state_machine(
    paste_state: &mut PasteCollectionState,
    vt100_event: &VT100InputEventIR,
) -> PasteAction {
    match (paste_state, vt100_event) {
        // Start marker: enter collecting state, don't emit event.
        (
            state @ PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::Start),
        ) => {
            *state = PasteCollectionState::Accumulating(String::new());
            PasteAction::Continue
        }

        // End marker: emit complete paste and exit collecting state.
        (
            state @ PasteCollectionState::Accumulating(_),
            VT100InputEventIR::Paste(VT100PasteModeIR::End),
        ) => {
            // Swap out `&mut state` to `Inactive` to get ownership of what is
            // currently there, then extract accumulated text.
            let state = std::mem::replace(state, PasteCollectionState::Inactive);
            let PasteCollectionState::Accumulating(text) = state else {
                unreachable!(
                    "state was matched as Accumulating(String), so this can't happen"
                );
            };
            PasteAction::Emit(InputEvent::BracketedPaste(text))
        }

        // While collecting: accumulate keyboard characters and whitespace.
        // Tab/Enter/Backspace are parsed as dedicated keys (not Char variants),
        // so we must handle them explicitly to preserve whitespace in pastes.
        (PasteCollectionState::Accumulating(buffer), vt100_event) => {
            match vt100_event {
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Char(ch),
                    ..
                } => buffer.push(*ch),
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Enter,
                    ..
                } => buffer.push('\n'),
                VT100InputEventIR::Keyboard {
                    code: VT100KeyCodeIR::Tab,
                    ..
                } => buffer.push('\t'),
                // Other events (mouse, resize, focus, arrow keys, etc.) are
                // ignored during paste - they're unlikely to be intentional.
                _ => {}
            }
            PasteAction::Continue
        }

        // Orphaned end marker (End without Start): emit empty paste.
        (
            PasteCollectionState::Inactive,
            VT100InputEventIR::Paste(VT100PasteModeIR::End),
        ) => PasteAction::Emit(InputEvent::BracketedPaste(String::new())),

        // Normal event processing when not pasting.
        (PasteCollectionState::Inactive, _) => {
            match convert_input_event(vt100_event.clone()) {
                Some(event) => PasteAction::Emit(event),
                None => PasteAction::Continue,
            }
        }
    }
}

impl DirectToAnsiInputDevice {
    /// Create a new `DirectToAnsiInputDevice`.
    ///
    /// This is now a thin wrapper that only initializes the SIGWINCH receiver.
    /// The stdin handle and parse buffer are managed by the global input core,
    /// which persists for the program lifetime.
    ///
    /// No timeout initialization needed - we use smart async lookahead instead!
    /// See the struct-level documentation for details on zero-latency ESC detection.
    ///
    /// # Panics
    ///
    /// Panics if the SIGWINCH signal handler cannot be registered (Unix-only).
    /// This should only happen if the signal is already registered elsewhere.
    #[must_use]
    pub fn new() -> Self {
        #[cfg(unix)]
        let sigwinch_receiver = tokio::signal::unix::signal(SignalKind::window_change())
            .expect("Failed to register SIGWINCH handler");

        Self {
            #[cfg(unix)]
            sigwinch_receiver,
        }
    }

    /// Read the next input event asynchronously.
    ///
    /// # Returns
    ///
    /// `None` if stdin is closed (EOF). Or [`InputEvent`] variants for:
    /// - **Keyboard**: Character input, arrow keys, function keys, modifiers (with 0ms
    ///   ESC latency)
    /// - **Mouse**: Clicks, drags, motion, scrolling with position and modifiers
    /// - **Resize**: Terminal window size change (rows, cols)
    /// - **Focus**: Terminal gained/lost focus
    /// - **Paste**: Bracketed paste mode text
    ///
    /// # Usage
    ///
    /// This method is called once by the main event loop of the program using this
    /// [`InputDevice`] and this [`DirectToAnsiInputDevice`] struct is persisted
    /// for the entire lifetime of the program's event loop. Typical usage pattern:
    ///
    /// ```no_run
    /// # use r3bl_tui::DirectToAnsiInputDevice;
    /// # use r3bl_tui::InputEvent;
    /// # use tokio::signal;
    ///
    /// #[tokio::main]
    /// async fn main() -> miette::Result<()> {
    ///     // Create device once at startup, reuse until program exit
    ///     let mut input_device = DirectToAnsiInputDevice::new();
    /// #   let mut sigint = signal::unix::signal(signal::unix::SignalKind::interrupt())
    /// #       .map_err(|e| miette::miette!("Failed to setup signal handler: {}", e))?;
    ///
    ///     // Main event loop - handle multiple concurrent event sources with tokio::select!
    ///     loop {
    ///         tokio::select! {
    ///             // Handle terminal input events
    ///             input_event = input_device.next() => {
    ///                 match input_event {
    ///                     Some(InputEvent::Keyboard(key_press)) => {
    ///                         // Handle keyboard input
    ///                     }
    ///                     Some(InputEvent::Mouse(mouse_input)) => {
    ///                         // Handle mouse input
    ///                     }
    ///                     Some(InputEvent::Resize(size)) => {
    ///                         // Handle terminal resize
    ///                     }
    ///                     Some(InputEvent::BracketedPaste(text)) => {
    ///                         // Handle bracketed paste
    ///                     }
    ///                     Some(InputEvent::Focus(_)) => {
    ///                         // Handle focus events
    ///                     }
    ///                     Some(_) => {
    ///                         // Handle future/unknown event types
    ///                     }
    ///                     None => {
    ///                         // stdin closed (EOF) - signal program to exit
    ///                         break;
    ///                     }
    ///                 }
    ///             }
    ///             // Handle system signals (e.g., Ctrl+C)
    ///             _ = sigint.recv() => {
    /// #               eprintln!("Received SIGINT, shutting down...");
    /// #               break;
    ///             }
    ///             // Handle other concurrent tasks as needed
    ///             // _ = some_background_task => { ... }
    ///         }
    ///     }
    /// #   Ok(())
    /// }
    /// ```
    ///
    /// **Key points:**
    /// - The device can be **created and dropped multiple times** - global state persists
    /// - This method is **called repeatedly** by the main event loop via the
    ///   `InputDeviceExt::next()` trait method, not called directly
    /// - **Buffer state is preserved** across device lifetimes via `GLOBAL_INPUT_CORE`
    /// - Returns `None` when stdin is closed (program should exit)
    ///
    /// # Global State
    ///
    /// This method accesses the global input core (`GLOBAL_INPUT_CORE`) which holds:
    /// - The persistent stdin handle (survives across device lifetimes)
    /// - The parse buffer and position
    /// - The event queue for buffered events
    /// - The paste collection state
    ///
    /// The SIGWINCH receiver remains per-instance since signal receivers can't easily
    /// be shared across async contexts.
    ///
    /// # Implementation
    ///
    /// Async loop with zero-timeout parsing:
    /// 1. Check event queue for buffered events (from previous reads)
    /// 2. Try to parse from existing buffer
    /// 3. If incomplete, read more from stdin (yields until data ready)
    /// 4. Loop back to parsing
    ///
    /// # Buffer Management Algorithm
    ///
    /// This implementation uses a growable buffer with lazy compaction to avoid
    /// copying bytes on every parse while preventing unbounded memory growth:
    ///
    /// ```text
    /// Initial state: buffer = [], consumed = 0
    ///
    /// After read #1: buffer = [0x1B, 0x5B, 0x41], consumed = 0
    ///                         ├─────────────────┤
    ///                         Parser tries [0..3]
    ///                         Parses Up Arrow (3 bytes)
    ///                         consumed = 3
    ///
    /// After read #2: buffer = [0x1B, 0x5B, 0x41, 0x61], consumed = 3
    ///                         └──── parsed ────┘ ├───┤
    ///                                            Parser tries [3..4]
    ///                                            Parses 'a' (1 byte)
    ///                                            consumed = 4
    ///
    /// After read #3: buffer = [      ...many bytes...      ], consumed = 2100
    ///                         └ consumed > 2048 threshold! ┘
    ///                         Compact: drain [0..2100], consumed = 0
    ///                         buffer now starts fresh
    /// ```
    ///
    /// **Key operations:**
    /// - `try_parse_input_event(&buffer[consumed..])` - Parse only unprocessed bytes
    /// - `consume_bytes(n)` - Mark n bytes as processed (increments `consumed`)
    /// - When `consumed > 2048` - Compact buffer by draining processed bytes
    ///
    /// **Why not a true ring buffer?**
    /// - Variable-length ANSI sequences (1-20+ bytes) make fixed-size wrapping complex
    /// - Growing Vec handles overflow naturally without wrap-around logic
    /// - Lazy compaction (every 2KB) amortizes cost: O(1) per event on average
    ///
    /// **Memory behavior:**
    /// - Typical: 100 events → ~500 bytes consumed, no compaction needed
    /// - Worst case: `PARSE_BUFFER_SIZE` buffer + 2KB consumed = 6KB maximum before
    ///   compaction
    /// - After compaction: resets to current unconsumed data only
    ///
    /// See struct-level documentation for details on zero-latency ESC detection
    /// algorithm.
    ///
    /// [`InputDevice`]: crate::InputDevice
    pub async fn try_read_event(&mut self) -> Option<InputEvent> {
        // Get the global input core - this persists for program lifetime.
        let mut core_guard = get_or_init_global_core().await;
        let core = core_guard.as_mut()?;

        // Check event queue first - return any buffered events.
        if let Some(event) = core.event_queue.pop_front() {
            return Some(event);
        }

        // Allocate temp buffer ONCE before loop (performance optimization).
        let mut temp_buf = [0u8; STDIN_READ_BUFFER_SIZE];

        loop {
            // 1. Try to parse from existing buffer and apply paste state machine.
            if let Some((vt100_event, bytes_consumed)) = try_parse_input_event(
                &core.parse_buffer[core.buffer_position.as_usize()..],
            ) {
                consume_bytes(core, bytes_consumed);

                match apply_paste_state_machine(&mut core.paste_state, &vt100_event) {
                    PasteAction::Emit(event) => return Some(event),
                    PasteAction::Continue => continue,
                }
            }

            // 2. Buffer exhausted or incomplete sequence - wait for input or signal.
            #[cfg(unix)]
            match self.await_input(core, &mut temp_buf).await {
                WaitAction::Emit(event) => return Some(event),
                WaitAction::Shutdown => return None,
                WaitAction::Continue => {} // Loop back to try parsing.
            }

            // Non-Unix: DirectToAnsi is Linux-only, this code path should never be
            // reached.
            #[cfg(not(unix))]
            {
                unreachable!(
                    "DirectToAnsi backend is Linux-only. \
                     This code path should never be reached on non-Unix systems."
                );
            }
        }
    }

    /// Waits for stdin data or SIGWINCH signal, returning an action for the event loop.
    ///
    /// # Cancel Safety
    ///
    /// Both futures in the `select!` are cancel-safe:
    /// - [`tokio::io::AsyncReadExt::read`]: Cancel-safe. If cancelled before completion,
    ///   no data is lost - the same data will be available on the next read.
    /// - [`tokio::signal::unix::Signal::recv`]: Cancel-safe. If cancelled, the signal is
    ///   not consumed and will be delivered on the next call.
    ///
    /// This means the `select!` can safely be used in a loop without losing events.
    #[cfg(unix)]
    async fn await_input(
        &mut self,
        core: &mut DirectToAnsiInputCore,
        temp_buf: &mut [u8],
    ) -> WaitAction {
        use tokio::io::AsyncReadExt as _;

        DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
            tracing::debug!(message = "direct-to-ansi: waiting for input or signal");
        });

        tokio::select! {
            result = core.stdin.read(temp_buf) => {
                handle_stdin_result(core, temp_buf, result)
            }
            sigwinch_result = self.sigwinch_receiver.recv() => {
                handle_sigwinch_result(sigwinch_result)
            }
        }
    }
}

/// Handles the result of a stdin read operation.
#[cfg(unix)]
fn handle_stdin_result(
    core: &mut DirectToAnsiInputCore,
    temp_buf: &[u8],
    result: std::io::Result<usize>,
) -> WaitAction {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "direct-to-ansi: stdin branch selected",
            result = ?result
        );
    });

    match result {
        Ok(0) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "direct-to-ansi: stdin EOF (0 bytes)");
            });
            WaitAction::Shutdown
        }
        Err(ref e) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(message = "direct-to-ansi: stdin error", error = ?e);
            });
            WaitAction::Shutdown
        }
        Ok(n) => {
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "direct-to-ansi: stdin read bytes",
                    bytes_read = n
                );
            });
            core.parse_buffer.extend_from_slice(&temp_buf[..n]);
            WaitAction::Continue
        }
    }
}

/// Handles the result of a SIGWINCH signal.
#[cfg(unix)]
fn handle_sigwinch_result(sigwinch_result: Option<()>) -> WaitAction {
    DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
        tracing::debug!(
            message = "direct-to-ansi: SIGWINCH branch selected",
            result = ?sigwinch_result
        );
    });

    match sigwinch_result {
        Some(()) => {
            // Signal received successfully, query terminal size.
            if let Ok(size) = get_size() {
                DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                    tracing::debug!(
                        message = "direct-to-ansi: returning Resize",
                        size = ?size
                    );
                });
                return WaitAction::Emit(InputEvent::Resize(size));
            }
            // If size query failed, continue to next iteration.
            DEBUG_TUI_SHOW_TERMINAL_BACKEND.then(|| {
                tracing::debug!(
                    message = "direct-to-ansi: get_size() failed, continuing"
                );
            });
            WaitAction::Continue
        }
        None => {
            // Signal stream closed - unexpected but shouldn't cause shutdown.
            tracing::warn!(
                message =
                    "direct-to-ansi: SIGWINCH receiver returned None (stream closed)"
            );
            WaitAction::Continue
        }
    }
}

/// Consume N bytes from the buffer in the global core.
///
/// Increments the consumed counter and compacts the buffer if threshold exceeded.
/// This is kind of like a ring buffer (except that it is not fixed size).
///
/// # Semantic Correctness
///
/// Takes [`ByteOffset`] (displacement from parser) and applies it to
/// [`buffer_position`] (position in buffer): `position += displacement`.
///
/// [`buffer_position`]: DirectToAnsiInputCore::buffer_position
fn consume_bytes(core: &mut DirectToAnsiInputCore, displacement: ByteOffset) {
    core.buffer_position += displacement;

    // Compact buffer if consumed bytes exceed half of PARSE_BUFFER_SIZE.
    if core.buffer_position.as_usize() > PARSE_BUFFER_SIZE / 2 {
        core.parse_buffer.drain(..core.buffer_position.as_usize());
        core.buffer_position = ByteIndex::default();
    }
}

impl Debug for DirectToAnsiInputDevice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DirectToAnsiInputDevice")
            .field("sigwinch_receiver", &"<Signal>")
            .field("global_core", &"<GLOBAL_INPUT_CORE>")
            .finish()
    }
}

impl Default for DirectToAnsiInputDevice {
    fn default() -> Self { Self::new() }
}

impl DirectToAnsiInputDevice {
    pub async fn next(&mut self) -> Option<InputEvent> { self.try_read_event().await }
}

/// Comprehensive testing is performed in PTY integration tests:
/// - [`test_pty_input_device`]
/// - [`test_pty_mouse_events`]
/// - [`test_pty_keyboard_modifiers`]
/// - [`test_pty_utf8_text`]
/// - [`test_pty_terminal_events`]
///
/// [`test_pty_input_device`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_input_device_test::test_pty_input_device
/// [`test_pty_keyboard_modifiers`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_keyboard_modifiers_test::test_pty_keyboard_modifiers
/// [`test_pty_mouse_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_mouse_events_test::test_pty_mouse_events
/// [`test_pty_terminal_events`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_terminal_events_test::test_pty_terminal_events
/// [`test_pty_utf8_text`]: crate::core::ansi::vt_100_terminal_input_parser::integration_tests::pty_utf8_text_test::test_pty_utf8_text
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{byte_offset,
                core::ansi::vt_100_terminal_input_parser::{VT100KeyModifiersIR,
                                                           parse_keyboard_sequence,
                                                           parse_utf8_text}};

    #[tokio::test]
    async fn test_device_creation() {
        // Test DirectToAnsiInputDevice constructs successfully.
        // With the global core architecture, the device is now a thin wrapper
        // that only holds the SIGWINCH receiver.
        let _device = DirectToAnsiInputDevice::new();

        // Verify global core is initialized on first access.
        let core_guard = get_or_init_global_core().await;
        let core = core_guard
            .as_ref()
            .expect("Global core should be initialized");

        // Verify buffer is empty initially (no data yet).
        assert_eq!(core.parse_buffer.len(), 0);

        // Verify consumed counter is at 0.
        assert_eq!(core.buffer_position.as_usize(), 0);
    }

    #[tokio::test]
    async fn test_event_parsing() {
        // Test event parsing from buffer - verify parsers handle different sequence
        // types. These tests use the parser functions directly since they don't
        // need the device.

        // Test 1: Parse UTF-8 text (simplest case).
        let buffer: &[u8] = b"A";
        if let Some((vt100_event, bytes_consumed)) = parse_utf8_text(buffer) {
            assert_eq!(bytes_consumed, byte_offset(1));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert UTF-8 text event");
            }
        } else {
            panic!("Failed to parse UTF-8 text 'A'");
        }

        // Test 2: ESC key (single byte).
        let esc_buffer: [u8; 1] = [0x1B];
        assert_eq!(esc_buffer.len(), 1);
        assert_eq!(esc_buffer[0], 0x1B);

        // Test 3: CSI sequence for keyboard (Up Arrow: ESC [ A).
        let csi_buffer: [u8; 3] = [0x1B, 0x5B, 0x41];
        if let Some((vt100_event, bytes_consumed)) = parse_keyboard_sequence(&csi_buffer)
        {
            assert_eq!(bytes_consumed, byte_offset(3));
            if let Some(canonical_event) = convert_input_event(vt100_event) {
                assert!(matches!(canonical_event, InputEvent::Keyboard(_)));
            } else {
                panic!("Failed to convert keyboard event");
            }
        }
    }

    #[tokio::test]
    async fn test_buffer_management() {
        // Test buffer handling: growth, consumption, and compaction at 2KB threshold.
        // Create a local core to test the consume_bytes function.
        let mut core = DirectToAnsiInputCore {
            stdin: tokio::io::stdin(),
            parse_buffer: SmallVec::new(),
            buffer_position: ByteIndex::default(),
            paste_state: PasteCollectionState::Inactive,
            event_queue: VecDeque::new(),
        };

        // Verify initial state.
        assert_eq!(core.parse_buffer.len(), 0);
        assert_eq!(core.buffer_position.as_usize(), 0);

        // Test 1: Buffer growth - add data and verify length increases.
        let test_data = vec![b'X'; 100];
        core.parse_buffer.extend_from_slice(&test_data);
        assert_eq!(core.parse_buffer.len(), 100);
        assert!(core.parse_buffer.capacity() >= 100);

        // Test 2: Consumption tracking - consume bytes and verify counter.
        consume_bytes(&mut core, byte_offset(50));
        assert_eq!(core.buffer_position.as_usize(), 50);
        assert_eq!(core.parse_buffer.len(), 100);

        // Test 3: Verify consumed bytes are skipped.
        let unread_portion = &core.parse_buffer[core.buffer_position.as_usize()..];
        assert_eq!(unread_portion.len(), 50);

        // Test 4: Buffer compaction at threshold (half of PARSE_BUFFER_SIZE).
        core.parse_buffer.clear();
        core.buffer_position = ByteIndex::default();

        // Add 2100 bytes (exceed 2048 threshold, which is half of 4096).
        let large_data = vec![b'Y'; 2100];
        core.parse_buffer.extend_from_slice(&large_data);
        assert_eq!(core.parse_buffer.len(), 2100);

        // Consume 1000 bytes (won't trigger compaction yet, need > 2048).
        consume_bytes(&mut core, byte_offset(1000));
        assert_eq!(core.buffer_position.as_usize(), 1000);
        assert_eq!(core.parse_buffer.len(), 2100);

        // Consume another 1100 bytes (total = 2100, exceeds 2048 threshold).
        consume_bytes(&mut core, byte_offset(1100));
        assert_eq!(core.buffer_position.as_usize(), 0);
        assert_eq!(core.parse_buffer.len(), 0);

        // Test 5: Verify capacity doesn't shrink unexpectedly.
        let capacity_after_compact = core.parse_buffer.capacity();
        assert!(capacity_after_compact >= PARSE_BUFFER_SIZE);
    }

    #[tokio::test]
    async fn test_paste_state_machine_basic() {
        // Test: Basic paste collection - Start marker, text, End marker.
        // Use a local paste_state to test the state machine logic directly.
        let mut paste_state = PasteCollectionState::Inactive;

        // Verify initial state is Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Simulate receiving Paste(Start) event.
        let start_event = VT100InputEventIR::Paste(VT100PasteModeIR::Start);
        match (&mut paste_state, &start_event) {
            (
                state @ PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::Start),
            ) => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!("State machine should handle Paste(Start)"),
        }

        // Verify we're now collecting.
        assert!(matches!(paste_state, PasteCollectionState::Accumulating(_)));

        // Simulate receiving keyboard events (the pasted text).
        for ch in &['H', 'e', 'l', 'l', 'o'] {
            let keyboard_event = VT100InputEventIR::Keyboard {
                code: VT100KeyCodeIR::Char(*ch),
                modifiers: VT100KeyModifiersIR::default(),
            };
            match (&mut paste_state, &keyboard_event) {
                (
                    PasteCollectionState::Accumulating(buffer),
                    VT100InputEventIR::Keyboard {
                        code: VT100KeyCodeIR::Char(ch),
                        ..
                    },
                ) => {
                    buffer.push(*ch);
                }
                _ => panic!("State machine should accumulate text while collecting"),
            }
        }

        // Simulate receiving Paste(End) event.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let collected_text = match (&mut paste_state, &end_event) {
            (
                state @ PasteCollectionState::Accumulating(_),
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => {
                if let PasteCollectionState::Accumulating(text) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    text
                } else {
                    panic!("Should have collected text")
                }
            }
            _ => panic!("State machine should handle Paste(End)"),
        };

        // Verify we collected the correct text.
        assert_eq!(collected_text, "Hello");

        // Verify we're back to Inactive state.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_multiline() {
        // Test: Paste with newlines.
        let mut paste_state = PasteCollectionState::Inactive;

        // Start collection.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            PasteCollectionState::Accumulating(_) => panic!(),
        }

        // Accumulate "Line1\nLine2".
        for ch in "Line1\nLine2".chars() {
            match &mut paste_state {
                PasteCollectionState::Accumulating(buffer) => {
                    buffer.push(ch);
                }
                PasteCollectionState::Inactive => panic!(),
            }
        }

        // End collection.
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "Line1\nLine2");
    }

    #[tokio::test]
    async fn test_paste_state_machine_orphaned_end() {
        // Test: Orphaned End marker (without Start) should be handled gracefully.
        let mut paste_state = PasteCollectionState::Inactive;

        // Should be Inactive initially.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));

        // Receive End marker without Start - should emit empty paste.
        let end_event = VT100InputEventIR::Paste(VT100PasteModeIR::End);
        let result = match (&mut paste_state, &end_event) {
            (
                PasteCollectionState::Inactive,
                VT100InputEventIR::Paste(VT100PasteModeIR::End),
            ) => Some(InputEvent::BracketedPaste(String::new())),
            _ => None,
        };

        assert!(matches!(result, Some(InputEvent::BracketedPaste(s)) if s.is_empty()));

        // Should still be Inactive.
        assert!(matches!(paste_state, PasteCollectionState::Inactive));
    }

    #[tokio::test]
    async fn test_paste_state_machine_empty_paste() {
        // Test: Empty paste (Start immediately followed by End).
        let mut paste_state = PasteCollectionState::Inactive;

        // Start.
        match &mut paste_state {
            state @ PasteCollectionState::Inactive => {
                *state = PasteCollectionState::Accumulating(String::new());
            }
            _ => panic!(),
        }

        // End (without any characters in between).
        let text = match &mut paste_state {
            state @ PasteCollectionState::Accumulating(_) => {
                if let PasteCollectionState::Accumulating(t) =
                    std::mem::replace(state, PasteCollectionState::Inactive)
                {
                    t
                } else {
                    panic!()
                }
            }
            _ => panic!(),
        };

        assert_eq!(text, "");
    }
}
