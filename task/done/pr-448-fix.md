// cspell:words cecton Buttonless URXVT trackpoint

# Task: PR 448 Integration & Fixes

## Overview

We are integrating the core fixes from PR
[#448](https://github.com/r3bl-org/r3bl-open-core/pull/448) (`@cecton`) related to the SGR
mouse event parsing pipeline. Because `main` has evolved and the original PR had some
critical gaps, we are cherry-picking the necessary fixes, augmenting them to fix legacy
heavily refactored commit with proper author attribution.

## Execution Workflow

We will process each of the 9 action items iteratively using the following loop:

1. **Implementation:** Write the specific code changes for the current heading.
2. **Local Testing:** Run `./check.fish --check` and, where applicable, test functionality
   using `cargo run --example mouse_inspector`.
3. **Mandatory Manual Review:** You (the user) will manually review the specifically
   touched files before the heading is marked as checked `[x]`.

_(Once all 9 headings are successfully implemented and checked off, we will proceed to
final verification and cleanup.)_

### Core Fixes from PR #448 (To Keep)

#### [x] Fix "Hover" Event Crashes (Buttonless Motion Parsing)

Prevent the parser from failing when the user hovers the mouse without clicking.
Currently, moving the mouse without holding a button generates an "Unknown" button code
that aborts the parse. Update `detect_mouse_button` to safely return `Unknown`, which will
cleanly map to a `MouseMove` (hover) event instead of crashing.

- _Context:_ When moving the mouse without clicking, terminals send a "motion" event (flag
  `32`) with no button code. The terminal packs mouse information into a single byte using
  bitwise flags. If the terminal sends exactly `32`, it means the mouse is moving but no
  buttons or modifiers are pressed. Here are the mappings:
  - Bits 0-1 define the button (0=Left, 1=Middle, 2=Right, 3=Release),
  - Bit 2 (4) is Shift,
  - Bit 3 (8) is Alt,
  - Bit 4 (16) is Ctrl, and
  - Bit 5 (32) is the Motion flag.
- _Hover vs Scroll:_ Hovering (moving the physical mouse without clicking) generates a
  `MouseMove` event. This is completely distinct from moving the scroll wheel, which the
  terminal actually fakes as a "Button 4" or "Button 5" click and generates entirely
  separate `Scroll` events.
- _The 5 Fundamental Terminal Mouse Events:_
  1. **Press/Click**: Pushing a button down.
  2. **Release**: Letting a button go.
  3. **Drag**: Moving the mouse while holding a button.
  4. **Motion/Hover**: Moving the mouse without holding any buttons.
  5. **Scroll**: Moving the scroll wheel.
- _Deep Dive (Life of a Motion Event):_
  1. **Full TUI Setup & Terminal Awareness:** The user opens a terminal emulator app (eg:
     Wezterm), and runs a full-TUI app. The app boots via
     `crate::tui::TerminalWindow::main_event_loop()`. The `r3bl_tui` framework
     automatically puts the terminal in Raw Mode, spins up the Resilient Reactor Thread
     (RRT) for `mio`, and emits ANSI sequences like `\x1b[?1003h` (Enable Any-Event Mouse
     Tracking) to `stdout`, which tells the terminal emulator app that we want hover
     coordinates sent back via `stdin`.
  2. **Physical Action:** A user moves their mouse or touchpad or trackball or trackpoint,
     specifically "hover-moving" over the terminal emulator window running our full TUI
     app.
  3. **OS Routing:** The OS (via Wayland) determines the terminal emulator window has
     focus and fires a UI event (using whatever UI toolkit the emulator is written in).
  4. **ANSI Serialization:** The _buttons and modifiers_ are packed into a single byte
     payload using a bitwise OR mask. The terminal emulator app then takes this payload
     byte, along with the X/Y coordinates, and converts them all into a human-readable
     ASCII text string (e.g., `ESC[ < 35 ; 12 ; 24M`). Here's the binary math for how the
     `35` payload byte only is calculated, using big endian notation (most significant bit
     first), and using 0-indexed bit positions (meaning Bit 5 is the 6th bit from the
     right, scanning right to left):
     ```text
     `76543210` - Bit positions
     `00100000` (Decimal `32`) : Motion Flag (Bit 5 is set)
     `00000011` (Decimal `3`) : Unknown Button (Bits 0 and 1 are set)
     `--------`
     `00100011` (Decimal `35`) : Final payload byte (`32 | 3`)
     ```
  5. **Process Delivery:** The terminal emulator app writes this string to the `stdin` of
     our TUI app's process.
  6. **Event Loop:** Our asynchronous `mio` event loop (running inside
     `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/mio_poller/mio_poll_worker.rs`)
     reads these bytes and routes them into `mouse.rs`.
  7. **Parsing:** The parser identifies it as an SGR 1006 mouse event and safely unpacks
     the `35` payload byte into an `Unknown` button with a motion flag.
  8. **Type-Safe Delivery:** It delivers a clean, type-safe
     `InputEvent::Mouse(MouseMove { x: 12, y: 24 })` through the framework directly into
     the developer's `App::app_handle_input_event()` implementation!
- _File Touched:_ `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs`
- _The Fix:_ Updated the action detection logic in all three parsers (`SGR`, `X10`, and
  `RXVT`) to correctly differentiate between `Motion` (hover) and `Drag` events using the
  motion bit (`Bit 5`) _and_ the parsed button code.
  - **SGR Fix**: Updated the `SGR` parser to check the button: if it is `Unknown`, emit
    `Motion`; otherwise, emit `Drag`.
  - **X10 & RXVT Fix**: Updated them to check the button bits: if it is `Unknown` (code
    3), emit `Motion`; if it is a valid button (0, 1, 2), emit `Drag` with the respective
    button.

#### [x] xterm Compatibility (`URXVT`/`SGR`)

Swaps the mode negotiation output order so that `URXVT (1015)` is emitted _before_
`SGR (1006)`.

- _Context:_ Some terminals (like `xterm`) use a "last writer wins" strategy for mouse
  modes. If the older `URXVT` mode was enabled after the modern `SGR` mode, `xterm` would
  use `URXVT`, breaking mouse parsing. Flipping the order ensures `SGR` dominates.
- _Question:_ I'm confused about what the correct emission of sequences should be. Why do
  we emit both? Is it for backward compatibility or VT-100 spec compliance?
- _Answer:_ Terminals can support multiple mouse tracking standards (`X10`, `URXVT`,
  `SGR`). When our app starts, it tells the terminal "turn on mouse tracking" by emitting
  a sequence of enable codes. `SGR` (`1006`) is the modern, robust standard. `URXVT`
  (`1015`) is older and buggier. If we emit `Enable SGR` and then `Enable URXVT`, `xterm`
  says "Okay, you asked for `URXVT` last, so I'll use that." This breaks our mouse
  support. By flipping the order so we emit `Enable URXVT` and _then_ `Enable SGR`,
  `xterm` locks onto the modern `SGR` standard. We emit both because older terminal
  emulators might only support the older standard, so we cast a wide net for maximum
  compatibility.
- _File Touched:_ `tui/src/core/ansi/generator/ansi_sequence_generator_output.rs`
- _The Fix:_ Swapped the emission order in `enable_mouse_tracking` so that
  `APPLICATION_MOUSE_TRACKING (1003)` and `URXVT (1015)` are emitted before `SGR (1006)`.
  Also mirrored this in `disable_mouse_tracking` by disabling in the exact reverse (LIFO)
  order (`1006`, `1015`, `1003`).

#### [x] Scroll Button Code Fix

Updates `MOUSE_SCROLL_DOWN_BUTTON` from 68 to 65.

- _Context:_ The ANSI standard assigns base code 64 to scroll up and 65 to scroll down.
  Our codebase mistakenly had scroll down defined as 68, which caused down-scroll events
  to not be recognized.
- _Question:_ Are there unit tests we can add here to avoid regressions? Unit tests
  wouldn't have caught this bug since ANSI VT-100 doesn't force changes, right?
- _Answer:_ We absolutely can (and did!) add unit tests for this. While the ANSI standard
  doesn't change, our parser code does. A unit test that explicitly feeds the
  `CSI < 65 ; x ; y M` byte sequence into the parser would have failed when the constant
  was mistakenly `68`. The PR actually adds these exact regression tests.
- _File Touched:_ `tui/src/core/ansi/constants/mouse.rs`
- _The Fix:_ Updated `MOUSE_SCROLL_DOWN_BUTTON` to `65` in `constants/mouse.rs`. Also
  added rigorous unit tests covering SGR scroll up, scroll down, and scroll with modifiers
  to verify that sequence generation matches expected parsing.

#### [x] Modifier Stripping

Adjusts `detect_scroll_event` to appropriately strip `Shift`, `Alt`, and `Ctrl` bits prior
to detecting the scroll direction. Also fixed `MOUSE_BASE_BUTTON_MASK` to correctly
isolate the base scroll button (by omitting the modifier bit positions entirely).

- _Context:_ Terminals encode modifiers by adding bitwise values to the base button code
  (Shift=+4, Alt=+8, Ctrl=+16). If a user held Ctrl while scrolling up, the code was
  `64 + 16 = 80`. By stripping those modifier bits first, we extract the base code `64`
  and correctly identify the scroll.
- _File Touched:_ `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs`
- _The Fix:_ Updated `MOUSE_BASE_BUTTON_MASK` from `0b0111_1111` to `0b0100_0011` to
  cleanly isolate the scroll bit without capturing modifier keys (Shift, Alt, Ctrl). Also
  updated `detect_scroll_event` to explicitly match the clean base codes (64=Up, 65=Down,
  66=Left, 67=Right) rather than using ambiguous ranges.

#### [x] Generator Fixes

Ensures `Unknown` correctly maps to `Release` and sets the `MOUSE_MOTION_FLAG` for
`Motion` events.

- _Context:_ In the output sequence generator (used for simulating terminal input in
  tests), we need to reconstruct exact byte sequences from our internal representations.
  This correctly assigns the standard button release code (`3`) and the motion flag (`32`)
  when simulating buttonless movement.
- _Question:_ Exactly what synthetic events are we generating here, and why? Who uses
  these synthetic events?
- _Answer:_ Yes, exactly! This is for **generation**, not parsing. Our codebase has a
  feature where we can programmatically _generate_ raw ANSI escape sequences from our
  internal `InputEvent` structs. This is heavily used by our test suite to simulate
  terminal input without needing a real physical terminal. To do this, we have to
  serialize a `MouseMove` event back into the exact byte sequence `\x1b[<35;x;yM`.
- _File Touched:_ `tui/src/core/ansi/generator/ansi_sequence_generator_input.rs`
- _The Fix:_ Updated `generate_sgr_sequence` so that `VT100MouseActionIR::Motion`
  correctly adds the `MOUSE_MOTION_FLAG` (bit 5) to the base button code (which is
  `Unknown` mapped to `MOUSE_RELEASE_BUTTON_CODE`). This fixes a bug where `Motion` events
  were generated as bare button presses without the motion modifier flag. (Note:
  `generate_x10_sequence` and `generate_rxvt_sequence` already did this correctly).

#### [ ] Testing

Adds three new regression tests in `mouse.rs`.

- _Context:_ The tests explicitly feed the raw ANSI byte sequences for `Ctrl + Scroll Up`,
  `Alt + Scroll Down`, and `Buttonless Motion` into the parser to ensure no future
  regressions occur.

### Critical Review & Missing Pieces (To Add/Fix)

During code review, we identified several holes in the original PR that we must fix during
integration:

#### [x] Phantom File Claim

The PR claims to fix a silent drop in `protocol_conversion.rs`. Our `main` branch has
already fixed this in a separate commit (it already matches on `action` before `button`).
Drop this from the scope.

- _Question:_ Can we reference which commit in the main branch fixed this?
- _Answer:_ This was fixed in commit `adc32c95` (Major PTY overhaul and ANSI refactoring).
- _File:_ `tui/src/tui/terminal_lib_backends/direct_to_ansi/input/protocol_conversion.rs`
- _The Fix:_ DONE. Verified that `adc32c95` already handles this correctly.

#### [x] Broken X10 & RXVT Parsers

The PR successfully fixes scroll parsing for `SGR`, but ignores `parse_x10_mouse` and
`parse_rxvt_mouse`. Because those parsers blindly mask `cb & 3`, they currently interpret
scroll events (64/65) as Left/Middle clicks! Add `detect_scroll_event` into both of these
legacy parsers.

- _Context:_ The legacy `X10` and `RXVT` parsers determine which button was clicked by
  looking at the bottom 2 bits of the button byte (`cb & 3`). If the result is `0`, it's a
  left click; if `1`, a middle click. However, scrolling up is base code `64`. What
  happens when you do `64 & 3`? The answer is `0`. So the legacy parser accidentally
  thinks a scroll up is a left click! To fix this, check if the byte is a scroll event
  (`cb >= 64`) _before_ we do the `cb & 3` math.
- _Question:_ What is the context of this fix? When does this code execute exactly,
  instead of the SGR code?
- _Answer:_ This code executes when a user runs our app on a legacy terminal that doesn't
  support the modern SGR format. In that scenario, the terminal sends legacy byte
  sequences (`CSI M` instead of `CSI <`), routing execution to `parse_x10_mouse` instead
  of the SGR code.
- _File:_ `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs`
- _The Fix:_ DONE. Added `detect_scroll_event(button_byte)` to the top of `parse_legacy_mouse_event` (which powers both X10 and RXVT), fixing the missing scroll functionality in legacy terminals.

#### [x] Asymmetric Mode Disabling

The PR correctly reordered `enable_mouse_tracking()` to emit `1015` then `1006`. However,
it improperly applied the exact same order to `disable_mouse_tracking()`. Terminal states
should be unwound in LIFO (Last-In, First-Out) order. Fix `disable_mouse_tracking` to emit
in reverse.

- _Context:_ Think of terminal modes like HTML tags. If you open them `<b><i>`, you must
  close them in reverse `</i></b>` to avoid weird states. The PR changed the "enable"
  sequence to emit: `1015`, `1006`, `1000`. But it incorrectly changed the "disable"
  sequence to emit the exact same order: `1015`, `1006`, `1000`. We need to fix the
  disable sequence so it unwinds in reverse (LIFO) order: `1000`, `1006`, `1015`.
- _Question:_ Does this have anything to do with the ordering of URXVT and SGR? Or is this
  unrelated?
- _Answer:_ Yes, this is directly related to the `URXVT` and `SGR` ordering! Since we emit
  them in the new order `1015, 1006` to enable, we must disable them in the reverse order
  `1006, 1015` so the terminal doesn't get corrupted when our app exits.
- _File:_ `tui/src/core/ansi/generator/ansi_sequence_generator_output.rs`
- _The Fix:_ DONE. Updated `disable_mouse_tracking()` to emit sequences in reverse LIFO order, and updated `test_disable_mouse_tracking` to reflect this.

#### [x] Motion Flag Vulnerability

The PR strips Shift/Alt/Ctrl in `detect_scroll_event` but leaves the `MOUSE_MOTION_FLAG`.
For bulletproof detection, strip the motion flag alongside the modifiers
(`let base = cb & !(SHIFT | ALT | CTRL | MOUSE_MOTION_FLAG);`).

- _Context:_ When you scroll, the terminal sends a base code of `64`. If you hold Shift,
  it adds `4` (making it `68`). `detect_scroll_event` strips out these modifiers so we
  always get back to the clean `64` to verify it's a scroll. But terminals also have a
  flag for "Motion" which adds `32`. While scrolling usually doesn't include physical
  motion, if a weird terminal _did_ send "Scroll + Motion", the code would be
  `64 + 32 = 96`. Because the current code doesn't strip the `32` bit, it would look at
  `96` and fail to recognize the scroll. By adding `MOUSE_MOTION_FLAG` to the list of bits
  we strip, we make it 100% bulletproof.
- _Question:_ What is the mouse motion flag? When is this sent by the terminal as a result
  of the user performing what action?
- _Answer:_ The motion flag is sent by the terminal when the user physically slides the
  mouse across the desk. It is the 6th bit (value `32`) in the byte sent by the terminal.
  If you click the Left button, it sends code `0`. If you click the Left button AND drag
  the mouse, it sends `0 + 32 = 32`.
- _File:_ `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs`
- _The Fix:_ DONE. Refactored `MOUSE_BASE_BUTTON_MASK` to explicitly whitelist ONLY bits 0, 1, and 6 (`0b0100_0011`). This mathematically strips the motion flag (bit 5) alongside the modifier bits, guaranteeing 100% bulletproof isolation.

### Final Verification & Cleanup

- [ ] Verify full test suite coverage using `./check.fish --full`.
- [ ] Run interactive rebase (`git rebase -i main`) on the PR branch to drop the 2
      unneeded commits, squash the remaining 4 into 1, and rewrite the commit message.
      (Git will naturally preserve `@cecton` as the author).
- [ ] Force-push the updated branch to GitHub, which automatically updates the PR.
- [ ] Merge the PR into `main`.
- [ ] Update `task/prepare-v0.8.0-meta-task.md` to check off PR #448.
- [ ] **Mandatory manual review:** Verify every file modified in this task for correct
      implementation and ensure no regressions.
  - [ ] `tui/src/core/ansi/constants/mouse.rs`
  - [ ] `tui/src/core/ansi/generator/ansi_sequence_generator_input.rs`
  - [ ] `tui/src/core/ansi/generator/ansi_sequence_generator_output.rs`
  - [ ] `tui/src/core/ansi/vt_100_terminal_input_parser/mouse.rs`
  - [ ]
    `tui/src/core/ansi/vt_100_terminal_input_parser/validation_tests/input_parser_validation_test.rs`
  - [ ] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/tests.rs`
  - [ ] `tui/examples/mouse_inspector.rs`
  - [ ] `task/prepare-v0.8.0-meta-task.md`
