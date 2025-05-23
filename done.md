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

- [x] installing `r3bl-cmdr` on a new VM / machine requies `libssl-dev` to be installed. fix
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
