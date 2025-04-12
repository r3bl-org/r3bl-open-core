# ✅ tuify

- [x] enhance `run examples` by presenting user with list to select from
- [x] impl `tuify/examples/choose_async.rs` that calls `tuify/src/public_api.rs::choose()`
- [x] move `terminal_async/src/public_api/styling.rs` into tuify (replace the old style code)
- [x] remove the following files/folders:
  - [x] `terminal_async/src/public_api/choose.rs`
  - [x] `terminal_async/src/public_api/styling.rs`
  - [x] `terminal_async/src/choose_impl/`

# ✅ terminal_async

- [x] Fix `run` script to show menu for all examples in this crate.
- [x] Ensure that the `terminal_async/examples/choose.rs` example pauses `ReadlineAsync` async
      stdout
- [x] Add test for paused `SharedWriter` in `tuify/src/public_api.rs::choose()`
- [x] remove all the `00: ` todos

# ✅ move tuify & terminal_async into tui

- [x] rename `select*` -> `choose()`
- [x] rename `choose()` -> `choose_async()`
- [x] move `tuify` into `terminal_async` (now called `choose`, with both sync and async variants)
- [x] archive `tuify`, update CHANGELOG

# ✅ move terminal_async into tui

- [x] move `terminal_async` into `tui` (now called `readline_async`)
- [x] merge all the examples both crates into the `tui` crate & update the main example runner
- [x] archive `terminal_async`, update CHANGELOG

# ✅ clean up redundant code in terminal_async (from tuify)

- [x] remove duplicate `keypress` mod (`key_press!`, `*KeyPress`)
- [x] remove duplicate use of colors in `choose_constants.rs` and `color_constants.rs`
- [x] remove duplicate code blocks in `tui/src/terminal_async/choose_impl/event_loop.rs`
- [x] remove duplicates `rgb_value!` and `tui_color!` (`tui_color` is the primary)
- [x] standardize error handling in `enter_event_loop_sync()` and `enter_event_loop_async()`;
- [x] use `return_if_not_interactive_terminal!()` consistently
- [x] remove `DefaultColors` and `cmdr/src/color_constants.rs` file

# ✅ investigate timing issues with SharedWriter flush

- [x] the patch to workaround is in `examples/choose_sync_and_async.rs` which does a sleep before
      the main function exits
- [x] perhaps it is necessary to put in some logic when the main event loops break to flush all
      buffers.
- [x] why does SharedWriter need to look for `'\n` before it writes / flushes buffers

# ✅ clean up output from ReadlineAsync

- [x] the last line of output has a prompt + "\n" ... this should be removed.

# ✅ remove Reedline from giti and replace with readline_async

- [x] remove Reedline from `cmdr/src/giti/branch/new.rs`
- [x] remove `reedline` from `cmdr/Cargo.toml`
- [x] remove the use of sync `choose()` from `giti`, replace with `choose_async()`

# ✅ clean up names from changed crate names

- [x] rename `terminal_async` -> `readline_async`

# 🚀 clean up giti

- [x] make `AnsiStyledText` own the `text`, this is just a more ergonomic and useful API than using
      a `&str` which introduces needless lifetimes and lots of other unergonomic code writing, which
      makes this a cumbersome API to use
- [x] make `ui_templates.rs` fns return `Header` instead of
      `InlineVec<InlineVec<ASTStyledText<'_>>`. Fix all callers so they dont need `_binding*`
      anymore to the underlying text
- [x] rewrite the `TuiStyle` by removing `bool` and use `Option<T>` where `T` are concrete marker
      types
- [x] refactor `giti` code to be more readable (remove hard coded strings, make smaller functions)
  - [x] checkout.rs
  - [x] new.rs
  - [x] delete.rs
- [ ] replace `SuccessReport` with an enum of valid variants (including user pressed ctrl+c)
- [ ] collect all the git commands in a single module `git.rs`
  - [ ] use `r3bl_script` to run commands (and not directly using `Command::new`)
- [ ] why does `show_exit_message()` not appear all the time
- [ ] make sure that analytics calls are made consistent throughout the giti codebase (currently
      they do nothing but this will get things ready for the new `r3bl_base` that will be self
      hosted in our homelab); currently `delete.rs` has analytics calls
- [ ] rewrite giti code to use the newtypes, like width, height, etc.
- [ ] use `r3bl_script` test fixtures to test `git` and `gh` adapters

# fix lib.rs / README.md

- [ ] main workspace remove all references to `r3bl_tuify` and `r3bl_terminal_async`, and all the
      other crates that have been archived
- [ ] `tui` folder do the same as above
- [ ] `core` folder do the same as above

# disable github actions from the repo

- [ ] undo all the github actions so they no longer run automatically
- [ ] create a github hook to run `nu run all` maybe?
- [ ] update all the deps for the crates in the workspace

# add fps counter row to bottom of edi

- [ ] just like in the `tui/examples/demo/*` add an FPS/telemetry display to bottom of edi

# merge tuifyasync branch into main

- [ ] upgrade all the deps to the latest versions in all Cargo.toml files
- [ ] crate a PR for tuifyasync & merge it into main

# remove `analytics_schema` as a top level crate

- [ ] move this code into `r3bl_core`
- [ ] deprecate the `analytics_schema` crate

# make a release

- [ ] `r3bl_core`
- [ ] `r3bl_tui`
- [ ] `r3bl_cmdr`

# create new issues & sub-issues for giti features

- [ ] `giti develop *` -> choose issues using TUI app as part of the flow
- [ ] `giti commit`
- [ ] `giti delete *` -> switch to main and pull (delete remotes)
