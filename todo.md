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
  - [x] `parse_block_heading_generic()`: change signature to use `AsStrSlice` and not `nom::Input`

- smart list

  - [x] `parse_block_smart_list_generic()`: copy this from OG
        `parse_block_smart_list.rs::parse_block_smart_list()`
  - [x] `parse_smart_list_and_extract_ir_generic()`: copy this from OG
        `parse_block_smart_list.rs::parse_smart_list()`:
  - [x] Remove all `Box::leak()`

- [x] revert "parse by line" approach to rewriting the parser, use `nom::Input` & `AsStrSlice`
      instead

- [x] make naming convention consistent, use `_alt`, drop `_generic`

- [x] use the correct method signatures for (most) parsers

  ```rust
  fn f<'a>(input: AsStrSlice<'a>) -> IResult</* rem */ AsStrSlice<'a>, /* output */ AsStrSlice<'a>> {}
  ```

  instead of:

  ```rust
  fn f<'a, I>(input: I) -> IResult</* rem */ I, /* output */ I>
  where
        I: Input + Clone + Compare<&'a str> + Offset + Debug,
        I::Item: AsChar + Copy
  {}
  ```

- [x] create new package `md_parser_alt`

- [x] use [`mimalloc` crate](https://docs.rs/mimalloc/latest/mimalloc/) to replace `jemalloc` w/
      microsoft [`mimalloc`](https://github.com/microsoft/mimalloc?tab=readme-ov-file#performance).
      [jemalloc](https://github.com/jemalloc/jemalloc) is archived.
      [more info on best rust allocators](https://gemini.google.com/app/e4979f6a69f5f9e5)

- [x] fix flaky `script` tests which change the cwd, all of them need to be run in a single function
      so there are no issues with multiple tests processes `serialtest` does not seem to address
      this issue. test file: `tui/src/core/script/fs_path.rs:434:5`

  - `core::script::fs_path::tests::test_try_directory_exists_not_found_error`
  - `core::script::fs_path::tests::test_try_directory_exists_permissions_errors`
  - `core::script::fs_path::tests::test_try_file_exists`
  - `core::script::fs_path::tests::test_try_file_exists_invalid_name_error`
  - `core::script::directory_create::tests_directory_create::test_try_mkdir`
  - `core::script::fs_path::tests::test_try_file_exists_permissions_errors`
  - `core::script::fs_path::tests::test_try_pwd`
  - `core::script::fs_path::tests::test_try_pwd_errors`
  - `core::script::fs_path::tests::test_try_write`

- [ ] fix regression bug in `md_parser` when using `AsStrSlice::to_string()`. see
      `EditorBuffer::new_empty()` & `EditorBuffer::set_lines()` for how "raw" data is loaded into
      the model in memory (from disk or new / empty).

  - [x] `impl Display for AsStrSlice` address all `FIXME: proper \n handling`
  - [x] test `test_parse_fragment_plaintext_unicode()` in
        `tui/src/tui/md_parser_alt/fragment_alt/parse_fragments_in_a_line_alt.rs:201` should PASS
  - [x] `extract_remaining_text_content_in_line()` in
        `tui/src/tui/md_parser_alt/string_slice.rs:241`: should NOT do anything with `NEW_LINE`
  - [x] fix `test_parse_fragment_markdown_inline()` and
        `parse_inline_fragments_until_eol_or_eoi_alt()`

- [x] migrate `atomics` parsers into `atomics_alt`

  - [x] `take_text_between.rs` -> `fragment_alt/take_text_between_alt.rs`: only used by
        `specialized_parsers_alt.rs`. fix tests & check logic; add more tests to `AsStrSlice` based
        on this

- [x] migrate `fragment` to `fragment_alt` mod

  - [x] `specialized_parsers.rs` -> `specialized_parsers_alt.rs`: fix tests & check logic
    - [x] `parse_fragment_starts_with_checkbox_checkbox_into_bool_alt()`
    - [x] `parse_fragment_starts_with_checkbox_into_str_alt()`
    - [x] `parse_fragment_starts_with_left_link_err_on_new_line_alt()`
    - [x] `parse_fragment_starts_with_left_image_err_on_new_line_alt()`
    - [x] `parse_fragment_starts_with_backtick_err_on_new_line_alt()`
    - [x] `parse_fragment_starts_with_star_err_on_new_line_alt()`
    - [x] `parse_fragment_starts_with_underscore_err_on_new_line_alt()`
  - [x] `specialized_parser_delim_matchers.rs` -> `specialized_parsers_alt.rs`: add tests
  - [x] `parse_fragments_in_a_line_alt.rs` => `test_parse_fragment_plaintext_unicode()`: something
        is wrong with synthetic `\n` handling
  - [x] what are semantics of `AsStrSlice::to_string()`? how does the `.to_string()` handle `\n`?
        this is making the "old" `parse_markdown()` fail silently when running the examples

- [x] fix regression in old code when starting to incorporate `AsStrSlice`

  - [x] `try_parse_and_highlight()` in
        `tui/src/tui/syntax_highlighting/md_parser_syn_hi/md_parser_syn_hi_impl.rs:141`: fix
        `write_to_byte_cache()` to behave in "legacy" or "compat" mode to be compatible w/ the old
        style (before `_alt`) markdown parser behavior
  - [x] why `AsStrSlice::max_len`? for `take()` to work!

- [x] break compat with `slice::lines()` in `AsStrSlice` and do it the "intuitive way". this means
      adding a new line at the end of the output when there is more than 1 line in `Self::lines` and
      the last line is empty. update docs and tests to match.

- [x] unicode bug is present in `Display` impl of `AsStrSlice`! this is what breaks the examples!
      when switching the implementation of `write_to_byte_cache_compat()` to use `Display` impl the
      bug can be seen in the tui/examples#3! revert back to the simplistic way of creating the
      output `&str` and the example works again.

  - [x] add test cases to repro the bug: `test_input_contains_emoji()` & fix it
  - [x] update comments to clarify the emoji handling logic in the `AsStrSlice` docs, and document
        the field `byte_index` and make it clear that byte indexing is used and not char indexing
  - [x] make sure there are tests which compare the `Display` impl to the
        `write_to_byte_cache_compat()` and ensure they produce the same output

- [x] migrate `atomics` parsers into `extended_alt`

  - [x] `take_text_until_eol_or_eoi.rs` -> `extended_alt/take_text_until_eol_or_eoi_alt.rs`: simply
        replace `&'a str` with `AsStrSlice<'a>`

- [x] delete `parser_impl.rs` (was only needed for experimentation)

- [x] migrate `fragment` to `fragment_alt` mod

  - [x] rename `string_slice.rs` -> `as_str_slice.rs`
  - [x] `plain_parser_catch_all.rs` -> `plain_parser_catch_all_alt.rs`: fix bugs & add tests
  - [x] `parse_fragments_in_a_line.rs` -> `parse_fragments_in_a_line_alt.rs`: fix bugs & add tests

- [x] refactor / rewrite `as_str_slice.rs` using `Length`, `Index`, `BoundsCheck` and lots of enums
      for state machine states and functions to calculate these

- [ ] migrate `extended` parsers into `extended_alt`

  - [x] `parse_metadata_k_csv_alt`

    - [x] migrate the functions `parse_csv_opt_eol_alt` and `parse_comma_separated_list_alt` over
      - [x] don't use `&str` anymore! implement lots of new traits for `AsStrSlice` for `Compare`
            for nom `tag` integration
      - [x] implement `From<InlineVec<AsStrSlice<'a>>> for List<AsStrSlice<'a>>` for `List` and
            `InlineVec` integration
    - [x] migrate all the existing tests over

  - [x] `as_str_slice.rs` clean up

    - [x] `extract_remaining_text_content_in_line()` -> `extract_to_line_end()`
    - [x] `extract_remaining_text_content_to_end()` -> `extract_to_slice_end()`
    - [x] `is_empty()`
    - [x] `contains()`
    - [x] `starts_with()`

  - [x] clean up messy test case input data creation using `as_str_slice_test_case!`

  - [x] `parse_metadata_k_v_alt`

  - [x] make sure all the docs have been copied over along with any `println!()` statements at the
        start of main parser function execution

- [x] should there be `block` and `line` parsers? if so, should `extract_to_line_end()` be used
      exclusively by `line` parsers? also who should use `extract_to_slice_end()`

- [x] migrate `block` parsers into `block_alt` (part 1)

  - not block parsers (in `standard_alt`):

    - [x] `parse_block_heading.rs` -> `parse_heading_alt.rs`
    - [x] `parse_block_markdown_text_until_eol_or_eoi.rs`

      - [x] `parse_block_markdown_text_with_checkbox_policy_with_or_without_new_line()` ->
            `parse_block_smart_list_alt.rs`
            `parse_markdown_text_with_checkbox_policy_until_eol_or_eoi_alt()`

      - [x] `parse_block_markdown_text_with_or_without_new_line()` ->
            `parse_markdown_text_including_eol_or_eoi_alt.rs`
            `parse_markdown_text_including_eol_or_eoi_alt()`

      - [x] Rename `MdBlock` -> `MdElement`

---

- [ ] migrate `block` parsers into `block_alt` (part 2)

  - block parsers:

    - [x] `parse_block_code.rs` -> `block_alt/parse_block_code_alt.rs`

    - [ ] `parse_block_smart_list.rs` -> `block_alt/parse_block_smart_list_alt.rs`

      - [x] `parse_smart_list_content_lines_alt()`
      - [x] `mod tests_parse_smart_list_content_lines_alt`
      - [ ] `mod tests_bullet_kinds`
      - [ ] `parse_smart_list_alt()`
      - [ ] `parse_block_smart_list_alt()`
      - [ ] `mod tests_parse_list_item`
      - [ ] `mod tests_parse_indents`
      - [ ] `mod tests_parse_block_smart_list`
      - [ ] `mod tests_parse_smart_lists_in_markdown`

- [ ] migrate `md_parser/parse_markdown()` -> `md_parser_alt/parse_mardown_alt()`

- [ ] maybe check `AsStrSlice::find_substring()` for performance penalty when looking ahead to find
      something in a really large `lines`. might be able to speed up this search using `line` and
      some match to calculate the "expected byte index"

---

- [ ] remove `md_parser` mod and `md_parser_alt` mod is the new one; update
      `md_parser_syn_hi_impl.rs` to use this. provide a new `bool` flag that allows the new `_alt`
      parser active instead of the old one (keep them all in the code for now)

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
