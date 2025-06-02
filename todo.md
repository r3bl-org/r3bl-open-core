# fix md parser: https://github.com/r3bl-org/r3bl-open-core/issues/397

- [x] document naming convention:
  - `parse_*()` -> splits bytes from input into remainder and output bytes
  - `*_extract` -> generates structs from already-split-input using a `parse_*()`
  - `*_parser()` -> function that recieves an input and is called by `parse_*()`
- [x] `AsStrSlice::extract_remaining_text_content_in_line()`: fix the naming and add fn docs
- code block
- [x] `parse_code_block_generic()`: fix the behavior when end maker "```" is missing, add fn docs
- [x] `extract_code_block_content()`: fix this so it doesn't return `String`, only `&str`

- use OG parsers _dump the new AI generated stuff_

  - [x] `parse_unique_kv_opt_eol_generic()`: copy from `parse_unique_kv_opt_eol()`
  - [x] `parse_csv_opt_eol_generic()`: copy from OG `parse_csv_opt_eol()`

- GStringSlice -> AsStrSlice

  - [x] change `fn parse_markdown_alt()`, from: `&'a [GCString]`, to `input: impl Into<AsStrSlice>`
  - [x] generalize impl of `GCStringSlice` into `AsStrSlice` which implements `nom::Input`, drop the
        `Copy` requirement, and make `Clone` explicit

- use OG parsers _dump the new AI generated stuff_

  - [x] `parse_code_block_generic()`: fix bug! strips
        "```" from start and end and `test_parse_block_code_with_missing_end_marker()`
  - [x] `extract_code_block_content()`: test this
  - [x] add tests for `extract_code_block_content()`

- smart list

  - [x] `parse_block_smart_list_generic()`: copy this from OG
        `parse_block_smart_list.rs::parse_block_smart_list()`
  - [x] `parse_smart_list_and_extract_ir_generic()`: copy this from OG
        `parse_block_smart_list.rs::parse_smart_list()`:
  - [x] Remove all `Box::leak()`

- use OG parsers _dump the new AI generated stuff_

  - [x] `parse_block_heading_generic()`: change signature to use `AsStrSlice` and not `nom::Input`
  - [ ] `parse_block_text_generic()`: copy from OG `???`
  - [ ] stop using the generic function signature, and use this:

- [ ] use the correct method signatures for (most) parsers

  ```rust
  fn f<'a>(i: AsStrSlice<'a>) -> IResult<AsStrSlice<'a>, AsStrSlice<'a>> {}
  ```

  instead of:

  ```rust
  fn f<'a, I>(input: I) -> IResult<I, I>
  where
        I: Input + Clone + Compare<&'a str> + Offset + Debug,
        I::Item: AsChar + Copy
  {}
  ```

- address markdown parsing test failure

  - [ ] fix test: `test_parse_simple_markdown()`

- vec -> inlinevec

  - [ ] change all `Vec` to `InlineVec` in `parse_markdown_alt.rs`

- [ ] remove println!() except in tests
- [ ] fix clippy warnings
- [ ] add docs for everything
- [ ] Title
- [ ] Tags
- [ ] Authors
- [ ] Date
- [ ] Heading
- [ ] SmartList
- [ ] CodeBlock
- [ ] Text

- lines approach (discard?)

  - [ ] convert `VecEditorContentLines` into a newtype
  - [ ] impl `nom::Input` for `VecEditorContentLines`
  - [ ] need to change `try_parse_and_highlight()`?
  - [ ] change `parse_markdown()` (et al) so it can recieve something other than `&str`

- table

  - [ ] impl md table parser
  - [ ] impl syn hi for this

- fix "`rust`" parsing in syn hi code (should support both "rust" and "rs")

# modernize `choose` and `giti` codebase: https://github.com/r3bl-org/r3bl-open-core/issues/427

- [ ] use `InlineString` & `InlineVec` in `giti` codebase (for sake of consistency)
- [ ] fix clap args using `color_print::cstr` instead of directly embedding ansi escape sequences in
      the clap macro attributes `clap_config.rs`. see `rust_scratch/tcp-api-server` for examples
- [ ] make sure that analytics calls are made consistent throughout the giti codebase (currently
      they do nothing but this will get things ready for the new `r3bl_base` that will be self
      hosted in our homelab); currently `delete.rs` has analytics calls
- [ ] rewrite `giti` code to use the newtypes, like width, height, etc. and introduce newtypes, etc
      where needed

# minor perf in `tui` and `edi`: https://github.com/r3bl-org/r3bl-open-core/issues/428

- [ ] replace `HashMap` with `BTreeMap` (better cache locality performance). `HashMap` is great for
      random access, `BTreeMap` is good for cache locality and iteration which is the primary use
      case for most code in `r3bl_open_core` repo
- [ ] add fps counter row to bottom of `edi`, just like in the `tui/examples/demo/*` add an
      FPS/telemetry display to bottom of `edi`

# make release of `r3bl-cmdr` and `r3bl_tui`

- [ ] make sure `cmdr` docker file works (with `pkg-config` and `libssl-dev` removed):
      https://github.com/r3bl-org/r3bl-open-core/issues/426
- [ ] release `r3bl_tui`, `r3bl_cmdr`: https://github.com/r3bl-org/r3bl-open-core/issues/429
- [ ] close this: https://github.com/r3bl-org/r3bl-open-core/issues/391

# create sub-issues for `giti`: https://github.com/r3bl-org/r3bl-open-core/issues/423

- [ ] `giti branch rename` -> rename an existing branch to other
- [ ] `giti show <name>` -> choose an older version of a file to checkout to `Downloads` and
      optionally view in the `editor_component` itself in a TUI applet
- [ ] `giti develop *` -> choose issues using TUI app as part of the flow
- [ ] `giti commit`
- [ ] `giti delete *` -> switch to main and pull (delete remotes)
- [ ] `giti --manual` -> show the user guide for giti using the TUI MD component w/ search, jump to
      headings, etc

# test `giti` user flow

- [ ] devise an approach to do this
