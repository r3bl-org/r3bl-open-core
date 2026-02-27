<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

Audit and fix all rustdoc comments (`///` and `//!`) in `tui/src/core/ansi/` to properly backtick
and link technical acronyms per the `write-documentation` skill rules.

## Rules

1. **Only rustdoc comments** (`///` and `//!`) are in scope. Regular `//` comments are excluded.
2. **Content inside code blocks** (` ```text `, ` ```rust `, ` ```bash `, etc.) is excluded.
3. **Content inside inline backticks** (`` `already backticked` ``) is already correct.
4. **Reference-style link definitions** (e.g., `[`ANSI`]: https://...`) are excluded (they are the
   target, not prose).

## Terms and Actions

### Backtick + Link (reference-style `[`TERM`]` with URL at bottom of doc block)

**Technical standards and protocols:**

| Term                   | Link URL                                                        |
| :--------------------- | :-------------------------------------------------------------- |
| `VT-100`               | `https://vt100.net/docs/vt100-ug/chapter3.html`                |
| `VT-100` spec          | `https://vt100.net/docs/vt100-ug/chapter3.html`                |
| `VT-100` specification | `https://vt100.net/docs/vt100-ug/chapter3.html`                |
| `VT-220`               | `https://en.wikipedia.org/wiki/VT220`                           |
| `DEC`                  | `https://en.wikipedia.org/wiki/Digital_Equipment_Corporation`   |
| `ANSI`                 | `https://en.wikipedia.org/wiki/ANSI_escape_code`               |
| `UTF-8`                | `https://en.wikipedia.org/wiki/UTF-8`                           |

**Software product names** (backtick + link per `write-documentation` skill):

| Term             | Link URL                                                |
| :--------------- | :------------------------------------------------------ |
| `xterm`          | `https://en.wikipedia.org/wiki/Xterm`                   |
| `Alacritty`      | `https://alacritty.org/`                                |
| `kitty`          | `https://sw.kovidgoyal.net/kitty/`                      |
| `gnome-terminal` | `https://help.gnome.org/users/gnome-terminal/stable/`   |
| `iTerm2`         | `https://iterm2.com/`                                   |

Each doc comment block (`//!` module doc or `///` item doc) is a **separate link scope** - link
definitions must be repeated in each block that uses them.

### Backtick Only (no link, just `` `TERM` ``)

`ESC`, `CSI`, `SGR`, `OSC`, `SS3`, `DCS`, `APC`, `SOS`, `PM`, `ASCII`, `RGB`, `PTY`,
`RXVT`, `X10`, `EOF`, `FIFO`, `VTE`, `DEL`, `Unicode`

Note: `XTerm` was previously in this list but moved to Backtick + Link as `xterm` (the exemplar
`constants/generic.rs` links it to Wikipedia). When referring to the standard/spec (e.g., "`XTerm`
Control Sequences"), use backtick-only `XTerm`; when referring to the software product, use
backtick+link `` [`xterm`] ``.

## Scope Estimate

~400+ individual changes across ~35 files in `tui/src/core/ansi/`.

# Implementation plan

## Step 0: Fix `vt_100_terminal_input_parser/validation_tests/` [COMPLETE]

These three files were fixed in a prior session:

- `validation_tests/mod.rs` - VT-100, ANSI linked
- `validation_tests/input_parser_validation_test.rs` - VT-100, ANSI, CSI backticked/linked
- `validation_tests/observe_real_interactive_terminal_input_events.rs` - ANSI, SGR backticked/linked

## Step 1: Fix `vt_100_terminal_input_parser/utf8.rs` [COMPLETE]

User explicitly requested this file. Needs: UTF-8 linked, ANSI linked, ASCII backticked.

~30 instances of UTF-8, ~2 ANSI, ~3 ASCII in rustdoc comments (excluding code blocks).

## Step 2: Fix `vt_100_terminal_input_parser/keyboard.rs` [COMPLETE]

Largest file in the module. Needs: VT-100 linked, ANSI linked, UTF-8 linked, ASCII backticked,
ESC backticked.

~52 ESC, ~10 ASCII, ~8 UTF-8, ~5 ANSI, ~2 VT-100 instances in rustdoc.

## Step 3: Fix `vt_100_terminal_input_parser/mouse.rs` [COMPLETE]

Needs: VT-100 linked, SGR backticked, CSI backticked, ASCII backticked, ESC backticked.

Also: software product names `xterm`, `gnome-terminal`, `iTerm2` on lines 80-81 need
backtick+link.

## Step 4: Fix `vt_100_terminal_input_parser/router.rs` [COMPLETE]

Needs: VT-100 linked, ESC backticked, UTF-8 backticked/linked, CSI backticked.

~13 ESC, ~2 VT-100 instances.

## Step 5: Fix `vt_100_terminal_input_parser/ir_event_types.rs` [COMPLETE]

Needs: VT-100 linked.

~5 VT-100 instances.

## Step 6: Fix `vt_100_terminal_input_parser/mod.rs` [COMPLETE]

Needs: VT-100 linked, ANSI linked, UTF-8 linked, ESC backticked, SGR backticked, CSI backticked.

~10 VT-100, ~3 ANSI, ~5 UTF-8 instances.

## Step 7: Fix `vt_100_terminal_input_parser/integration_tests/` [COMPLETE]

Multiple files:

- `pty_mouse_events_test.rs` - VT-100 linked
- `pty_bracketed_paste_test.rs` - UTF-8 linked, ASCII backticked
- `pty_utf8_text_test.rs` - UTF-8 linked, ASCII backticked
- `pty_input_device_test.rs` - ESC backticked
- `mod.rs` - VT-100 linked

## Step 8: Fix `vt_100_terminal_input_parser/unit_tests/` [COMPLETE]

Scan for unbackticked terms in test module docs.

## Step 9: Fix `mod.rs` (top-level `core/ansi/mod.rs`) [COMPLETE]

Needs: ANSI linked, VT-100 linked, PTY backticked, RGB backticked, ESC backticked, CSI backticked,
SGR backticked, OSC backticked.

Also: software product names `Alacritty` (lines 139, 154) and `Kitty` (line 188) need
backtick+link.

~50+ ANSI, ~4 ESC, ~5 PTY, ~3 RGB instances.

## Step 10: Fix `generator/` submodule [COMPLETE]

Multiple files:

- `sgr_code.rs` - ANSI linked, ESC backticked, `xterm` backtick+linked (lines 43, 51, 53),
  `RGB` backticked
- `dsr_sequence.rs` - ANSI linked, ESC backticked
- `ansi_sequence_generator_input.rs` - ANSI linked, ASCII backticked, SGR backticked,
  VT-100 linked (lines 662, 664)
- `esc_sequence.rs` - ESC backticked, ASCII backticked
- `mod.rs` - ESC backticked

## Step 11: Fix `color/` submodule [COMPLETE]

- `ansi_value.rs` - ANSI linked (~7 instances), RGB backticked (lines 75, 86, 105)
- `rgb_value.rs` - RGB backticked (lines 3, 11, 152)
- `convert.rs` - ANSI linked (~2 instances), RGB backticked (line 171)
- `mod.rs` - RGB backticked (line 10)

## Step 12: Fix `constants/` submodule [COMPLETE]

Multiple files:

- `utf8.rs` - UTF-8 linked, ASCII backticked (~15 instances each)
- `input_sequences.rs` - ASCII backticked, VT-100 linked, ESC backticked (~15 ASCII)
- `generic.rs` - ESC backticked
- `esc.rs` - ESC backticked, ASCII backticked (~18 ESC)
- `mouse.rs` - ESC backticked
- `dsr.rs` - ESC backticked
- `mod.rs` - ESC backticked, UTF-8 linked

## Step 13: Fix `detect_color_support.rs` [COMPLETE]

Needs: OSC backticked (~5 instances), `xterm` backtick+linked (line 340).

## Step 14: Fix `terminal_raw_mode/mod.rs` [COMPLETE]

Needs: ESC backticked.

## Step 15: Fix `vt_100_pty_output_parser/` submodule [COMPLETE]

Multiple files:

- `performer.rs` - PTY backticked, CSI backticked, ESC backticked
- `ansi_parser_public_api.rs` - ANSI linked, PTY backticked, ASCII backticked, ESC backticked
- `operations/vt_100_shim_*.rs` - ESC backticked, ASCII backticked, OSC backticked across ~10 files
- `protocols/csi_codes/mod.rs` - CSI backticked, ESC backticked
- `protocols/csi_codes/sgr_color_sequences.rs` - ESC backticked, RGB backticked

## Step 16: Fix `vt_100_pty_output_parser/vt_100_pty_output_conformance_tests/` [COMPLETE]

Multiple files:

- `conformance_data/basic_sequences.rs` - VT-100 linked, ESC backticked
- `conformance_data/cursor_sequences.rs` - ESC backticked
- `conformance_data/mod.rs` - ESC backticked
- `tests/vt_100_test_*.rs` files - various terms
- `test_sequence_generators/mod.rs` - ANSI linked, VT-100 linked

## Step 17: Run `./check.fish --doc` and verify no broken links [COMPLETE]

After all changes, verify that:

1. `./check.fish --doc` passes (no broken intra-doc links)
2. `cargo rustdoc-fmt` is clean (formatting)
3. `./check.fish --clippy` passes
