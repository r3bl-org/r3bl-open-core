// Copyright (c) 2025 R3BL LLC. Licensed under Apache License, Version 2.0.

//! This module exports [`MaybeMore`] for terminal input stream availability heuristics.

/// A stream availability hint that tells the input parser whether more bytes are expected
/// from the operating system.
///
/// This hint enables **0ms zero-latency [`ESC`] key handling** without introducing fixed
/// delay timers.
///
/// When the I/O reader thread ([`MioPollWorker`]) reads a chunk from standard input, it
/// evaluates whether the kernel's queue was completely drained or if our userspace read
/// buffer was filled to capacity. It passes this [`MaybeMore`] status to
/// [`InputByteStreamToIrParser`], which forwards it to [`try_parse_input_event()`]. This
/// allows the parser to immediately distinguish between a standalone physical [`ESC`]
/// keystroke and the start of a multi-byte escape sequence (such as an arrow key `ESC [
/// A`).
///
/// # Why We Need Disambiguation
///
/// In terminal input, keystrokes and escape sequences share identical prefix bytes. For
/// example, both begin with `1B` in hex:
/// - **Physical [`ESC`] key**: sends `1B` (1 byte).
/// - **Up Arrow key (`ESC [ A`)**: sends `1B 5B 41` (3 bytes).
///
/// Many terminal apps (such as Vim via [`ttimeoutlen`]) pause and wait 25-100ms on a
/// timer to see if more bytes arrive before deciding what was pressed. Because we
/// intentionally avoid timers to keep keystrokes instant (0ms latency), [`MaybeMore`]
/// provides stream availability heuristics to disambiguate input without delays.
///
/// For the complete parser-wide collision architecture (including `Alt+[` and `Alt+]`),
/// see the [Escape Sequence Disambiguation] section in the [`mod@super`] module
/// documentation.
///
/// Within this parser, [`MaybeMore`] participates in two specific disambiguation
/// scenarios:
///
/// 1. **Standalone [`ESC`] Key (`1B` in hex) vs. Multi-Byte Sequences (Primary Role)**:
///    - *Collision*: A physical [`ESC`] keypress emits the single byte `1B` in hex. Every
///      multi-byte escape sequence (such as Up Arrow `ESC [ A`) also begins with `1B` in
///      hex.
///    - *Resolution*: Handled directly by stream availability via [`MaybeMore`]:
///      - **[`Self::KernelDrained`]** (`bytes_read < buffer_size`): The kernel buffer was
///        completely drained, so a lone `1B` in hex is emitted immediately as an [`ESC`]
///        keystroke with 0ms latency.
///      - **[`Self::KernelMayHaveMore`]** (`bytes_read == buffer_size`): The userspace
///        buffer was filled to capacity. More bytes may be pending in the kernel, so the
///        parser waits.
///
/// 2. **Standalone `Alt+]` Key (`ESC ]`, `1B 5D` in hex) vs. [`OSC`] Responses
///    (Collaborative Role)**:
///    - *Collision*: The keystroke `Alt+]` emits `ESC ]`. Operating System Command
///      ([`OSC`]) responses written by the terminal (such as color queries `ESC ] 11 ;
///      rgb:... BEL`) also begin with `ESC ]`.
///    - *Resolution*: Handled in [`try_disambiguate_osc_or_alt_bracket()`] using a
///      combination of [`OSC` spec] conformance and [`MaybeMore`]. If command digits
///      arrive but the stream is [`Self::KernelDrained`] before the delimiter, the parser
///      recognizes that human typing occurred and emits `Alt+]`.
///
/// # Zero-Latency Heuristic vs. Fixed Timer Delays
///
/// Traditional terminal libraries (such as [`ncurses`] or Vim via [`ttimeoutlen`]) use a
/// fixed 25-100ms timer to disambiguate a lone [`ESC`] keypress from an incoming escape
/// sequence. While robust, this introduces perceptible latency to every physical [`ESC`]
/// keystroke.
///
/// [`MaybeMore`] adopts a zero-latency heuristic based on stream availability:
/// - **Zero-Latency Local Keystrokes**: Terminal emulators write multi-byte escape
///   sequences atomically in a single [`write()`] syscall. In the common case, the entire
///   sequence arrives in a single [`read()`]. When our userspace read buffer is not full
///   (`bytes_read < buffer_size`), the kernel queue was drained, allowing a lone [`ESC`]
///   to be emitted with 0ms latency.
/// - **High-Volume Clipboard Pasting**: Multi-kilobyte paste operations saturate the
///   userspace read buffer (`bytes_read == buffer_size`), preserving
///   [`Self::KernelMayHaveMore`] across chunk boundaries and preventing spurious [`ESC`]
///   emissions mid-paste.
/// - **Packet Fragmentation & Network Jitter**: Over high-latency or jittery [`SSH`]
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
/// Packet 1: [ESC, '[']       read() → 2 bytes, more = KernelDrained
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
///                            KernelMayHaveMore → router returns None
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
/// Packet 1: [ESC]            read() → 1 byte, more = KernelDrained
///                            Router emits: standalone ESC key immediately (0ms latency)
/// Packet 2: ['[', 'A']       read() → 2 bytes
///                            Accumulator was cleared; parses '[' and 'A' as literal characters
/// ```
///
/// # Pipeline Flow
///
/// ```text
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ handler_stdin.rs (I/O Worker Thread)                             │
/// │ Reads up to STDIN_READ_BUFFER_SIZE (1,024 bytes) from stdin      │
/// │ Computes MaybeMore::from_read_count(bytes_read, 1024)            │
/// └───────────────────────────────┬──────────────────────────────────┘
///                                 │
///                                 ▼
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ input_byte_stream_to_ir (InputByteStreamToIrParser::advance)     │
/// │ Appends read chunk to self.accumulator                           │
/// │ Forwards (&self.accumulator, maybe_more) to the router           │
/// └───────────────────────────────┬──────────────────────────────────┘
///                                 │
///                                 ▼
/// ┌──────────────────────────────────────────────────────────────────┐
/// │ router.rs (try_parse_input_event)                                │
/// │ • If lone ESC and KernelMayHaveMore => wait (returns None)       │
/// │ • If lone ESC and KernelDrained => emit ESC immediately (0ms)    │
/// │ • Complete multi-byte sequences parsed directly                  │
/// └──────────────────────────────────────────────────────────────────┘
/// ```
///
/// [`ASCII`]: https://en.wikipedia.org/wiki/ASCII
/// [`CSI` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#CSI
/// [`CSI`]: crate::CsiSequence
/// [`ESC`]: crate::EscSequence
/// [`InputByteStreamToIrParser`]:
///     super::input_byte_stream_to_ir::InputByteStreamToIrParser
/// [`Kitty`]: https://sw.kovidgoyal.net/kitty/
/// [`MioPollWorker`]:
///     crate::terminal_lib_backends::direct_to_ansi::input::mio_poller::MioPollWorker
/// [`mod@super`]: super
/// [`ncurses`]: https://en.wikipedia.org/wiki/Ncurses
/// [`OSC` spec]: https://en.wikipedia.org/wiki/ANSI_escape_code#OSC
/// [`OSC`]: crate::osc_codes::OscSequence
/// [`PTY`]: <https://en.wikipedia.org/wiki/Pseudoterminal>
/// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
/// [`SSH`]: <https://en.wikipedia.org/wiki/Secure_Shell>
/// [`stdin`]: std::io::stdin
/// [`TCP`]: <https://en.wikipedia.org/wiki/Transmission_Control_Protocol>
/// [`try_disambiguate_osc_or_alt_bracket()`]:
///     super::terminal_events::try_disambiguate_osc_or_alt_bracket
/// [`try_parse_input_event()`]: super::try_parse_input_event
/// [`ttimeoutlen`]:
///     https://vi.stackexchange.com/questions/24925/usage-of-timeoutlen-and-ttimeoutlen
/// [`VT-100`]: https://vt100.net/docs/vt100-ug/chapter3.html
/// [`write()`]: https://man7.org/linux/man-pages/man2/write.2.html
/// [`xterm`]: https://en.wikipedia.org/wiki/Xterm
/// [Escape Sequence Disambiguation]: mod@super#escape-sequence-disambiguation
/// [Kitty Keyboard Protocol]: <https://sw.kovidgoyal.net/kitty/keyboard-protocol/>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub enum MaybeMore {
    /// The userspace read buffer was filled to capacity (`bytes_read == buffer_size`).
    ///
    /// This is a heuristic: because our buffer was filled, more bytes may still be in
    /// flight or queued in the OS kernel [`PTY`] buffer waiting for the next read
    /// syscall.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    KernelMayHaveMore,

    /// The userspace read buffer was not full (`bytes_read < buffer_size`).
    ///
    /// Because the OS [`read()`] returned fewer bytes than requested, the kernel's
    /// [`PTY`] buffer was completely drained.
    ///
    /// [`PTY`]: https://en.wikipedia.org/wiki/Pseudoterminal
    /// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
    #[default]
    KernelDrained,
}

impl MaybeMore {
    /// Determines kernel input availability based on whether the OS [`read()`] syscall
    /// completely filled the read buffer.
    ///
    /// - If `bytes_read == buffer_size`: returns [`MaybeMore::KernelMayHaveMore`].
    /// - If `bytes_read < buffer_size`: returns [`MaybeMore::KernelDrained`].
    ///
    /// [`read()`]: https://man7.org/linux/man-pages/man2/read.2.html
    #[must_use]
    pub const fn from_read_count(bytes_read: usize, buffer_size: usize) -> Self {
        if bytes_read == buffer_size {
            Self::KernelMayHaveMore
        } else {
            Self::KernelDrained
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
            MaybeMore::KernelDrained
        );
        assert_eq!(
            MaybeMore::from_read_count(0, BUFFER_SIZE),
            MaybeMore::KernelDrained
        );
    }

    #[test]
    fn test_default() {
        assert_eq!(MaybeMore::default(), MaybeMore::KernelDrained);
    }
}

// cspell:words ttimeoutlen
