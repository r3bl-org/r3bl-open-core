<!-- Oldest tasks are on top, most recently completed tasks on the bottom -->

# tuify

- [x] enhance `run examples` by presenting user with list to select from
- [x] impl `tuify/examples/choose_async.rs` that calls `tuify/src/public_api.rs::choose()`
- [x] move `terminal_async/src/public_api/styling.rs` into tuify (replace the old style code)
- [x] remove the following files/folders:
  - [x] `terminal_async/src/public_api/choose.rs`
  - [x] `terminal_async/src/public_api/styling.rs`
  - [x] `terminal_async/src/choose_impl/`

# terminal_async

- [x] Fix `run` script to show menu for all examples in this crate.
- [x] Ensure that the `terminal_async/examples/choose.rs` example pauses `ReadlineAsync` async
      stdout
- [x] Add test for paused `SharedWriter` in `tuify/src/public_api.rs::choose()`
- [x] remove all the `00: ` todos

# move tuify & terminal_async into tui

- [x] rename `select*` -> `choose()`
- [x] rename `choose()` -> `choose_async()`
- [x] move `tuify` into `terminal_async` (now called `choose`, with both sync and async variants)
- [x] archive `tuify`, update CHANGELOG

# move terminal_async into tui

- [x] move `terminal_async` into `tui` (now called `readline_async`)
- [x] merge all the examples both crates into the `tui` crate & update the main example runner
- [x] archive `terminal_async`, update CHANGELOG

# clean up redundant code in terminal_async (from tuify)

- [x] remove duplicate `keypress` mod (`key_press!`, `*KeyPress`)
- [x] remove duplicate use of colors in `choose_constants.rs` and `color_constants.rs`
- [x] remove duplicate code blocks in `tui/src/terminal_async/choose_impl/event_loop.rs`
- [x] remove duplicates `rgb_value!` and `tui_color!` (`tui_color` is the primary)
- [x] standardize error handling in `enter_event_loop_sync()` and `enter_event_loop_async()`;
- [x] use `return_if_not_interactive_terminal!()` consistently
- [x] remove `DefaultColors` and `cmdr/src/color_constants.rs` file

# investigate timing issues with SharedWriter flush

- [x] the patch to workaround is in `examples/choose_sync_and_async.rs` which does a sleep before
      the main function exits
- [x] perhaps it is necessary to put in some logic when the main event loops break to flush all
      buffers.
- [x] why does SharedWriter need to look for `'\n` before it writes / flushes buffers

# clean up output from ReadlineAsync

- [x] the last line of output has a prompt + "\n" ... this should be removed.

# remove Reedline from giti and replace with readline_async

- [x] remove Reedline from `cmdr/src/giti/branch/new.rs`
- [x] remove `reedline` from `cmdr/Cargo.toml`
- [x] remove the use of sync `choose()` from `giti`, replace with `choose_async()`

# clean up names from changed crate names

- [x] rename `terminal_async` -> `readline_async`

# clean up giti phase 1

- [x] make `AnsiStyledText` own the `text`, this is just a more ergonomic and useful API than using
      a `&str` which introduces needless lifetimes and lots of other unergonomic code writing, which
      makes this a cumbersome API to use
- [x] make `ui_templates.rs` fns return `Header` instead of
      `InlineVec<InlineVec<ASTStyledText<'_>>`. Fix all callers so they don't need `_binding*`
      anymore to the underlying text
- [x] rewrite the `TuiStyle` by removing `bool` and use `Option<T>` where `T` are concrete marker
      types
- [x] refactor `giti` code to be more readable (remove hard coded strings, make smaller functions)
  - [x] checkout.rs
  - [x] new.rs
  - [x] delete.rs
- [x] add `XMARK` for `bool -> Option<T>` code in `TuiStyle`
- [x] move all the colors in `choose` style to `tui_color!`

# update all deps

- [x] upgrade all the deps to the latest versions in all Cargo.toml files

# clean up giti phase 2

- [x] replace `SuccessReport` with an enum of valid variants (including user pressed `Ctrl+C`)
- [x] change how errors are reported using `miette`
- [x] collect all the git commands in a single module `git.rs`

# clean up giti phase 3

- [x] introduce consistent error reporting, and output handling using `CompletionReport`. there is
      no need for individual subcommands to do something specific to report command success, not
      success, or failure to run command. centralize this and simplify ALL subcommands, and make it
      easy to perform logging and analytics reporting.
- [x] replace `UIStrings` enum
  - [x] with simple functions
  - [x] consider moving this functionality into `impl Display` for `CommandExecutionReport`, instead
        of `giti.rs` -> `display_command_run_result()`
- [x] make `git.rs` use `InlineString` and `ItemsOwned` consistently. provide function arguments
      that can be converted to these easily. use `String` everywhere, except for interfacing with
      `choose` and then convert `ItemsOwned` to `String` and `Vec<String>`

# rewrite ItemsOwned to make choose() API simple to use

- [x] Remove ItemsBorrowed, and rewrite and radically simplify `ItemsOwned` and `choose()` API so
      that it is easy to use.

# disable github actions from the repo

- [x] undo all the github actions so they no longer run automatically
- [x] create a github hook to run `nu run all` maybe?
- [x] update all the deps for the crates in the workspace

# clean up giti phase 4

- [x] fix `show_exit_message()` does not appear all the time
- [x] in `git.rs` use `r3bl_script` to run commands (and not directly using `Command::new`)
- [x] in `Display` impl of `CommandRunResult` don't print everything, write some items log (eg:
      `CommandRunDetails`, etc.); does this need to be in `r3bl_script`?

# remove `r3bl_core` as a top level crate

- [x] rename all the `run` nushell script files to `run.nu` so that syn-hi works in rustrover
- [x] move this code into `r3bl_tui`
- [x] update docs for the `r3bl_tui` crate (`mod.rs`, `lib.rs`); the `README.md` files are generated
      from these. make the top level docs "mental model" level, and leave the specifics to each
      underlying mod.
  - [x] `README.md`
  - [x] `tui/src/lib.rs`
  - [x] `cmdr/src/lib.rs`
- [x] update `CHANGELOG.md` and move `r3bl_core` to archive section
- [x] deprecate the `r3bl_core` crate & move to `/home/nazmul/github/r3bl-open-core-archive`,
- [x] update all the projects in `rust_scratch` to work with the `tuifyasync` branch
  - [x] `tls`
  - [x] `tcp-api-server`

# refactor protocol.rs out of `tcp-api-server` into `r3bl_tui`

- [x] refactor and move `protocol.rs` into `r3bl_tui`, but keep the specific server in
      `tcp-api-server`, which showcases how this can be reused.
- [x] remove `use crossterm::style::Stylize;` from `tcp-api-server`

# use `jemalloc` in `r3bl_tui` and `rust_scratch/tcp-api-server`

- [x] use `jemalloc` in `r3bl-cmdr` and all the examples in `r3bl_tui`
- [x] use `jemalloc` in `rust-scratch/tcp-api-server`

# clean up jank in `readline_async`

- [x] in `giti branch delete` you can really see the jank caused by the cursor moving across the
      long prompt. clean this up and adjust all the existing examples to reflect this change.

# merge tuifyasync branch into main

- [x] crate a PR for tuifyasync & merge it into main
- [x] update all the projects in `rust_scratch` to work with latest github version of `r3bl_tui`
  - [x] `tls`
  - [x] `tcp-api-server`

# clean up giti phase 5

- [x] reorder `giti` commands so that `checkout` is first (not `delete`)
- [x] fix `giti` output https://github.com/r3bl-org/r3bl-open-core/issues/418

# clean up giti phase 6

- [x] use newtype pattern to make sense of how git commands produce branches so that current branch
      and not-current branches are represented naturally. there can be a conversion from
      "(current-branch, branches)" into some struct that can implement `Display`, and transform from
      UI selections to this struct.
- [x] decide which string to use for `CURRENT_PREFIX` = `(◕‿◕)`
- [x] fix `giti` ux https://github.com/r3bl-org/r3bl-open-core/issues/419

# clean up giti phase 7

- [x] fix `giti branch delete <branch-name>` which currently does not work since this command
      ignores branches that are passed as a command line arg

# clean up giti phase 8

- [x] use crates.io api to check for latest release to show upgrade message for `edi` and `giti`
      https://github.com/r3bl-org/r3bl-open-core/issues/415
- [x] Test this by changing the local version number so it's different from the crates.io version
      for r3bl-cmdr `cmdr/src/analytics_client.rs:308`

# fix cargo install without needing libssl-dev and pkg-config

- [x] installing `r3bl-cmdr` on a new VM / machine requires `libssl-dev` to be installed. fix
      `reqwest` so it uses `rustls` and not `openssl`

# clean up giti phase 9

- [x] make `analytics_client.rs` its own module since it has to much code inside of it
- [x] move all the ui strings into a module, so they're not defined one-off / inline.
- [x] fix single and multiselect instruction formatting for `choose()` call sites & ensure they're
      used everywhere: `cmdr/src/giti/ui_templates.rs:24`

# clean up giti phase 10

- [x] remove all the leading space from each ui string for `giti` and `edi`
  - [x] `cmdr/src/analytics_client/ui_str.rs`
  - [x] `cmdr/src/giti/ui_templates.rs:24`
  - [x] `giti/ui_str.rs`
- [x] introduce consistent imperative formatting for `giti` and `edi`
- [x] fix `giti branch checkout`

# clean up giti phase 11

- [x] rename `AST` -> `ASText`, etc.
- [x] evaluate the use of `AST[]` -> `PixelChar[]` which can then be clipped for `readline_async`
      prompt, `spinner`, and `choose` display. not all `ui_str::*` functions have to be changed,
      just the ones that are related to the prompt and spinner displays
  - `tui/src/core/tui_core/spinner_impl/spinner_render.rs:58`
  - `tui/src/readline_async/readline_async_api.rs:121`
  - [issue](https://github.com/r3bl-org/r3bl-open-core/issues/420)
- [x] `tui/src/core/tui_core/spinner_impl/spinner_render.rs:58`
  - move this into the constructor
  - add debug_assert! to ensure no ANSI esc seq in message

# clean up `edi.rs`

- [x] `cmdr/src/bin/edi.rs:113`
  - [x] clean up the module organization (many things need to be moved out of `bin/edi.rs`)
  - [x] remove all the leading space from each ui strings

# Ctrl+C during upgrade run of `cargo install r3bl-cmdr`: https://github.com/r3bl-org/r3bl-open-core/issues/424

- [x] `fn install_with_spinner()`:
  - `cmdr/src/analytics_client/upgrade_check.rs:162`
  - `tui/examples/spinner.rs:33`
  - figure out what to do with `Ctrl+C` pressed in `giti` when `cargo install ...` is being run as
    part of upgrade
  - is this a special use case for the spinner? if so, consider adding functionality that is not
    tied to `readline_async` for this type of "blocking" use case
- [x] `tui/src/readline_async/spinner.rs:139` when `maybe_shared_writer` is None
  - try_start: enable raw mode, hide cursor
  - stop: disable raw mode, show cursor

# Fix lifecycle shutdown issues for async structs like Spinner and AsyncReadline

- [x] add `async fn wait_for_shutdown()` to `Spinner` for better testability
- [x] add `async fn wait_for_shutdown()` to `ReadlineAsync` similar to `Spinner`

# test `giti` thoroughly: https://github.com/r3bl-org/r3bl-open-core/issues/425

- [x] manual testing
- [x] use `r3bl_script` test fixtures to test `git.rs` commands

# make release of `r3bl-cmdr` and `r3bl_tui`

- [x] make sure `cmdr` docker file works (with `pkg-config` and `libssl-dev` removed):
      https://github.com/r3bl-org/r3bl-open-core/issues/426
- [x] release `r3bl_tui`, `r3bl_cmdr`: https://github.com/r3bl-org/r3bl-open-core/issues/429
- [x] close this: https://github.com/r3bl-org/r3bl-open-core/issues/391

# fix md parser: https://github.com/r3bl-org/r3bl-open-core/issues/397

- [x] document naming convention:
  - `parse_*()` -> splits bytes from input into remainder and output bytes
  - `*_extract` -> generates structs from already-split-input using a `parse_*()`
  - `*_parser()` -> function that receives an input and is called by `parse_*()`
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

- [x] fix regression bug in `md_parser` when using `AsStrSlice::to_string()`. see
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

- [x] migrate `block` parsers into `block_alt` (part 2)

  - block parsers:

    - [x] `parse_block_code.rs` -> `block_alt/parse_block_code_alt.rs`

    - [x] `parse_block_smart_list.rs` -> `block_alt/parse_block_smart_list_alt.rs`
      - [x] `parse_smart_list_content_lines_alt()`
      - [x] `mod tests_parse_smart_list_content_lines_alt`
        - [x] `parse_smart_list_alt()`
        - [x] review implementation and see if `digit1` and other parts can be refactored into small
              functions
        - [x] `mod tests_parse_smart_list_alt`
      - [x] `mod tests_bullet_kinds`
      - [x] `parse_block_smart_list_alt()`
      - [x] `mod tests_parse_block_smart_list`

- [x] migrate `md_parser/parse_markdown()` -> `md_parser_alt/parse_markdown_alt()`

  - [x] review `as_str_slice.rs` changes
  - [x] review this fix `fallback_parse_any_line_as_plain_alt()`
  - [x] break up `as_str_slice.rs` into `as_str_slice_mod`
  - [x] check performance of block code parser which materializes strings to look for enclosing
        three-backticks. this is avoidable, by using `lines` to check which ones only contain
        three-backticks and keep track of how many lines are involved, and then convert them into
        start and end char indices for the `AsStrSlice`
  - [x] do the same performance check (don't materialize) for the block smart list parser
  - [x] check `AsStrSlice::find_substring()` for performance penalty when looking ahead to find
        something in a really large `lines`. might be able to speed up this search using `line` and
        some match to calculate the "expected byte index"
  - [x] continue migrating all the remaining tests into this file from `parse_markdown.rs`

- [x] rename `md_parser_alt` to `md_parser_ng`; all `_alt` -> `_ng`

  - [x] remove all compiler warnings
  - [x] extract 1 common function between both legacy and ng parser (used in block code parser)
  - [.] create compat tests between legacy and ng parser
    - [x] fix `find_substring()` which wasn't handling unicode correctly leading to failure of
          `InlineCode` parsing of "`foo💕bar`" in `parse_inline_fragments_until_eol_or_eoi_ng()`
    - [x] make sure that the new trailing `NEW_LINE` generation of the `_ng` parser is respected in
          the `compatibility.rs` tests

- [x] update `md_parser_syn_hi_impl.rs` to use this. provide a new `bool` flag that allows the new
      `_ng` parser active instead of the old one (keep them all in the code for now). there is 1
      function that is shared between the two, so move that to `_ng` for smart code block tests.
- [x] add a new test case in `parse_heading_ng()` test cases, and use the markdown constant from the
      demo examples for the editor, which currently shows up in 2 places (consolidate them into
      one):
  - `get_real_world_content()` in
    `tui/src/tui/md_parser_ng/as_str_slice/compatibility_test_suite.rs`
  - `get_default_content()` in `tui/examples/tui_apps/ex_editor/state.rs`
- [x] make quality and compatibility improvements to `md_parse_ng` now that it is attached to the
      test examples. verify that extra line at the bottom of editor example shows up both for legacy
      and NG parser
- [x] deal with compiler warning (turned error) affecting the location of the
      `as_str_slice_test_case!`
- [x] fix all the cargo clippy warnings generated by this command
      `cargo clippy -- -W clippy::all -W clippy::pedantic -W clippy::nursery`

- [x] spend some time with `cargo clippy`

  - [x] add clippy related commands to `run.nu` (top-level) and figure out what they should be:
        `clippy-basic-fix`, `clippy-pedantic-fix`?
  - [x] figure out how to add some lint checks that are being fixed in the `lib.rs` in the first
        place so they are caught every time that clippy runs; this prevents having to do massive
        clean up after the fact:
    - `clippy::doc_markdown`
    - `clippy::redundant_closure`
    - `clippy::redundant_closure_for_method_calls`
    - `clippy::cast_sign_loss`
    - `clippy::cast_lossless`
    - `clippy::cast_possible_truncation`
    - `clippy::semicolon_if_nothing_returned`
    - `clippy::must_use_candidate`
    - `clippy::items_after_statements`

- [x] use `cargo bench` in `compatibility_test_suite.rs` to compare the relative performance of both
      legacy and NG parsers
- [x] run more test with more complex documents in the `compatibility_test_suite.rs` to ensure that
      large READMEs, journal entries, and other complex and long MD files can be parsed correctly.
      might be nice to benchmark these too (og vs ng). and save these MD files in the project itself
      in the tests cases folder or something. this would be beyond the compatibility test suite, and
      should be some kind of `markdown_verification_test_suite` for quality.
- [x] would it be possible to cache the AST returned by `parse_markdown_ng()`? is this a tree
      structure or an array structure? how to mark areas as dirty? how to reparse only sections that
      have changed? this might speed up parsing a whole lot, if the entire thing does not have to be
      re-parsed? need to verify all of this and not just use intuition.
- [x] perf optimize codebase using flamegraph profiling & claude using `docs/ng_parser.md`
  - [x] try incorporate memoized size calc in `GetMemSize`
- [x] try and fix NG parser
  - [x] identified O(n) character counting bottlenecks in `AsStrSlice` hot paths
  - [x] implemented caching infrastructure for character counts and byte offsets
  - [x] optimized `extract_to_line_end()` to use cached byte offsets
  - [x] fixed O(n) loop in `take_from()` with binary search
  - [x] implemented lazy cache initialization to reduce overhead
  - [x] fixed cache sharing bug when cloning `AsStrSlice`
  - [x] fixed position tracking bug in `skip_take_in_current_line()`
  - [x] achieved 600-5,000x performance improvement (from 50,000x to 9-83x slower)
- [x] incorporate both NG and legacy parsers into the `try_parse_and_highlight()`
  - [x] implemented hybrid parser approach with 100KB threshold
  - [x] documents ≤100KB use legacy parser (better performance)
  - [x] documents >100KB use NG parser (better memory efficiency)
  - [x] removed `ENABLE_MD_PARSER_NG` constant in favor of dynamic selection
  - [x] added comprehensive documentation to `try_parse_and_highlight()`
- [x] updated `docs/ng_parser.md` with detailed performance optimization findings
- [x] moved regression tests from separate file to `parse_fragments_in_a_line_ng.rs`
- [x] create a simple parser intended to replace both legacy (nom) and NG (hybrid) parsers. use
      `docs/ng_parser_simple_drop_nom.md` as the planning and progress tracking document for this
      task.
- [x] ensure that we have snapshot testing capability for comparing output of valid MD content,
      rather than relying on comparing the output of one parser to the others to verify that the
      output is correct. i know that we do the comparison tests for compat between the parsers, but
      i just want to audit the individual legacy parser tests to ensure that they have full coverage
      of all the `compat_test_data` files.
- [x] archive the NG parser and the simple parser, to the `r3bl-open-core-archive` repo as a new
      crate called `md_parser_ng`. keep the legacy parser as the main one. use the
      `docs/ng_parser_archive.md` as the planning and progress tracking document for this task.

# clean up and release: https://github.com/r3bl-org/r3bl-open-core/issues/397

- [x] fix all the lints after the extraction & archival of the `md_parser_ng`
- [x] create parser conformance snapshot test, and make sure they pass
      [docs/parser_conformance.md](docs/done/task_parser_conformance.md)
- [x] review the flamegraph.svg and cargo bench results to ensure no regressions
      [docs/task_tui_perf_optimize.md](docs/task_tui_perf_optimize.md)
- [x] complete the performance work started in
      [task_tui_perf_optimize](docs/task_tui_perf_optimize.md)
- [x] fix windows bug: https://github.com/r3bl-org/r3bl-open-core/issues/433
- [x] refactor `md_parser` with consistent naming and module organization
- [x] add missing tests to `editor` module
- [x] fix copy/paste bugs in `editor` module (support bracketed paste mode too)
- [x] there are test failures in doctests that try to use terminal I/O (which fails in test
      environment). can you identify and mark them to be "```no_run"
- [x] fix all the pedantic lints using claude (and don't allow them anymore in Cargo.toml)
- [x] update changelog
- [x] rebase `fix-md-parser` on to `main`. push it to remote `origin`. fixes:
      <https://github.com/r3bl-org/r3bl-open-core/issues/397>. link to this PR:
      <https://github.com/r3bl-org/r3bl-open-core/pull/430>
- [x] fix "`rust`" parsing in syn hi code (should support both "rust" and "rs"), and other
      extensions like "`ts`", etc.
- [x] make a release using the [`release-guide.md`](docs/release-guide.md) document as a guide -
      <https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.0.20-cmdr> -
      <https://github.com/r3bl-org/r3bl-open-core/releases/tag/v0.7.2-tui>

# editor content storage enhancements: https://github.com/r3bl-org/r3bl-open-core/issues/387

- [x] use [`task_zero_copy_gap_buffer`](docs/done/task_zero_copy_gap_buffer.md) to implement a gap
      buffer for the editor content storage. This will improve performance and memory usage when
      editing large files.
  - The summary highlights:
    - 2-3x overall application performance improvement
    - Complete elimination of major bottlenecks (100% reduction each)
    - Significant reductions in other bottlenecks (27-89% improvements)
    - ZeroCopyGapBuffer's specific achievements including zero-copy access and 50-90x faster append
      operations
    - ~88.64% of total execution time eliminated from the top 5 bottlenecks
    - Real-world impact on parser performance, editor responsiveness, memory usage, and large
      document handling
    - Current well-balanced performance profile with overhead only in necessary areas
