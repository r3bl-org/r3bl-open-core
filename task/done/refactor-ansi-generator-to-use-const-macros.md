# Task: Refactor ANSI Generator to Use Const Macros and `ok!()`

## Overview

The ANSI generators currently use manual, multi-step buffer writes for static ANSI sequences
(e.g., writing `CSI_START`, then a digit, then `SGR_SET_GRAPHICS`). This is redundant since we
have a macro DSL that can pre-compute these full sequences as static string literals.

This refactor will:
1.  **Reduce boilerplate**: Use the pre-computed `&str` constants from our macro DSL.
2.  **Improve performance**: Single `push_str` call per static attribute instead of 3+.
3.  **Modernize style**: Replace `Ok(())` with the semantic `ok!()` macro (`core/decl_macros/macros.rs`).

## Implementation Plan

### 1. Expand Missing Constants

Add static attribute constants to the respective constants files using the macro DSL.
Constants already exist for: `SGR_RESET_STR`, `SGR_BOLD_STR`, `SGR_ITALIC_STR`,
`SGR_UNDERLINE_STR` (in `sgr.rs`); only the missing ones below need to be added.

#### `sgr.rs` (Select Graphic Rendition)
- [ ] Add `SGR_DIM_STR = ["2m"]`
- [ ] Add `SGR_SLOW_BLINK_STR = ["5m"]`
- [ ] Add `SGR_RAPID_BLINK_STR = ["6m"]`
- [ ] Add `SGR_INVERT_STR = ["7m"]`
- [ ] Add `SGR_HIDDEN_STR = ["8m"]`
- [ ] Add `SGR_STRIKETHROUGH_STR = ["9m"]`
- [ ] Add `SGR_OVERLINE_STR = ["53m"]`
- [ ] Add Reset variant strings:
    - `SGR_RESET_BOLD_DIM_STR = ["22m"]`
    - `SGR_RESET_ITALIC_STR = ["23m"]`
    - `SGR_RESET_UNDERLINE_STR = ["24m"]`
    - `SGR_RESET_BLINK_STR = ["25m"]`
    - `SGR_RESET_INVERT_STR = ["27m"]`
    - `SGR_RESET_HIDDEN_STR = ["28m"]`
    - `SGR_RESET_STRIKETHROUGH_STR = ["29m"]`

#### `esc.rs` (Escape Sequences)
- [ ] Add `ESC_SAVE_CURSOR_STR = ["7"]` (via `@esc_str`)
- [ ] Add `ESC_RESTORE_CURSOR_STR = ["8"]` (via `@esc_str`)
- [ ] Add `ESC_INDEX_DOWN_STR = ["D"]` (via `@esc_str`)
- [ ] Add `ESC_REVERSE_INDEX_STR = ["M"]` (via `@esc_str`)
- [ ] Add `ESC_RESET_TERMINAL_STR = ["c"]` (via `@esc_str`)
- [ ] Add `ESC_SELECT_ASCII_STR = ["(B"]` (via `@esc_str`)
- [ ] Add `ESC_SELECT_DEC_GRAPHICS_STR = ["(0"]` (via `@esc_str`)

#### `dsr.rs` (Device Status Report)
- [ ] Add `DSR_STATUS_OK_RESPONSE_STR = ["0n"]` (via `@dsr_str`)

### 2. Refactor Generators (Static Arms)

Replace multi-step writes with single `push_str(CONSTANT)` for arms that produce
fully static sequences.

#### `sgr_code.rs` — 18 STATIC arms
- [ ] Replace 3-step writes (CSI_START + digit + SGR_SET_GRAPHICS) with single `push_str`:
  Reset, Bold, Dim, Italic, Underline, SlowBlink, RapidBlink, Invert, Hidden,
  Strikethrough, Overline, ResetBoldDim, ResetItalic, ResetUnderline, ResetBlink,
  ResetInvert, ResetHidden, ResetStrikethrough.
- [ ] ForegroundBasic/BackgroundBasic (SEMI-STATIC): Leave as match dispatch for now.
  Each arm does 3 pushes but requires runtime `ANSIBasicColor` dispatch. Per-color
  constants (32 total) are possible but low-priority.
- [ ] ForegroundAnsi256, BackgroundAnsi256, ForegroundRGB, BackgroundRGB: DYNAMIC — no change.

#### `esc_sequence.rs` — 7 STATIC arms
- [ ] Replace multi-step `push(ESC_START); push(...)` with single `push_str(CONSTANT)`:
  SaveCursor, RestoreCursor, IndexDown, ReverseIndex, ResetTerminal,
  SelectAscii (3 pushes → 1), SelectDECGraphics (3 pushes → 1).

#### `dsr_sequence.rs` — 1 STATIC arm
- [ ] Replace `push_str(DSR_STATUS_OK_CODE); push(DSR_STATUS_RESPONSE_END)` in
  `StatusOkResponse` with single `push_str(DSR_STATUS_OK_RESPONSE_STR)`.
- [ ] CursorPositionResponse: DYNAMIC — no change.

### 3. Replace `Ok(())` with `ok!()`

All `FastStringify::write_to_buf()` implementations across the codebase.

| File | Location | `Ok(())` count |
|---|---|---|
| `sgr_code.rs` | `SgrCode::write_to_buf` | 24 |
| `esc_sequence.rs` | `EscSequence::write_to_buf` | 1 |
| `dsr_sequence.rs` | `DsrSequence::write_to_buf` | 1 |
| `cli_text.rs` | `CliStyle` + `CliTextInline` + others | 4 |
| `sequence.rs` (CsiSequence) | `CsiSequence::write_to_buf` | 1 |
| `osc_codes.rs` (OscSequence) | `OscSequence::write_to_buf` | 1 |
| **Total** | | **32** |

### 4. Validation
- [ ] Run `./check.fish --check` to ensure type-safety.
- [ ] Run `./check.fish --test` to ensure ANSI output remains identical.
- [ ] Run `./check.fish --clippy` to ensure idiomatic code.

## Out of Scope

These generators are entirely DYNAMIC (all arms require runtime values) and only need `ok!()`
replacement, not constant optimization:

- **`CsiSequence`** (`protocols/csi_codes/sequence.rs`): 22 arms, all parameterized
  (row/col/count). Only `SaveCursor`/`RestoreCursor` are static but they already write
  just `CSI_START` + single char — minimal gain from a constant.
- **`OscSequence`** (`core/osc/osc_codes.rs`): 6 arms, all take runtime strings (titles,
  URIs) or dynamic values (percent).
- **`ansi_sequence_generator_output.rs::text_attributes()`**: Builds SGR from a dynamic
  `Vec<u16>` of codes. Cannot use static constants.
- **`ForegroundBasic`/`BackgroundBasic`** in `sgr_code.rs`: 32 color variants requiring
  runtime `ANSIBasicColor` dispatch. Per-color constants possible but low-priority.

## Example Refactoring

### Before (`sgr_code.rs`)
```rust
SgrCode::Bold => {
    buf.push_str(CSI_START);
    buf.push('1');
    buf.push(SGR_SET_GRAPHICS);
    Ok(())
}
```

### After (`sgr_code.rs`)
```rust
SgrCode::Bold => {
    buf.push_str(SGR_BOLD_STR);
    ok!()
}
```
