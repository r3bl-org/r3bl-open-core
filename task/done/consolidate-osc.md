# Task: Consolidate OSC Module into ANSI Subsystem and Add First-Class Color Query/Report Support

## Overview

Operating System Command (OSC) escape sequences in `r3bl_tui` currently suffer from
architectural fragmentation:

1. **Misplaced Module**: `core/osc` sits at the top level of `core/`, while all other ANSI
   sequence families (ESC, DSR, SGR in `core/ansi/generator`, and CSI in
   `core/ansi/vt_100_pty_output_parser`) live under `core/ansi/`. This causes inverted
   dependencies (e.g. `core/ansi/constants/macros.rs` importing backward from
   `core/osc/osc_codes.rs`).
2. **Missing Outbound Color Query/Report Generation**: `vt_100_terminal_input_parser`
   decodes inbound color reports (`OSC 10..19`), but `OscSequence` lacks `ColorQuery` and
   `ColorReport` variants to generate outbound queries or reports. Synthetic color reports
   were trapped in test-only code in `generator/ansi_input.rs`.
3. **Outdated Module Rustdoc**: `tui/src/core/osc/mod.rs` lines 3-25 only mention OSC 0,
   8, 9;4, and 52, completely omitting dynamic color queries/reports (`OSC 10..19`), the
   bidirectional flow, and the relationship to `vt_100_terminal_input_parser`.
4. **Missing Bidirectional Cross-References**: Code for generating OSC sequences via
   `OscController`/`OscSequence` and code for interpreting incoming OSC bytes into
   `InputEvent::TerminalColor` are two sides of the same coin, but lack intra-doc links
   and cross-references explaining their lifecycle.
5. **Missing PTY Integration Tests**: No PTY integration test verifies the true end-to-end
   bidirectional query-response loop between a virtual terminal emulator and
   `DirectToAnsiInputDevice`.

This task moves `core/osc/` to `core/ansi/osc/`, equips `OscSequence` and `OscController`
with first-class `ColorQuery` and `ColorReport` support, integrates with
`generator::ansi_output::osc`, updates module and symbol documentation with extensive
bidirectional cross-references, and adds an exhaustive PTY round-trip test.

## Implementation plan

### Phase 1: Move OSC to `core/ansi/osc` and Wire Re-exports

- [x] Move directory `tui/src/core/osc/` to `tui/src/core/ansi/osc/`
- [x] Update `tui/src/core/ansi/mod.rs` to declare `pub mod osc;` and re-export
      `pub use osc::*;`
- [x] Update `tui/src/core/mod.rs` to remove top-level `pub mod osc;` and re-export
      `pub use ansi::osc::*;` to maintain 100% backward compatibility
- [x] Fix inverted dependency in `tui/src/core/ansi/constants/macros.rs` to point to
      `crate::core::ansi::osc::osc_codes::OSC_START`
- [x] Re-export `OscSequence` and `ClipboardTarget` in
      `tui/src/core/ansi/generator/mod.rs`
- [x] Run `./check.fish --check` to ensure all existing call sites and re-exports compile
      cleanly
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/mod.rs`
    - [x] `tui/src/core/mod.rs`
    - [x] `tui/src/core/ansi/constants/macros.rs`
    - [x] `tui/src/core/ansi/generator/mod.rs`
    - [x] `tui/src/core/ansi/osc/mod.rs`

### Phase 2: Add First-Class Color Query & Report Support to `OscSequence`

- [x] Use existing `TerminalColorRole::as_str(&self)` directly in
      `tui/src/core/ansi/osc/osc_codes.rs`
- [x] Add `ColorQuery(TerminalColorRole)` and `ColorReport(TerminalColorReport)` variants
      to `OscSequence` in `tui/src/core/ansi/osc/osc_codes.rs`
- [x] Implement `FastStringify` formatting for `ColorQuery` (`\x1b]<code>;?\x07`) and
      `ColorReport` (`\x1b]<code>;rgb:rrrr/gggg/bbbb\x1b\`) in `osc_codes.rs`
- [x] Add unit tests in `osc_codes.rs` verifying byte generation for all 7 roles
      (`OSC 10, 11, 12, 13, 14, 17, 19`) and round-trip decoding with
      `terminal_events::osc_color::parse`
- [x] Add `query_color()`, `query_background_color()`, and `query_foreground_color()`
      methods to `OscController` in `tui/src/core/ansi/osc/osc_controller.rs`
- [x] Delegate to `OscSequence` and `OscController` directly for outbound OSC generation
      (avoiding redundant `ansi_output::osc` shims)
- [x] Deduplicate `generate_color_report_sequence` by delegating directly to
      `OscSequence::ColorReport` (preserving test fixture API symmetry) and relocate test
      input generators to `tui/src/core/ansi/generator/test_fixtures/`
- [x] Run `./check.fish --check` and `./check.fish --test` on modified modules
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/osc_controller.rs`
    - [x] `tui/src/core/ansi/generator/mod.rs`
    - [x] `tui/src/core/ansi/generator/test_fixtures/mod.rs`
    - [x] `tui/src/core/ansi/generator/test_fixtures/ansi_input.rs`

### Phase 3: Consolidate OSC Deserialization into `core/ansi/osc`

- [x] Implement `FromStr` for `TerminalColorRole` in
      `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs` to unify forward
      (`as_str()`) and reverse (`parse()`) code mappings on the enum
- [x] Create `tui/src/core/ansi/osc/osc_color.rs` containing dedicated color parsing
      logic:
    - `strip_osc_enclosure`
    - `split_osc_command_and_payload`
    - `parse_color_spec` (`parse_rgb_spec_color`, `parse_hex_channel`,
      `parse_hash_hex_color`)
    - `parse_color_report(&[u8]) -> Option<TerminalColorReport>`
    - `parse_color_query(&[u8]) -> Option<TerminalColorRole>`
- [x] Add `OscSequence::parse(bytes: &[u8]) -> Option<Self>` in
      `tui/src/core/ansi/osc/osc_codes.rs` delegating to `osc_color`
- [x] Register `mod osc_color; pub use osc_color::*;` in `tui/src/core/ansi/osc/mod.rs`
- [x] Streamline
      `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`:
    - Remove private `mod osc_color` (150 lines of duplicate parsing)
    - Delegate `parse_osc_response` to `OscSequence::parse`
- [x] Run `./check.fish --check` and `./check.fish --test` on modified modules
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
    - [x] `tui/src/core/ansi/osc/osc_color.rs`
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/mod.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`

### Phase 4: Consolidate OSC Constants into `constants/osc_constants.rs`

- [x] Create `tui/src/core/ansi/constants/osc_constants.rs` defining all 43 OSC constants
      across 6 logical groups:
    - Group 1: Enclosures, Delimiters, Queries & Terminators (`ANSI_OSC_CLOSE_BRACKET`,
      `OSC_PREFIX`, `OSC_PREFIX_LEN`, `OSC_START`, `OSC_START_BYTES`, `OSC_DELIMITER`,
      `OSC_DELIMITER_BYTE`, `OSC_QUERY`, `OSC_QUERY_STR`, `OSC_QUERY_BYTES`,
      `OSC_TERMINATOR_BEL`, `OSC_TERMINATOR_BEL_BYTE`, `OSC_TERMINATOR_ST`,
      `OSC_TERMINATOR_ST_BYTES`, plus backward-compatibility aliases `ANSI_BEL`,
      `ANSI_ST_FINAL`, `ANSI_ST_7BIT_TRANSPORT_ENCODING`,
      `ANSI_ST_7BIT_TRANSPORT_ENCODING_LEN`)
    - Group 2: Semantic Sequence Prefixes & Ends (`OSC_TITLE_AND_ICON_START`,
      `OSC_ICON_START`, `OSC_TITLE_START`, `OSC_HYPERLINK_START`, `OSC_PROGRESS_START`,
      `OSC_TITLE_END`, `OSC_HYPERLINK_END`, `OSC_PROGRESS_END`)
    - Group 3: Command Identification Codes (`OSC_CODE_TITLE_AND_ICON`, `OSC_CODE_ICON`,
      `OSC_CODE_TITLE`, `OSC_CODE_HYPERLINK`, `OSC_CODE_PROGRESS`,
      `OSC_PROGRESS_SUBCOMMAND`, `OSC_PROGRESS_STATE_UPDATE`, `OSC_CODE_CLIPBOARD`,
      `OSC_CODE_COLOR_REPORT_*` for all 7 roles)
    - Group 4: Color Specification Payload Tokens (`OSC_COLOR_SPEC_RGB_PREFIX`,
      `OSC_COLOR_SPEC_CHANNEL_SEPARATOR`, `OSC_COLOR_SPEC_HASH_PREFIX`)
    - Group 5: Clipboard Target Tokens (`CLIPBOARD_TARGET_CLIPBOARD`,
      `CLIPBOARD_TARGET_PRIMARY`)
    - Group 6: Framing Limits & Circuit Breaker Ceilings (`MAX_OSC_SEQUENCE_LENGTH`,
      `MAX_OSC_DRAIN_BYTES`)
    - Add Tier 1 and Tier 2 rustdocs and unit tests for constant values
- [x] Register `pub mod osc_constants; pub use osc_constants::*;` in
      `tui/src/core/ansi/constants/mod.rs`
- [x] Register `osc_constants` in
      `#[doc(inline)] pub use constants::{..., osc_constants, ...};` in
      `tui/src/core/ansi/mod.rs` with clean `pub use osc::*;` without namespace collisions
- [x] Purge redundant OSC constants from `tui/src/core/ansi/constants/input_sequences.rs`
      and remove corresponding assertions from its unit tests
- [x] Decouple `tui/src/core/ansi/osc/osc_codes.rs`: remove duplicate constant definitions
      and eliminate `pub use constants` re-exports, importing only constants needed for
      `OscSequence`
- [x] Update callsites (`osc_buffer.rs`, `vt_100_shim_osc_ops.rs`, `osc_scanner.rs`,
      `terminal_events.rs`, `osc_color.rs`) to import directly from
      `crate::core::ansi::constants::*`
- [x] Run `./check.fish --check`, `./check.fish --test`, and `./check.fish --clippy`
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/constants/osc_constants.rs`
    - [x] `tui/src/core/ansi/constants/mod.rs`
    - [x] `tui/src/core/ansi/constants/input_sequences.rs`
    - [x] `tui/src/core/ansi/constants/macros.rs`
    - [x] `tui/src/core/ansi/mod.rs`
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/osc_buffer.rs`
    - [x] `tui/src/core/ansi/osc/osc_color.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_osc_ops.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/osc_scanner.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`

### Phase 5: Cross-Referenced Rustdocs (Bidirectional Generation & Parsing)

- [x] Rewrite module-level rustdoc in `tui/src/core/ansi/osc/mod.rs:3-25`:
    - Document that OSC handling in `r3bl_tui` spans both generation and parsing:
        1. Outbound sequence generation via `OscSequence` and `OscController` (`stdout`)
        2. Inbound terminal response parsing via `OscSequence::parse` and
           `vt_100_terminal_input_parser` (`stdin`) into `InputEvent::TerminalColor`
        3. Child PTY stream interception via `PtyOscProgressScanner` into `OscPtyEvent`
    - Document dynamic terminal color queries & reports (`OSC 10..19`) alongside OSC 0, 8,
      9;4, and 52
    - Explicitly document platform & backend constraints:
        - Outbound (`stdout`): Cross-platform across Linux (`DirectToAnsi`) and
          macOS/Windows (`Crossterm`)
        - Inbound (`stdin`): Linux-only via `DirectToAnsiInputDevice`. Crossterm lacks OSC
          parsing support and misinterprets replies as phantom keypresses (`Alt+]`, `1`,
          `1`, ...)
    - Add explicit cross-references linking outbound generation types to inbound event
      types
- [x] Cross-reference outbound generation types in `tui/src/core/ansi/osc/osc_codes.rs`
      and `osc_controller.rs`:
    - Add rustdoc on `OscSequence::ColorQuery` and `OscController::query_color` pointing
      to `vt_100_terminal_input_parser::terminal_events::parse_osc_response` and
      `InputEvent::TerminalColor` to explain where terminal replies are decoded
    - Add platform warning caveat that `query_color` requires `DirectToAnsiInputDevice`
      (Linux) and should not be invoked when running on the Crossterm backend
- [x] Cross-reference inbound parser types in
      `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs` and
      `chunk_decoder/terminal_events.rs`:
    - Link color report parsing documentation back to `OscSequence::ColorQuery` and
      `OscController::query_color` as the originating query constructors
- [x] Cross-reference `InputEvent::TerminalColor` in
      `tui/src/core/terminal_io/input_event.rs`:
    - Document that `TerminalColor` is emitted on `stdin` in response to queries sent via
      `OscSequence::ColorQuery` or `OscController::query_color`, and that it requires
      `DirectToAnsiInputDevice` (Linux)
- [x] Update `tui/src/lib.rs`:
    - Framework Highlights: document that inbound OSC response framing, absorption, and
      `InputEvent::TerminalColor` reporting is delivered by Linux-native `direct_to_ansi`
    - Backend Architecture & Platform Selection: add an explicit callout contrasting
      outbound OSC (cross-platform) with inbound OSC (`DirectToAnsi` on Linux only),
      warning against issuing queries under Crossterm (macOS & Windows)
    - Capability Matrix: annotate the _Terminal OSC Query Replies_ row indicating that
      framing and absorption applies to `direct_to_ansi` / `vt_100_terminal_input_parser`
- [x] Update `docs/release-notes/r3bl_tui/v0.8.0.md`:
    - Update Multi-Backend Architecture and Modern Terminal Input highlights
    - Add a dedicated section:
      `## 🌟 Net New Feature: Bidirectional OSC Subsystem & Dynamic Color Querying`
      clarifying that this is non-breaking net-new functionality (not an old-to-new
      migration recipe), demonstrating query and event handling, and detailing the
      Linux-only requirement for inbound response parsing
- [x] Run `./check.fish --quick-doc` to verify documentation builds without warnings and
      all intra-doc links resolve cleanly
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/osc/mod.rs`
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/osc_controller.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`
    - [x] `tui/src/core/terminal_io/input_event.rs`
    - [x] `tui/src/lib.rs`
    - [x] `docs/release-notes/r3bl_tui/v0.8.0.md`

### Phase 6: Virtual Terminal Emulator (PTY) Integration Tests & Scanner Clarification

- [x] Create
      `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`:
    - Implement `# Run with:` rustdoc header
      (`cargo test -p r3bl_tui --lib test_pty_osc_color -- --nocapture`)
    - Use `generate_pty_test!` in `PtyTestMode::Raw`
    - Controlled child initializes `DirectToAnsiInputDevice`, prints
      `OscSequence::ColorQuery` to stdout, and awaits `InputEvent::TerminalColor`
    - Controller parent acts as virtual terminal emulator: reads query from child stdout,
      synthesizes `OscSequence::ColorReport`, writes to slave stdin, and verifies child
      confirmation
    - Test multiple color roles (Background, Foreground, Cursor)
    - Verify inbound `OSC 52` clipboard response is safely absorbed without emitting
      spurious color events or physical keystrokes
    - Follow deadlock prevention guidelines with
      `child.drain_and_wait(buf_reader, pty_pair)`
- [x] Register `pub mod pty_osc_color_test;` in `vt_100_parser_integration_tests/mod.rs`
- [x] Run `cargo test -p r3bl_tui --lib test_pty_osc_color -- --nocapture` to verify PTY
      execution
- [x] Clarify and rename `OscBuffer` to `PtyOscProgressScanner`:
    - Rename `osc_buffer.rs` to `pty_osc_progress_scanner.rs` and introduce
      `PtyOscProgressScanner`
    - Clarify architectural taxonomy: `PtyOscProgressScanner` in `pty_session`
      specifically buffers and extracts streaming `OSC 9;4` progress sequences, whereas
      virtual terminal emulation (titles `OSC 0` and hyperlinks `OSC 8`) is handled by
      `OfsBufVT100` and verified in `vt_100_test_osc_ops.rs`
    - Rename `OscController` to `OscSender` and adopt hybrid `send_*` methods
      (`send_set_progress`, `send_set_title`, `send_set_icon`, `send_set_hyperlink`,
      `send_clear_hyperlink`, and enhanced `send_event`)
- [x] Verify `osc_capture_test.rs` passes for `PtyOscProgressScanner` progress capture
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/mod.rs`
    - [x] `tui/src/core/ansi/osc/osc_sender.rs`
    - [x] `tui/src/core/ansi/osc/pty_osc_progress_scanner.rs`
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/osc_pty_event.rs`
    - [x] `tui/src/core/ansi/osc/mod.rs`
    - [x] `tui/src/core/pty/pty_session/threads/reader.rs`
    - [x] `tui/src/core/pty/pty_session/builder.rs`
    - [x] `tui/src/core/ansi/constants/osc_constants.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ops/vt_100_shim_osc_ops.rs`
    - [x] `tui/src/tui/terminal_lib_backends/direct_to_ansi/mod.rs`

### Phase 7: Add `osc_diagnostics` Example to `r3bl_tui`

- [x] Create `tui/examples/osc_diagnostics.rs`:
    - Exercise OSC operations using `OscSender` (color queries for all roles, titles,
      hyperlinks, clipboard, progress)
    - Run in raw mode using `DefaultIoDevices` / `InputDevice` / `OutputDevice`
    - Check active backend / platform and display a warning banner if run under Crossterm
      (informing the user that inbound color responses cannot be parsed by Crossterm and
      will appear as raw keystrokes)
    - Read incoming events from `InputDevice` (matching `InputEvent::TerminalColor`)
    - Print generated queries and the corresponding responses received from the terminal
      emulator to stdout so the actual responses can be visualized
    - Support clean exit on 'q' or Ctrl+C
    - Document example usage in rustdoc (`cargo run --example osc_diagnostics`)
- [x] Verify execution with `cargo check --example osc_diagnostics`
- [x] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
    - [x] `tui/src/core/ansi/vt_100_pty_output_parser/ansi_parser_public_api.rs`
    - [x] `tui/examples/osc_diagnostics.rs`
    - [x] `check_cli.fish`
    - [x] `check.fish`
    - [x] `run.fish`

### Phase 8: Comprehensive Quality Checks

- [x] Run `./check.fish --check`
- [x] Run `./check.fish --clippy`
- [x] Run `./check.fish --test`
- [x] Run `./check.fish --quick-doc`
- [x] Run `git diff` to audit for surgical precision and documentation preservation
- [x] Run `fish -c "beep"` to signal completion
- [x] **Mandatory manual review:** Verify every modified file across the entire task.
    - [x] `tui/src/core/ansi/mod.rs`
    - [x] `tui/src/core/mod.rs`
    - [x] `tui/src/core/ansi/constants/macros.rs`
    - [x] `tui/src/core/ansi/constants/mod.rs`
    - [x] `tui/src/core/ansi/constants/input_sequences.rs`
    - [x] `tui/src/core/ansi/constants/osc_constants.rs`
    - [x] `tui/src/core/ansi/generator/mod.rs`
    - [x] `tui/src/core/ansi/generator/test_fixtures/mod.rs`
    - [x] `tui/src/core/ansi/generator/test_fixtures/ansi_input.rs`
    - [x] `tui/src/core/ansi/osc/mod.rs`
    - [x] `tui/src/core/ansi/osc/osc_codes.rs`
    - [x] `tui/src/core/ansi/osc/osc_color.rs`
    - [x] `tui/src/core/ansi/osc/osc_controller.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/mod.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/chunk_decoder/terminal_events.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/ir_event_types.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/mod.rs`
    - [x] `tui/src/core/ansi/vt_100_terminal_input_parser/vt_100_parser_integration_tests/pty_osc_color_test.rs`
    - [x] `tui/src/core/pty/e2e_tests/osc_capture_test.rs`
    - [x] `tui/src/core/terminal_io/input_event.rs`
    - [x] `tui/src/lib.rs`
    - [x] `docs/release-notes/r3bl_tui/v0.8.0.md`
    - [x] `tui/examples/osc_diagnostics.rs`
