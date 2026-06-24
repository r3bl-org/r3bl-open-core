_Task: PR 455 Integration (DA1 Responses timeout fix)_

# User Story & Context

## Problem

Currently, when a user launches the PTY multiplexer example (by running
`cargo run --example pty_mux_example`) and tries to run certain interactive terminal
applications inside it like `hx` (Helix editor), they experience a significant delay or
timeout during startup. This happens with `fish` (Fish shell) too.

## Root Cause

During startup, the application process attempts to negotiate capabilities, like whether
the "terminal" supports certain colors or features, by sending a Primary Device Attributes
(DA1) query.

The process assumes that it is running in either a:

1. Terminal emulator: PTY, typically `/dev/pts/<n>` (Linux) or `/dev/ttysN` (BSD).
2. Hardware/virtual console managed by the Linux kernel (TTY, like `/dev/ttyN`), accessed
   using `Ctrl+Alt+F[1..4]`.

It attempts to communicate with the terminal by reading and writing directly to its
`stdin` and `stdout` file descriptors.

1. The process writes the raw byte sequence to the "terminal", to its `stdout`. According
   to standard terminal specifications, such as those documented in [VT100][VT100] and
   [XTerm Control Sequences][XTerm], it can write either of these:

- `\x1b[c` (which is the `ESC` byte `0x1b`, followed by `[` and `c`)
- `\x1b[0c` (which is the `ESC` byte `0x1b`, followed by `[` and `0c`)

2. Then it waits for the "terminal" to reply with a matching sequence of bytes via its
   `stdin`.

However, because this process is running inside the `pty_mux` engine its `stdout` goes to
the `pty_mux` engine, and not the "terminal". The query bytes never actually reaches the
true host terminal emulator (like WezTerm or Alacritty) or virtual console (Linux kernel,
`Ctrl+Alt+F[1..4]`).

1. Instead, `AnsiToOfsBufPerformer::csi_dispatch` method in the VT100 output parser
   intercepts the sequence, explicitly ignores it, logs a warning ("CSI c: Device
   Attributes query not supported in multiplexer"), and drops it entirely.
2. As a result the child process never receives the expected response back through its
   `stdin`, it blocks and waits until a hardcoded timeout is reached before finally
   falling back to a default degraded state. This is why `hx` takes a long time after it
   is launched to display anything to the screen (in the `pty_mux_example`).

## Expected Behavior

When a (PTY child) process asks for device attributes, by writing the query bytes to its
`stdout`, `pty_mux` should immediately parse these bytes in the VT100 output parser, and
respond with its capabilities, by writing the response bytes to the PTY controller (which
the operating system then routes directly into the child process's `stdin`).

> Note that `pty_mux` does not know (nor does it care) what host terminal emulator or
> virtual console it is actually running in. To the child process, `pty_mux` _is_ the
> terminal. Our VT100 output parser generates pixel chars that are painted to an
> OffscreenBuffer, and the rendering of this is left up to the terminal backend, whose
> responsibility it is to render it as best it can.

Specifically, the system should execute a complete round-trip through the `pty_mux`
architecture to deliver the `CSI ? 62 ; 22 c` response:

1. **The event dispatch:** Inside the output parser, the `csi_dispatch` method recognizes
   the `c` query and creates a `PtyResponseEvent::PrimaryDeviceAttributes` response event.
   We need to add this variant to the enum as part of this impl. It should push this into
   the returned `pending_pty_response_events` Vec (returned by `apply_ansi_bytes()`).
2. **The ANSI bytes serialization builder:** The `Process` struct (managed by
   `ProcessManager`) receives this event and uses the `DaSequence` enum (which we will
   create) to format it into the byte sequence `\x1b[?62;22c` (which is the `ESC` byte
   `0x1b`, followed by `[?62`, `;`, `22`, `c`).
   - _Architecture Note:_ `PtyResponseEvent` is the "domain logic" event flowing through
     the channels, while `DaSequence` (like `DsrSequence`) is strictly a "serialization
     builder" that formats the event into raw ANSI bytes. We separate these concerns so
     formatting is decoupled from event routing.
3. **The delivery:** The `Process` struct wraps these bytes in a `PtyInputEvent::Write`
   message and pushes it into its `PtySession`'s `tx_input_event` channel. This
   asynchronous channel routes the message to the background PTY writer task, which then
   writes those bytes directly into the PTY controller file descriptor (which the OS
   routes directly to the child process's `stdin`).

This allows processes to immediately complete their capability negotiation and start up
instantly without any timeouts.

Here's a breakdown of the `\x1b[?62;22c`, according to standard terminal specifications,
such as those documented in [VT100][VT100] and [XTerm Control Sequences][Xterm]:

- the `62` indicates a VT220-family terminal (establishing a widely-compatible baseline
  that tells the application it supports advanced control sequences beyond the original
  VT100)
  - The VT200 series (specifically VT220) is a strict superset of VT100. By responding
    with 62 (VT220-family) and the parameter 22 (ANSI color support), we are essentially
    telling the child application: "I fully support the VT100 specification, but I also
    support modern extensions like ANSI color and advanced control sequences."
  - This is a standard industry practice. Almost all modern terminal emulators (like
    WezTerm, Alacritty, GNOME Terminal, etc.) identify themselves as VT220, VT320, or
    VT420 for exactly this reason: to unlock colors and modern features in child apps
    while remaining backwards compatible with the VT100 standard.
  - Note - In our codebase we use the `VT100` in our type & module names because it's the
    universally recognized name for the technology and protocol. It encompasses VT220 with
    color extensions, etc. It's very similar to how we still use the term TTY (which
    stands for Teletypewriter) even though we haven't used mechanical teletypewriters with
    ink and paper in over 40 years.
- the `22` indicates ANSI color support.

# Overview

PR 455 (by Cecile Tonglet) originally implemented the DA1 (Primary Device Attributes,
`CSI c`) query handling in the PTY mux shim to prevent these timeouts. It responds with
`CSI ? 62 ; 22 c` on DA1 and correctly forwards DA responses alongside existing channels.

# PR Handling & Attribution

We are implementing our own fixes based on Cecile's original intent and closing her PR
without merging it directly due to underlying architectural changes. We will add a
`Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>` trailer to all of the commits
we make for this task to ensure she gets proper attribution for the feature!

# Implementation plan

## Phase 1: Terminal Response Channel Generalization & DA1 Support

**Architecture Note:** Changing `apply_ansi_bytes()` to return a 3-tuple would cause
massive code churn across hundreds of tests. To reuse the existing response channel
without violating semantic naming and type safety, we will generalize the DSR channel into
a `PtyResponse` channel that handles both DSR and DA events.

- [ ] Rename `DsrRequestFromPtyEvent` to `PtyResponseEvent` across the codebase.
  - Move the definition out of `dsr_sequence.rs` into its own file at
    `tui/src/core/ansi/generator/pty_response_event.rs` to reflect its generalized role.
- [ ] Add a `PrimaryDeviceAttributes` variant to the new `PtyResponseEvent` enum.
  - _Note: Using `PrimaryDeviceAttributes` instead of just `DeviceAttributes` keeps
    terminology precise, leaving room for DA2 (Secondary) and DA3 (Tertiary) in the
    future._
- [ ] Rename the `pending_dsr_responses` vector to `pending_pty_response_events` in
      `OfsBufVt100`, `ProcessManager`, and related components. This provides a clear 1:1
      mapping with `PtyResponseEvent` and perfectly parallels `pending_osc_events`.
- [ ] Create a new `DaSequence` type in the `generator` module
      (`tui/src/core/ansi/generator/da_sequence.rs`) that implements `Display` and
      `FastStringify` to format and output the DA1 response `\x1b[?62;22c`.
  - Update `PtyResponseEvent::Display` to delegate formatting to `DaSequence` for DA1
    events.
- [ ] Update `AnsiToOfsBufPerformer::csi_dispatch` in
      `tui/src/core/ansi/vt_100_pty_output_parser/performer.rs` to handle the `'c'`
      dispatch character.
  - Explicitly consume and ignore unhandled DA queries like DA2 (intermediate `>`) or DA3
    (intermediate `=`) by logging a warning via
    `DEBUG_TUI_VT100_PARSER.then(|| tracing::warn!(...))` to match existing parser
    patterns.
- [x] Create new files `vt_100_shim_da_ops.rs` and `vt_100_impl_da_ops.rs` for DA
      operations to maintain module cohesion.
- [x] Implement `vt_100_shim_da_ops::device_attributes` to parse the `'c'` sequence:
  - Check `intermediates` and `params` to ensure it is a valid DA1 query.
  - **DA1 Parameter Edge Case:** Ensure the shim accepts both `CSI c` (no params) and
    `CSI 0 c` (param = 0) as functionally identical valid queries.
  - Delegate valid DA1 requests to
    `performer.ofs_buf_vt_100.handle_device_attributes_request()`.
- [x] Implement `handle_device_attributes_request` in `vt_100_impl_da_ops.rs` to push the
      `PrimaryDeviceAttributes` event into the renamed `pending_pty_response_events`
      vector.
- [x] Testing Requirements:
  - Add unit tests for `DaSequence` formatting (similar to existing tests in
    `dsr_sequence.rs`).
  - Add parser logic tests to ensure both `CSI c` and `CSI 0 c` are correctly parsed as
    DA1.
  - Update conformance tests for DA1 behavior in the `vt_100_pty_output_conformance_tests`
    module.
- [x] Run `./check.fish --check` and `./check.fish --test` to ensure stability.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
  - [x] `tui/src/core/ansi/generator/mod.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/pty_response_event.rs`
  - [x] `tui/src/core/ansi/generator/da_sequence.rs`
  - [x] `tui/src/core/ansi/generator/dsr_sequence.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/mod.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_da_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/mod.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_da_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/performer.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/mod.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/tests/vt_100_test_da_ops.rs`
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
  - [x] `tui/src/core/ansi/constants/da.rs`
  - [x] `tui/src/core/ansi/constants/mod.rs`

## Phase 2: Expand `pty_mux_example`

- [x] Update `tui/examples/pty_mux_example.rs` to spawn a `fish` shell process in addition
      to the existing processes.
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation.
  - [x] `tui/examples/pty_mux_example.rs`
- [x] **Mandatory manual testing:** Verify that `fish` works in `pty_mux_example` without
      delay
- [x] **Mandatory manual testing:** Verify that `hx` works in `pty_mux_example` without
      delay
- [x] Update `task/prepare-v0.8.0-meta-task.md` to check off this PR.

## Phase 3: Fix `fish` Visual Artifacts (Underlines)

During manual testing in Phase 2, persistent underlines were observed when running `fish`
and executing `clear`.

### Investigation Findings

- **1. The Issue**: During manual testing of the `pty_mux` example with `fish`, persistent
  underlines appeared on the screen immediately after executing the `clear` command.
- **2. The Root Cause**: The bug was located in how `OfsBufVT100` simulates terminal erase
  operations. When `fish` cleared the screen (`CSI 2 J`), it happened to have an underline
  attribute active. `OfsBufVT100::create_empty_pixel_char()` was erroneously copying the
  **entire** active `TuiStyle` (including text attributes like underline) from the
  `ParserGlobalState`, and assigning it to the blank spaces it generated. This violated
  the "Background Color Erase" (BCE) terminal spec, which mandates that erased areas
  should only inherit the current _background color_, **not** text attributes.
- **3. The Fix**: Updated `OfsBufVT100::create_empty_pixel_char()` to drop all text
  attributes and the foreground color, retaining _only_ the `color_bg` when generating
  blank spaces for erase operations. This perfectly fixed the `fish` + `clear` underline
  bug.
- **4. The Side Quest Fix**: While investigating this, we mistakenly hypothesized that
  `PixelCharRenderer` was leaking style state across render passes. To fix this, we
  updated `PixelCharRenderer::render_line()` to ensure it always appends `\x1b[0m` (reset)
  if it leaves any style active at the end of a string. While this didn't fix the `fish`
  bug, it was a critical catch! Because `PixelCharRenderer` resets its internal memory at
  the start of every render pass, failing to clean up the hardware terminal at the end of
  a pass would cause subsequent default-text renders to silently inherit the active
  terminal style. We merged this defensive fix and added tests for it.

### Implementation Steps

- [x] Update
      `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/pixel_char_renderer.rs` to
      append `SGR_RESET_BYTES` at the end of `render_line` if `has_active_style` is true
      (Good defensive practice).
- [x] Update
      `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
      so `create_empty_pixel_char` only inherits `color_bg` and resets text attributes.
- [x] Run `./check.fish --check` and `./check.fish --test`.
- [x] **Mandatory manual testing:** Run `pty_mux_example` with `fish` and verify that the
      persistent underlines after running `clear` are fixed.
- [x] **Mandatory manual review:**
  - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops_impl_ofs_buf/vt_100_impl_clear_ops.rs`
  - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/output/pixel_char_renderer.rs`

# References

[VT100]: https://vt100.net/docs/vt220-rm/chapter4.html
[XTerm]: https://invisible-island.net/xterm/ctlseqs/ctlseqs.html
