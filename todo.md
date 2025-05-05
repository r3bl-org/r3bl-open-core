# Ctrl+C during upgrade run of `cargo install r3bl-cmdr`: https://github.com/r3bl-org/r3bl-open-core/issues/424

- [ ] `fn install_with_spinner()`:
  - `cmdr/src/analytics_client/upgrade_check.rs:142`
  - `tui/examples/spinner.rs:33`
  - figure out what to do with `Ctrl+C` pressed in `giti` when `cargo install ...` is being run as
    part of upgrade ;
  - is this a special use case for the spinner? if so, consider adding functionality that is not
    tied to `readline_async` for this type of "blocking" use case;
  - why does the cursor show while spinner is active (remove the cursor?)

# test `giti` thoroughly: https://github.com/r3bl-org/r3bl-open-core/issues/425

- [ ] manual testing
- [ ] use `r3bl_script` test fixtures to test `git` commands

# make release of `r3bl-cmdr` and `r3bl_tui`

- [ ] make sure `cmdr` docker file works (with `pkg-config` and `libssl-dev` removed):
      https://github.com/r3bl-org/r3bl-open-core/issues/426
- [ ] release `r3bl_tui`, `r3bl_cmdr`: https://github.com/r3bl-org/r3bl-open-core/issues/429
- [ ] close this: https://github.com/r3bl-org/r3bl-open-core/issues/391

# minor perf in `tui` and `edi`: https://github.com/r3bl-org/r3bl-open-core/issues/428

- [ ] replace `HashMap` with `BTreeMap` (better cache locality performance). `HashMap` is great for
      random access, `BTreeMap` is good for cache locality and iteration which is the primary use
      case for most code in `r3bl_open_core` repo
- [ ] add fps counter row to bottom of `edi`, just like in the `tui/examples/demo/*` add an
      FPS/telemetry display to bottom of `edi`

# modernize `choose` and `giti` codebase: https://github.com/r3bl-org/r3bl-open-core/issues/427

- [ ] use `InlineString` & `InlineVec` in `giti` codebase (for sake of consistency)
- [ ] fix clap args using `color_print::cstr` instead of directly embedding ansi escape sequences in
      the clap macro attributes `clap_config.rs`. see `rust_scratch/tcp-api-server` for examples
- [ ] make sure that analytics calls are made consistent throughout the giti codebase (currently
      they do nothing but this will get things ready for the new `r3bl_base` that will be self
      hosted in our homelab); currently `delete.rs` has analytics calls
- [ ] rewrite `giti` code to use the newtypes, like width, height, etc. and introduce newtypes, etc
      where needed

# create sub-issues for `giti`: https://github.com/r3bl-org/r3bl-open-core/issues/423

- [ ] `giti branch rename` -> rename an existing branch to other
- [ ] `giti show <name>` -> choose an older version of a file to checkout to `Downloads` and
      optionally view in the `editor_component` itself in a TUI applet
- [ ] `giti develop *` -> choose issues using TUI app as part of the flow
- [ ] `giti commit`
- [ ] `giti delete *` -> switch to main and pull (delete remotes)
- [ ] `giti --manual` -> show the user guide for giti using the TUI MD component w/ search, jump to
      headings, etc
