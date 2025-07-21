# clean up and release: https://github.com/r3bl-org/r3bl-open-core/issues/397

- [x] fix all the lints after the extraction & archival of the `md_parser_ng`
- [x] create parser conformance snapshot test, and make sure they pass
      [docs/parser_conformance.md](docs/parser_conformance.md)
- [x] review the flamegraph.svg and cargo bench results to ensure no regressions
      [docs/task_tui_perf_optimize.md](docs/task_tui_perf_optimize.md)
- [x] complete the performance work started in
      [task_tui_perf_optimize](docs/task_tui_perf_optimize.md)
- [x] fix windows bug: https://github.com/r3bl-org/r3bl-open-core/issues/433
- [x] refactor `md_parser` with consistent naming and module organization
- [x] add missing tests to `editor` module
- [x] fix copy/paste bugs in `editor` module (support bracketed paste mode too)
- [ ] there are test failures in doctests that try to use terminal I/O (which fails in test
      environment). can you identify and mark them to be "```no_run"
- [ ] fix all the pedantic lints using claude (and don't allow them anymore in Cargo.toml)
- [ ] update changelog
- [ ] squash all the commits in this branch `fix-md-parser` into one (fixup all the commit
      messages); this commit fixes this issue:
      <https://github.com/r3bl-org/r3bl-open-core/issues/397>. then rebase this branch onto `main`
      and push it to remote `origin`.
- [ ] make a release using the [`release-guide.md`](docs/release-guide.md) document as a guide

# editor content storage enhancements

- [ ] Change `EditorContent::lines: VecEditorContentLines` to a different data structure that is
      still an array of lines, which doesn't need to be materialized into `String` and can be
      accessed as `&str`, but works with a modified legacy parser which knows how to handle a
      different kind of `EOL`. This data structure represents a line as a char array of some default
      size (eg: 256 chars) and is preallocated. The `\n` char followed by a char that can't be typed
      in the editor (eg: `\0`) is used to represent the end of line. This will require changes to
      the editor component as well as the parser. Effectively, this is a gap buffer implementation.
      Once a line is allocated, the only time it will be reallocated / resized is when the line is
      too long (eg: more than 256 chars), or a line is deleted. Reallocation is cheap (relatively
      speaking) because to copy 100K bytes it takes a few thousand nanoseconds. And this
      reallocation will only happen rarely. This means that to syntax highlight these `lines` it is
      zero-copy! This won't make that big of a difference except in cases where the documents are
      very large (> 1MB). This comes from the work done in the `md_parser_ng` crate which is archive
      that showed that a `&str` parser is the fastest. So instead of bringing the mountain to
      Muhammad, we will bring Muhammad to the mountain. The mountain is the `&str` parser, and
      Muhammad is the editor component. Here's some code:

  ```rust
  // Assume the line is 256 chars long.
  let empty_line = ['\0', '\0', '\0', ..., '\0']  // 256 '\0' chars
  let line_with_content = ['H', 'e', 'l', 'l', 'o', '\n', '\0', '\0', ..., '\0']
  use nom::{bytes::complete::take_while, character::complete::char, combinator::recognize,
        sequence::tuple, IResult, Parser};

  // The parser will consume the entire pattern and you can extract the actual content
  // by finding the position of \n in the matched slice.
  fn parse_editor_line(input: &str) -> IResult<&str, &str> {
        recognize((
        (
              take_while(|c| c != '\n' && c != '\0'),  // Line content
              char('\n'),                              // Required newline
              take_while(|c| c == '\0')                // Zero or more null padding
        )
        )).parse(input)
  }
  ```

# rewrite textwrap

- [ ] consider rewriting `textwrap` crate for better unicode performance. this could be used in
      `edi` as well, for wrap & TOC create/update on save. More info in
      [task_tui_perf_optimize](docs/task_tui_perf_optimize.md)
  ```
  **Text Wrapping Operations** (45M samples - HIGHEST PRIORITY):
  - `textwrap::wrap_single_line`: 18.8M samples
  - `find_words_unicode_break_properties`: 25.9M samples
  - Heavy overhead in log formatting paths
  - Consider caching wrapped text or optimizing unicode word breaking
  ```

# edi feature

- [ ] add feature that shows editor status to the terminal window title bar (eg: "edi - [filename] -
      [status]") using
      [OSC sequences](<https://en.wikipedia.org/wiki/ANSI_escape_code#OSC_(Operating_System_Command)_sequences>)
- [ ] add a new feature to edi: `cat file.txt | edi` should open the piped output of the first
      process into edi itself

# markdown parser enhancements

- [ ] fix "`rust`" parsing in syn hi code (should support both "rust" and "rs")
- [ ] support both `"**"` and `"*"` for bold, and `"_"` and `"__"` for italic (deviate from the
      markdown spec)
- [ ] add blockquote support
  - [ ] impl parser support for blockquote
  - [ ] impl syntax highlighting support for blockquote
- [ ] markdown table support
  - [ ] impl parser support for table
  - [ ] impl syntax highlighting support for table

# giti feature

- add giti feature to search logs

# editor fixes and enhancements

- [ ] fix copy / paste bugs in editor component
- [ ] add basic features to editor component (find, replace, etc)
- [ ] add support for `rustfmt` and `prettier` like reformatting of the code in the editor component

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
