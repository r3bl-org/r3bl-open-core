# ✔ clean up giti phase 3

- [x] introduce consistent error reporting, and output handling using `CompletionReport`. there is
      no need for individual subcommands to do something specific to report command success, not
      success, or failure to run command. centralize this and simplify ALL subcommands, and make it
      easy to perform logging and analytics reporting.
- [ ] replace `UIStrings` enum
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

# 📌 clean up giti phase 4

- [x] fix `show_exit_message()` does not appear all the time
- [x] in `git.rs` use `r3bl_script` to run commands (and not directly using `Command::new`)
- [ ] in `Display` impl of `CommandRunResult` don't print everything, write some items log (eg:
      `CommandRunDetails`, etc.); does this need to be in `r3bl_script`?
- [ ] use `InlineString` & `InlineVec` in `giti` codebase (for sake of consistency)
- [ ] fix `giti branch delete <branch-name>` which currently does not work since this command
      ignores branches that are passed as a command line arg
- [ ] fix clap args using `color_print::cstr` instead of directly embedding ansi escape sequences in
      the clap macro attributes `clap_config.rs`. see `rust_scratch/tcp-api-server` for examples
- [ ] make sure that analytics calls are made consistent throughout the giti codebase (currently
      they do nothing but this will get things ready for the new `r3bl_base` that will be self
      hosted in our homelab); currently `delete.rs` has analytics calls
- [ ] rewrite giti code to use the newtypes, like width, height, etc. and introduce newtypes, etc
      where needed
- [ ] use `r3bl_script` test fixtures to test `git` and `gh` adapters

# remove `r3bl_core` as a top level crate

- [ ] move this code into `r3bl_tui`
- [ ] deprecate the `r3bl_core` crate

# replace HashMap with BTreeMap (better cache locality performance)

- [ ] HashMap is great for random access, BTreeMap is good for cache locality and iteration which is
      the primary use case for most code in r3bl_open_core repo

# fix lib.rs / README.md

- [ ] main workspace remove all references to `r3bl_tuify` and `r3bl_terminal_async`, and all the
      other crates that have been archived
- [ ] `tui` folder do the same as above
- [ ] `core` folder do the same as above

# add fps counter row to bottom of edi

- [ ] just like in the `tui/examples/demo/*` add an FPS/telemetry display to bottom of edi

# merge tuifyasync branch into main

- [ ] crate a PR for tuifyasync & merge it into main

# make a release

- [ ] `r3bl_tui`
- [ ] `r3bl_cmdr`

# create new issues & sub-issues for giti features

- [ ] `giti develop *` -> choose issues using TUI app as part of the flow
- [ ] `giti commit`
- [ ] `giti delete *` -> switch to main and pull (delete remotes)
- [ ] `giti --manual` -> show the user guide for giti using the TUI MD component w/ search, jump to
      headings, etc
