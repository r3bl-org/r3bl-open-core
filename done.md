# ✔ tuify

- [x] enhance `run examples` by presenting user with list to select from
- [x] impl `tuify/examples/choose_async.rs` that calls `tuify/src/public_api.rs::choose()`
- [x] move `terminal_async/src/public_api/styling.rs` into tuify (replace the old style code)
- [x] remove the following files/folders:
  - [x] `terminal_async/src/public_api/choose.rs`
  - [x] `terminal_async/src/public_api/styling.rs`
  - [x] `terminal_async/src/choose_impl/`

# ✔ terminal_async

- [x] Fix `run` script to show menu for all examples in this crate.
- [x] Ensure that the `terminal_async/examples/choose.rs` example pauses `ReadlineAsync` async
      stdout
- [x] Add test for paused `SharedWriter` in `tuify/src/public_api.rs::choose()`
- [x] remove all the `00: ` todos

# ✔ move tuify & terminal_async into tui

- [x] rename `select*` -> `choose()`
- [x] rename `choose()` -> `choose_async()`
- [x] move `tuify` into `terminal_async` (now called `choose`, with both sync and async variants)
- [x] archive `tuify`, update CHANGELOG

# ✔ move terminal_async into tui

- [x] move `terminal_async` into `tui` (now called `readline_async`)
- [x] merge all the examples both crates into the `tui` crate & update the main example runner
- [x] archive `terminal_async`, update CHANGELOG

# ✔ clean up redundant code in terminal_async (from tuify)

- [x] remove duplicate `keypress` mod (`key_press!`, `*KeyPress`)
- [x] remove duplicate use of colors in `choose_constants.rs` and `color_constants.rs`
- [x] remove duplicate code blocks in `tui/src/terminal_async/choose_impl/event_loop.rs`
- [x] remove duplicates `rgb_value!` and `tui_color!` (`tui_color` is the primary)
- [x] standardize error handling in `enter_event_loop_sync()` and `enter_event_loop_async()`;
- [x] use `return_if_not_interactive_terminal!()` consistently
- [x] remove `DefaultColors` and `cmdr/src/color_constants.rs` file

# ✔ investigate timing issues with SharedWriter flush

- [x] the patch to workaround is in `examples/choose_sync_and_async.rs` which does a sleep before
      the main function exits
- [x] perhaps it is necessary to put in some logic when the main event loops break to flush all
      buffers.
- [x] why does SharedWriter need to look for `'\n` before it writes / flushes buffers

# ✔ clean up output from ReadlineAsync

- [x] the last line of output has a prompt + "\n" ... this should be removed.

# ✔ remove Reedline from giti and replace with readline_async

- [x] remove Reedline from `cmdr/src/giti/branch/new.rs`
- [x] remove `reedline` from `cmdr/Cargo.toml`
- [x] remove the use of sync `choose()` from `giti`, replace with `choose_async()`

# ✔ clean up names from changed crate names

- [x] rename `terminal_async` -> `readline_async`

# ✔ clean up giti phase 1

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

# ✔ update all deps

- [x] upgrade all the deps to the latest versions in all Cargo.toml files

# ✔ clean up giti phase 2

- [x] replace `SuccessReport` with an enum of valid variants (including user pressed ctrl+c)
- [x] change how errors are reported using `miette`
- [x] collect all the git commands in a single module `git.rs`

# ✔ clean up giti phase 3

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

# ✔ rewrite ItemsOwned to make choose() API simple to use

- [x] Remove ItemsBorrowed, and rewrite and radically simplify `ItemsOwned` and `choose()` API so
      that it is easy to use.

# ✔ disable github actions from the repo

- [x] undo all the github actions so they no longer run automatically
- [x] create a github hook to run `nu run all` maybe?
- [x] update all the deps for the crates in the workspace

# ✔ clean up giti phase 4

- [x] fix `show_exit_message()` does not appear all the time
- [x] in `git.rs` use `r3bl_script` to run commands (and not directly using `Command::new`)
- [x] in `Display` impl of `CommandRunResult` don't print everything, write some items log (eg:
      `CommandRunDetails`, etc.); does this need to be in `r3bl_script`?
