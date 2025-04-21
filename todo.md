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
- [ ] update all the projects in `rust_scratch` to work with the `tuifyasync` branch
  - [x] `tls`
  - [ ] `tcp-api-server`

# clean up giti phase 5

- [ ] use crates.io api to check for latest release to show upgrade message for `edi` and `giti`
      https://github.com/r3bl-org/r3bl-open-core/issues/415

# clean up giti phase 6

- [ ] reorder `giti` commands so that `checkout` is first (not `delete`)
- [ ] use `InlineString` & `InlineVec` in `giti` codebase (for sake of consistency)
- [ ] fix `giti branch delete <branch-name>` which currently does not work since this command
      ignores branches that are passed as a command line arg
- [ ] fix clap args using `color_print::cstr` instead of directly embedding ansi escape sequences in
      the clap macro attributes `clap_config.rs`. see `rust_scratch/tcp-api-server` for examples
- [ ] make sure that analytics calls are made consistent throughout the giti codebase (currently
      they do nothing but this will get things ready for the new `r3bl_base` that will be self
      hosted in our homelab); currently `delete.rs` has analytics calls
- [ ] rewrite `giti` code to use the newtypes, like width, height, etc. and introduce newtypes, etc
      where needed
- [ ] use `r3bl_script` test fixtures to test `git` and `gh` adapters

# replace HashMap with BTreeMap (better cache locality performance)

- [ ] `HashMap` is great for random access, `BTreeMap` is good for cache locality and iteration
      which is the primary use case for most code in `r3bl_open_core` repo

# add fps counter row to bottom of edi

- [ ] just like in the `tui/examples/demo/*` add an FPS/telemetry display to bottom of edi

# merge tuifyasync branch into main

- [ ] crate a PR for tuifyasync & merge it into main
- [ ] update all the projects in `rust_scratch` to work with latest github version of `r3bl_tui`
  - [ ] `tls`
  - [ ] `tcp-api-server`

# make a release

- [ ] `r3bl_tui`
- [ ] `r3bl_cmdr`
- [ ] close this: https://github.com/r3bl-org/r3bl-open-core/issues/365

# create sub-issues for giti https://github.com/r3bl-org/r3bl-open-core/issues/391

- [ ] `giti show <name>` -> choose an older version of a file to checkout to `Downloads` and
      optionally view in the `editor_component` itself in a TUI applet
- [ ] `giti develop *` -> choose issues using TUI app as part of the flow
- [ ] `giti commit`
- [ ] `giti delete *` -> switch to main and pull (delete remotes)
- [ ] `giti --manual` -> show the user guide for giti using the TUI MD component w/ search, jump to
      headings, etc
