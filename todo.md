<!-- Move completed tasks to done.md. The task on top is the one to work on next. -->
<!-- Keep this file in sync with dashboard: https://github.com/users/nazmulidris/projects/4/views/1 -->

# create tmux POC

- [x] [`task_ring_buffer_enhance`](task/done/task_ring_buffer_enhance.md)
- [x] [`task_make_editor_vt_100_parser_more_typesafe`](task/done/task_make_editor_vt_100_parser_more_typesafe.md)
- [x] [`task_task_scroll_viewport_selection_refactor`](task/done/task_scroll_viewport_selection_refactor.md)
- [x] 2025-10-07 review `bounds_check`, and make polished documentation & ergonomic API
- [x] [`task_fix_params_ext`](task/done/task_fix_params_ext.md)
- [⌛] review `pty_mux`, `offscreen_buffer` and `ansi conformance tests`
- [⌛] [`task_pty_mux_example`](task/task_pty_mux_example.md)
- [ ] extend `main_event_loop.rs` to support OSC output to terminal emulator (main window)
  - via `GlobalData::main_thread_channel_sender` -> add a variant to
    `TerminalWindowMainThreadSignal`
  - such that it can be handled by `run_main_event_loop()`'s `main_thread_channel_receiver.recv()`
  - using `OscController` to write to `OutputDevice`

# remove crossterm

- [x] use [`task_clean_render_ops_type_design`](task/done/task_clean_render_ops_type_design.md) to
      clean up `RenderOp` type ambiguity between "output" and "ir" contexts. The render pipeline is
      actually:
      `App -> Component -> RenderOps(IR) -> OffscreenBuffer -> RenderOps(Output) -> OutputDevice`

- [x] use [`task_refactor_input_device`](task/task_refactor_input_device.md) to refactor
      `InputDevice` to unify `crossterm`, `direct_to_ansi`, and `mock` variants

- [⌛] use [`task_remove_crossterm`](task/task_remove_crossterm.md) to remove crossterm from the
  `r3bl_open_core` codebase

- [⌛] use [`task_readline_async_add_shortcuts.md`](task/task_readline_async_add_shortcuts.md) to
  add `readline_async` support for all the shortcuts that we don't currently support but `readline`
  does

- [⌛] migrate `check.fish` into `build_infra` crate as `cargo monitor` command using
  [`build_infra_cargo_monitor.md`](/task/build_infra_cargo_monitor.md)

- [x] bootstrap `r3bl_build_infra` crate with first tool (binary)
      [`task_cargo_rustdoc_fmt.md](task/done/task_cargo_rustdoc_fmt.md)

- [ ] fix [`add-pty-offscreen-buffer-output-tests.md`](task/add-pty-offscreen-buffer-output-tests.md)

- [x] fix [`add-backend-compat-tests.md`](task/done/add-backend-compat-tests.md:28)

- [x] fix `rl_async` bugs [`rl_async_update.md`](task/done/rl_async_update.md):
  - https://github.com/r3bl-org/r3bl-open-core/issues/439
  - https://github.com/r3bl-org/r3bl-open-core/pull/440

- [ ] use [`task_unify_cli_and_styled_text`](task/pending/task_unify_cli_and_styled_text.md) to
      unify `CliText` and `TuiStyledText` rendering paths

- [ ] use [`task_render_path_2_add_ofs_buf`](task/pending/task_render_path_2_add_ofs_buf.md) to add
      use `OffscreenBuffer` to radically simplify hybrid / partial TUI codepaths! This paves the way
      for having each Component paint into its own OffscreenBuffer, and then composing them together
      for automatic scrolling and z-index handling

# optimize offscreen buffer

- [ ] [`task_ofs_buf_1d_array.md`](task/pending/task_ofs_buf_1d_array.md)

# unify rendering paths

- [x] 2025-10-22 use [`task_unify_rendering`](task/done/task_unify_rendering.md) to unify the
      rendering paths of `ASText`, `TuiStyledText`, and `readline_async` into a single, optimized
      rendering pipeline that works for both use cases while preparing for the future removal of
      crossterm dependency

# submit talk for tokio conf 2026 proposal

- [x] 2025-10-20 submit proposal for tokio conf 2026 talk on async TTY primitives in `r3bl_tui`
  - [`task_tokio_conf_2026_proposal`](task/done/task_tokio_conf_2026_proposal.md)
  - [submitted](https://sessionize.com/app/speaker)

# rearchitect how scrolling and rendering is done

- [ ] Instead of having a single OffscreenBuffer for the terminal window, have 1 for each component.
      Then compose them together w/ proper Z-index handling. This should simplify a lot of
      complexity around scrolling

# fix editor component bugs

- [ ] Run the tui_examples #3 (editor) and invoke simple or complex dialog box. Select a few chars
      of text at the end and scroll to the left, and see the component painting outside of its
      bounds (on the right).

# create robust atomic file based user configuration settings: https://github.com/nazmulidris/rust-scratch/issues/114

# add analytics server to r3bl-cmdr: https://github.com/r3bl-org/r3bl-base/issues/6

- [ ] with `shuttle.rs` gone, and homelab up and running, implement this, before launching too many
      new features in `r3bl-cmdr` that will need telemetry and analytics to see what users care
      about

# incorporate TLS and tcp-api-server work into the codebase: https://github.com/r3bl-org/r3bl-base/issues/6

# build and deploy r3bl-base backend (in homelab) for telemetry and analytics: https://github.com/r3bl-org/r3bl-base/issues/6

# add analytics to `ch` binary in r3bl-cmdr

- [ ] task [`task_ch_analytics`](task/pending/task_ch_analytics.md) to add analytics to `ch`

# build `chi` binary in r3bl-cmdr

- [ ] PRD [`task_prd_chi`](task/pending/task_prd_chi.md) to build `chi`

# build `build-infra-tools` binary to replace all the fish scripts

- [ ] PRD [`task_build_tools_infra_plan`](task/pending/task_build_tools_infra_plan.md) to build
      `build-infra-tools`

# refactor or rewrite the UI layout, sizing, and styling code

- [ ] the code should be easier to use, do an audit and figure out what needs to be done

# consider replacing syntect

- [ ] use [`task_syntect_improve`](task/pending/task_syntect_improve.md) to add support for
      TypeScript, TOML, SCSS, Kotlin, Swift, and Dockerfile languages by adding custom
      `.sublime-syntax` files to syntect

# rewrite textwrap

- [ ] use [`task_textwrap_rewrite`](task/pending/task_textwrap_rewrite.md) to rewrite `textwrap`
      crate for better unicode performance. this could be used in `edi` as well, for wrap & TOC
      create/update on save.

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

# add animation api: https://github.com/r3bl-org/r3bl-open-core/issues/174

# add TTS "assistive technology" support in the framework

- [ ] Not "accessibility" but "assistive technology" support in order to support people of
      [infinite diversity in infinite combinations](<https://en.wikipedia.org/wiki/Vulcan_(Star_Trek)#IDIC>).
      This is meant to augment users with situational, temporary disabilities, AND varying
      capabilities. For example, someone might have a preference (e.g., they absorb information 3x
      faster aurally vs visually) to hear text read out loud, because this is their preferred /
      optimized way to consume information. So, notifications should be read out loud for these
      users, in addition to being displayed in terminal / GUI environment, etc. So this is about
      meeting the user where they are, rather than forcing them to adapt to the technology. Another
      AI feature could be "summarize this and read out loud" that is available everywhere in the
      framework.

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

# giti features: https://github.com/r3bl-org/r3bl-open-core/issues/423

- [ ] The parent issue has some pre-existing sub-issues. Make sure that all sub-issues in the body
      of the issue have been created. Then work on each of them.

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
