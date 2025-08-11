<!-- Move completed tasks to done.md. The task on top is the one to work on next. -->
<!-- Keep this file in sync with dashboard: https://github.com/users/nazmulidris/projects/4/views/1 -->

# fix r3bl-cmdr upgrade code: https://github.com/r3bl-org/r3bl-open-core/issues/437, https://github.com/nazmulidris/rust-scratch/issues/117

- [x] do research and make a rust-scratch issue and project for this:
      https://github.com/nazmulidris/rust-scratch/issues/117
- [x] publish the README.md from the rust-scratch project to developerlife.com as tutorial:
      https://developerlife.com/2025/08/10/pty-rust-osc-seq/
- [x] configure serena mcp server for semantic code search
- [x] add `pty` module and implement the following:
  - [x] single channel
    - [x] code
    - [x] example
    - [x] tests in [`task_test_pty`](docs/task_test_pty.md)
  - [ ] dual channel in [`task_dual_channel_pty`](docs/task_dual_channel_pty.md)
    - [ ] code
    - [ ] example
    - [ ] tests
- [ ] implement the upgrade code in `r3bl-cmdr` to use the code above:
      https://github.com/r3bl-org/r3bl-open-core/issues/437
- [ ] make YT video on the research (in rust-scratch) issue
- [ ] make YT video on the implementation (to r3bl-cmdr) issue

# unify rendering paths

- [ ] use [`task_unify_rendering`](docs/task_unify_rendering.md) to unify the rendering paths of
      `ASText` and `TuiStyledText` into a single, optimized rendering pipeline that works for both
      use cases while preparing for the future removal of crossterm dependency

# remove crossterm

- [ ] use [`task_remove_crossterm`](docs/task_remove_crossterm.md) to remove crossterm from the
      `r3bl_open_core` codebase

# add analytics server to r3bl-cmdr: https://github.com/r3bl-org/r3bl-base/issues/6

- [ ] with `shuttle.rs` gone, and homelab up and running, implement this, before launching too many
      new features in `r3bl-cmdr` that will need telemetry and analytics to see what users care
      about

# refactor or rewrite the UI layout, sizing, and styling code

- [ ] the code should be easier to use, do an audit and figure out what needs to be done

# consider replacing syntect

- [ ] use [`task_syntect_improve`](docs/task_syntect_improve.md) to add support for TypeScript,
      TOML, SCSS, Kotlin, Swift, and Dockerfile languages by adding custom `.sublime-syntax` files
      to syntect

# rewrite textwrap

- [ ] use [`task_textwrap_rewrite`](docs/task_textwrap_rewrite.md) to rewrite `textwrap` crate for
      better unicode performance. this could be used in `edi` as well, for wrap & TOC create/update
      on save.

# markdown parser enhancements

- [ ] support both `"**"` and `"*"` for bold, and `"_"` and `"__"` for italic (deviate from the
      markdown spec)
- [ ] support bold and italic that spawn multiple lines (deviate from the markdown spec)
- [ ] add blockquote support
  - [ ] impl parser support for blockquote
  - [ ] impl syntax highlighting support for blockquote
- [ ] markdown table support
  - [ ] impl parser support for table
  - [ ] impl syntax highlighting support for table

# add prettier

- [ ] implement prettier-like functionality and "run on save" support performs text wrapping and
      markdown formatting, using `dprint-plugin-markdown` crate. this is different than the textwrap
      functionality, since this only runs on save, just before the editor contents are written to
      disk; it does not have the same high performance runtime requirements as textwrap.

# enable mouse support

- [ ] figure out how to interpret mouse events into something that is usable for the apps

# incorporate TLS and tcp-api-server work into the codebase: https://github.com/r3bl-org/r3bl-base/issues/6

# build and deploy r3bl-base backend (in homelab) for telemetry and analytics: https://github.com/r3bl-org/r3bl-base/issues/6

# add animation api: https://github.com/r3bl-org/r3bl-open-core/issues/174

# create robust atomic file based user configuration settings: https://github.com/nazmulidris/rust-scratch/issues/114

# edi features

- [ ] cache AST in editor to implement jump to link (intra doc link) and jump to heading (intra doc
      link) functionality
- [ ] to implement `Ctrl+b` in the editor, so that jumping to hyperlinks can be implement it might
      be a good idea to save the abstract syntax tree of the parsed markdown data structure
      MdDocument in memory, along with a way to find the element under a given (row, col) index
- [ ] add telemetry HUD to bottom of `edi` (FPS, memory usage, etc)
- [ ] add feature that shows editor status to the terminal window title bar (eg: " edi -
      [filename] - [status]") using
      [OSC sequences](https://en.wikipedia.org/wiki/ANSI_escape_code#Operating_System_Command_sequences)
- [ ] add a new feature to edi: `cat file.txt | edi` should open the piped output of the first
      process into edi itself
- [ ] show table of contents with type ahead complete on `ctrl+y`

# giti feature

- add giti feature to search logs

# editor fixes and enhancements

- [ ] fix copy / paste bugs in editor component
- [ ] add basic features to editor component (find, replace, etc)
- [ ] add support for `rustfmt` and `prettier` like reformatting of the code in the editor component

# modernize `choose` and

`giti` codebase: https://github.com/r3bl-org/r3bl-open-core/issues/427

- [ ] use `InlineString` & `InlineVec` in `giti` codebase (for sake of consistency)
- [ ] fix clap args using `color_print::cstr` instead of directly embedding ansi escape sequences in
      the clap macro attributes `clap_config.rs`. see `rust_scratch/tcp-api-server` for examples
- [ ] make sure that analytics calls are made consistent throughout the giti codebase ( currently
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

# make release of `r3bl-cmdr` and `r3bl_tui` v1

- [ ] remove all the language about early access release / preview (update r3bl.com website as well)
- [ ] make sure `cmdr` docker file works (with `pkg-config` and `libssl-dev` removed):
      https://github.com/r3bl-org/r3bl-open-core/issues/426
- [ ] make sure this works on macOS (via cargo install)
- [ ] make sure this works on windows (via cargo install)

# brainstorm UX and impl of multi-user editing on LAN without any configuration (mDNS, etc)

# brainstorm the UX and impl of the r3bl-runner concept (jupyter-like notebooks for md)
