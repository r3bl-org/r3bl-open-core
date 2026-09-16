// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module exports [`MaybeMore`] for terminal input stream availability
//! heuristics.

/// Describes the availability of future input bytes following the current byte.
///
/// Centralizes the heuristics used across the I/O reader thread, stateful accumulator,
/// protocol router, and terminal event disambiguation logic.
///
/// # Escape Sequence Disambiguation Context
///
/// In asynchronous terminal byte streams, user keypresses and terminal control sequences
/// share identical byte prefixes. Because R3BL intentionally avoids fixed timer delays
/// (such as Vim's 25-100ms `ttimeoutlen`), the parser must disambiguate these ambiguous
/// prefixes using stream availability heuristics, lexical grammar validation, or enhanced
/// keyboard protocols.
///
/// There are three canonical prefix collisions:
///
/// 1. **Standalone [`ESC`] Key (`0x1B`) vs. Multi-Byte Sequences**:
///    - *The Collision*: A physical [`ESC`] key press emits the single byte `0x1B`. Every
///      multi-byte escape sequence (such as Up Arrow `ESC [ A`) also begins with `0x1B`.
///    - *How Solved*: The [`MaybeMore`] stream availability heuristic. If the OS read
///      buffer has drained (`bytes_read < buffer_size`), a lone `0x1B` is emitted
///      immediately as an [`ESC`] keystroke with 0ms latency. If the buffer was filled,
///      the parser waits for subsequent bytes.
///
/// 2. **Standalone `Alt+]` Key (`ESC ]`, `0x1B 0x5D`) vs. [`OSC`] Responses**:
///    - *The Collision*: The keystroke `Alt+]` emits `ESC ]`. Operating System Command
///      ([`OSC`]) responses written by the terminal (such as color queries `ESC ] 11 ;
///      rgb:... BEL`) also begin with `ESC ]`.
///    - *How Solved*: Strict grammar validation combined with stream availability.
///      [`OSC`] grammar requires decimal command digits followed by a delimiter (`;` or
///      `?`). Non-digits immediately identify human input (`Alt+]`). Incomplete digits on
///      a drained stream emit `Alt+]` while preserving trailing digits. Delimited
///      payloads wait across reads for the terminator.
///
/// 3. **Standalone `Alt+[` Key (`ESC [`, `0x1B 0x5B`) vs. [`CSI`] Sequences**:
///    - *The Collision*: In legacy [`VT-100`] / [`xterm`], `Alt+[` emits `ESC [`. This is
///      the Control Sequence Introducer ([`CSI`]) prefix used by all arrow keys, function
///      keys, mouse tracking, and bracketed paste.
///    - *Why Unsolvable in Legacy Mode*: [`CSI`] grammar is wide-open: almost any
///      printable character (`0x20..=0x7E`) can follow `ESC [`. Distinguishing bare
///      `Alt+[` from an incomplete [`CSI`] prefix without a timer is mathematically
///      impossible, and treating drained `ESC [` as `Alt+[` breaks arrow keys over
///      jittery network connections.
///    - *Modern Resolution*: Enhanced keyboard protocols like the [[`Kitty`] Keyboard
///      Protocol] and `CSI u`, which encode `Alt+[` unambiguously as `ESC [ 91 ; 3 u`.
///
/// ## The Disambiguation Matrix
///
/// - **`0x1B` (`[ESC]`)**:
///   - _User keypress:_ Physical [`ESC`] key.
///   - _Terminal / Protocol Sequence:_ Arrow keys, F-keys, mouse, terminal replies.
///   - _Grammar Properties:_ None - single byte has no syntax.
///   - _Disambiguation Strategy:_ Stream Availability ([`MaybeMore`]) - drained emits
///     _immediately (0ms); full waits.
///   - _Status:_ Solved.
/// - **`0x1B 0x5D` (`[ESC, '\]']`)**:
///   - _User keypress:_ `Alt+]`.
///   - _Terminal / Protocol Sequence:_ [`OSC`] responses (`ESC ] 11 ; rgb:... BEL`).
///   - _Grammar Properties:_ Strict - must follow `ESC ] <digits> [;?] <payload>`.
///   - _Disambiguation Strategy:_ Grammar Validation + Stream Status - reject non-digits
///     as `Alt+]`; incomplete digits on drained stream emit `Alt+]`.
///   - _Status:_ Solved.
/// - **`0x1B 0x5B` (`[ESC, '[']`)**:
///   - _User keypress:_ `Alt+[`.
///   - _Terminal / Protocol Sequence:_ [`CSI`] sequences (`ESC [ A`, `ESC [ 1 ; 2 H`).
///   - _Grammar Properties:_ Wide-Open - any [`ASCII`] byte can follow `[`.
///   - _Disambiguation Strategy:_ Protocol Negotiation - unambiguous via [[`Kitty`]
///     Keyboard Protocol] (`\x1b[91;3u`); legacy [`VT-100`] unsupported.
///   - _Status:_ Documented Limitation.
///
/// # The Two Levels of Stream Availability
///
/// [`MaybeMore`] distinguishes between two levels of stream availability:
///
/// 1. **Deterministic User-Space Boundaries**: Any byte before the final index of a read
///    buffer slice is guaranteed to have subsequent bytes already loaded in memory.
///    Evaluated per-byte via [`Self::refine_for_byte_index()`].
/// 2. **Probabilistic OS Kernel Queue Availability**: On the final byte of a read buffer
///    slice, availability depends on whether the underlying OS `read()` syscall
///    completely filled the read buffer. Evaluated at the I/O boundary via
///    [`Self::from_read_count()`].
///
/// # Zero-Latency Heuristic vs. Fixed Timer Delays
///
/// Traditional terminal libraries (such as `ncurses` or Vim via `ttimeoutlen`) use a
/// fixed 25-100ms timer to disambiguate a lone [`ESC`] key press from an incoming escape
/// sequence. While robust, this introduces perceptible latency to every physical [`ESC`]
/// keystroke.
///
/// [`MaybeMore`] adopts a zero-latency heuristic based on stream availability:
/// - **Zero-Latency Local Keystrokes**: Terminal emulators write multi-byte escape
///   sequences atomically in a single `write()` syscall. In the common case, the entire
///   sequence arrives in a single `read()`, allowing a lone [`ESC`] to be emitted with
///   0ms latency whenever the read buffer is drained.
/// - **High-Volume Clipboard Pasting**: Multi-kilobyte paste operations saturate the read
///   buffer (`bytes_read == buffer_size`), preserving [`MaybeMore::KernelMayHaveMore`]
///   across read boundaries and preventing spurious [`ESC`] emission mid-stream.
/// - **SSH & Network Segmentation Trade-off**: Over high-latency or jittery [`SSH`]
///   connections, [`TCP`] packet fragmentation may deliver a lone [`ESC`] byte in a
///   packet smaller than the read buffer. The heuristic assumes the stream is drained and
///   emits [`ESC`] immediately, causing trailing bytes (e.g. `[ A`) to be parsed as
///   literal characters. This faster [`ESC`] response represents an explicit design
///   trade-off.
///
/// # Packet Fragmentation & Buffer Boundary Scenarios
///
/// Across asynchronous I/O and network boundaries, multi-byte escape sequences can be
/// split across read operations:
///
/// ## Scenario 1: Split After Sequence Prefix (Successful Reassembly)
///
/// When packet fragmentation occurs after `ESC [` or `ESC ]`, the parser recognizes an
/// incomplete sequence and preserves the accumulator across read boundaries:
///
/// ```text
/// Packet 1: [ESC, '[']       read() → 2 bytes, more = Drained
///                            CSI is incomplete → router returns None
///                            Accumulator retains: [ESC, '[']
/// Packet 2: ['A']            read() → 1 byte
///                            Accumulator pushes 'A': [ESC, '[', 'A']
///                            Parsed as: Up Arrow ✓
/// ```
///
/// ## Scenario 2: Multi-KB Paste Across Buffer Boundaries (Successful Reassembly)
///
/// During high-volume paste operations that saturate the 1024-byte read buffer, an escape
/// sequence split across chunk boundaries is preserved by [`Self::KernelMayHaveMore`]:
///
/// ```text
/// Read 1:   [..., ESC]       read() → 1024 bytes (full buffer) → KernelMayHaveMore
///                            is_more_anticipated() == true → router returns None
///                            Accumulator retains: [ESC]
/// Read 2:   ['[', 'A', ...]  read() → next chunk
///                            Accumulator pushes '[' and 'A'
///                            Parsed as: Up Arrow ✓
/// ```
///
/// ## Scenario 3: Split Immediately After Lone [`ESC`] (The Zero-Latency Trade-off)
///
/// If [`TCP`] packet fragmentation delivers a lone [`ESC`] byte in a packet smaller than
/// the buffer size, the stream appears drained:
///
/// ```text
/// Packet 1: [ESC]            read() → 1 byte, more = Drained
///                            Router emits: standalone ESC key immediately (0ms latency)
/// Packet 2: ['[', 'A']       read() → 2 bytes
///                            Accumulator was cleared; parses '[' and 'A' as literal characters
/// ```
///
/// This is the explicit trade-off of a timer-less heuristic: physical [`ESC`] keystrokes
/// have 0ms latency, but an [`ESC`] arriving isolated over a jittery network cannot be
/// distinguished from a human pressing the [`ESC`] key.
///
/// # Pipeline Flow
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ handler_stdin.rs (OS read boundary)                              │
/// │ MaybeMore::from_read_count(bytes_read, STDIN_READ_BUFFER_SIZE)   │
/// └───────────────────────────────┬──────────────────────────────────┘
///                                 │
///                                 ▼
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ stateful_parser.rs (per-byte loop)                               │
/// │ • if idx + 1 < read_buffer.len() => RemainingInReadBuffer        │
/// │ • else                           => kernel status from read      │
/// └───────────────────────────────┬──────────────────────────────────┘
///                                 │
///                                 ▼
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ router.rs & terminal_events.rs (disambiguation decisions)        │
/// │ • if maybe_more.is_more_anticipated() => wait                    │
/// │ • else => emit keystroke immediately (0ms latency)               │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// [`PTY`]: <https://en.wikipedia.org/wiki/Pseudoterminal>
/// [`SSH`]: <https://en.wikipedia.org/wiki/Secure_Shell>
/// [`TCP`]: <https://en.wikipedia.org/wiki/Transmission_Control_Protocol>
/// [`VT-100`]: <https://vt100.net/docs/vt100-ug/chapter3.html>
/// [Kitty Keyboard Protocol]: <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
/// [`StatefulInputParser`]:
///     crate::terminal_lib_backends::direct_to_ansi::input::stateful_parser::StatefulInputParser
/// [`stdin`]: std::io::stdin
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum MaybeMore {
    /// Subsequent bytes have already been read from `stdin` and are present in the
    /// current user-space read buffer.
    ///
    /// This is deterministic: no further I/O or syscalls are needed to process these
    /// bytes.
    RemainingInReadBuffer,

    /// All bytes in the current read buffer have been processed, but the OS read buffer
    /// was completely filled (`bytes_read == buffer_size`).
    ///
    /// This is a heuristic: more bytes may be in flight or queued in the OS kernel
    /// [`PTY`] buffer waiting for the next read syscall.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    KernelMayHaveMore,

    /// All bytes in the current read buffer have been processed, and the OS read was
    /// smaller than the read buffer size.
    ///
    /// This is deterministic: both the user-space read buffer and the OS kernel buffer
    /// have been completely drained.
    #[default]
    Drained,
}

impl MaybeMore {
    /// Determines kernel input availability based on whether the OS `read()` syscall
    /// completely filled the read buffer.
    ///
    /// - If `bytes_read == buffer_size`: returns [`MaybeMore::KernelMayHaveMore`].
    /// - If `bytes_read < buffer_size`: returns [`MaybeMore::Drained`].
    #[must_use]
    pub const fn from_read_count(bytes_read: usize, buffer_size: usize) -> Self {
        if bytes_read == buffer_size {
            Self::KernelMayHaveMore
        } else {
            Self::Drained
        }
    }

    /// Returns `true` if more bytes are either definitely in the user-space read buffer
    /// or potentially pending in the OS kernel buffer.
    #[must_use]
    pub const fn is_more_anticipated(&self) -> bool {
        match self {
            Self::RemainingInReadBuffer | Self::KernelMayHaveMore => true,
            Self::Drained => false,
        }
    }

    /// Refines the kernel-level stream availability heuristic for a specific byte index
    /// within a user-space read buffer.
    ///
    /// - If more bytes remain in the slice (`idx + 1 < buffer_len`), they are guaranteed
    ///   to already be present in user space: returns [`Self::RemainingInReadBuffer`].
    /// - On the final byte of the slice (`idx + 1 >= buffer_len`), falls back to `self`
    ///   (the kernel availability status).
    #[must_use]
    pub const fn refine_for_byte_index(self, idx: usize, buffer_len: usize) -> Self {
        if idx + 1 < buffer_len {
            Self::RemainingInReadBuffer
        } else {
            self
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_read_count() {
        const BUFFER_SIZE: usize = 1024;
        assert_eq!(
            MaybeMore::from_read_count(BUFFER_SIZE, BUFFER_SIZE),
            MaybeMore::KernelMayHaveMore
        );
        assert_eq!(
            MaybeMore::from_read_count(100, BUFFER_SIZE),
            MaybeMore::Drained
        );
        assert_eq!(
            MaybeMore::from_read_count(0, BUFFER_SIZE),
            MaybeMore::Drained
        );
    }

    #[test]
    fn test_is_more_anticipated() {
        assert!(MaybeMore::RemainingInReadBuffer.is_more_anticipated());
        assert!(MaybeMore::KernelMayHaveMore.is_more_anticipated());
        assert!(!MaybeMore::Drained.is_more_anticipated());
    }

    #[test]
    fn test_refine_for_byte_index() {
        // Multi-byte slice: bytes before the final index return RemainingInReadBuffer.
        assert_eq!(
            MaybeMore::Drained.refine_for_byte_index(0, 3),
            MaybeMore::RemainingInReadBuffer
        );
        assert_eq!(
            MaybeMore::KernelMayHaveMore.refine_for_byte_index(1, 3),
            MaybeMore::RemainingInReadBuffer
        );

        // Final byte index falls back to the underlying self status.
        assert_eq!(
            MaybeMore::Drained.refine_for_byte_index(2, 3),
            MaybeMore::Drained
        );
        assert_eq!(
            MaybeMore::KernelMayHaveMore.refine_for_byte_index(2, 3),
            MaybeMore::KernelMayHaveMore
        );

        // Single-byte slice (idx 0 of len 1) falls back to self.
        assert_eq!(
            MaybeMore::Drained.refine_for_byte_index(0, 1),
            MaybeMore::Drained
        );
        assert_eq!(
            MaybeMore::KernelMayHaveMore.refine_for_byte_index(0, 1),
            MaybeMore::KernelMayHaveMore
        );
    }

    #[test]
    fn test_default() {
        assert_eq!(MaybeMore::default(), MaybeMore::Drained);
    }
}

// cspell:words ttimeoutlen
