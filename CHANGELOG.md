# Changelog
<a id="markdown-changelog" name="changelog"></a>


<!-- TOC -->

- [r3bl_simple_logger](#r3bl_simple_logger)
  - [v0.1.0 2023-10-14](#v010-2023-10-14)
- [r3bl_rs_utils_core](#r3bl_rs_utils_core)
  - [v0.9.5 2023-10-14](#v095-2023-10-14)
  - [v0.9.1 2023-03-06](#v091-2023-03-06)
- [r3bl_tui](#r3bl_tui)
  - [Next release](#next-release)
  - [v0.3.3 2023-04-20](#v033-2023-04-20)
  - [v0.3.2 2023-03-06](#v032-2023-03-06)
  - [v0.3.1 2023-03-06](#v031-2023-03-06)
- [More info on changelogs](#more-info-on-changelogs)

<!-- /TOC -->

## `r3bl_simple_logger`
<a id="markdown-r3bl_simple_logger" name="r3bl_simple_logger"></a>

### v0.1.0 (2023-10-14)
<a id="markdown-v0.1.0-2023-10-14" name="v0.1.0-2023-10-14"></a>

- Added:
  - First changelog entry. This crate is a fork of the
    [`simplelog`](https://crates.io/crates/simplelog) repo w/ conditional compilation
    (feature flags) removed. This crate was causing transitive dependency issues in
    upstream repos that added `r3bl_tuify` as a dependency. Here's a link to the related
    [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/160).

## `r3bl_rs_utils_core`
<a id="markdown-r3bl_rs_utils_core" name="r3bl_rs_utils_core"></a>


### v0.9.5 (2023-10-14)
<a id="markdown-v0.9.5-2023-10-14" name="v0.9.5-2023-10-14"></a>

- Updated:
  - Dependency on `simplelog` is replaced w/ `r3bl_simple_logger` (which is in the
    `r3bl_rs_utils` repo workspace as `simple_logger`).
  - `TuiColor` has a few new variants. They can be `RgbValue`, `AnsiValue`, or `ANSIBasicColor`. It
    is safe to use just `RgbValue` since the library will degrade gracefully to ANSI 256 or
    grayscale based on terminal emulator capabilities at runtime (provided by `to_crossterm_color()`
    and `ColorSupport`). If a color is specified as `AnsiValue` or `ANSIBasicColor` then it will not
    be downgraded.


### v0.9.1 (2023-03-06)
<a id="markdown-v0.9.1-2023-03-06" name="v0.9.1-2023-03-06"></a>

- Added:
  - First changelog entry.
  - Move lolcat into `tui_core` crate.
- Removed:
  - ANSI escape sequences are no longer used internally in any intermediate format used by the TUI
    engine. It is reserved exclusively for output to stdout using (for now) crossterm. This opens
    the door for future support for GUI app (not just terminal emulators).

## `r3bl_rs_utils_macro`

### v0.9.4 (2023-10-14)

- Updated:
  - Change dependency on all workspace crates to be locally resolved without going to
    [crates.io](https://crates.io).

## `r3bl_tui`
<a id="markdown-r3bl_tui" name="r3bl_tui"></a>

### Next release
<a id="markdown-next-release" name="next-release"></a>

- Fixed:
    - Main event loop was actually doing the wrong thing and blocking on the thread. Even though it
      accepted an input event asynchronously using `AsyncEventStream` (`EventStream` is provided by
      `crossterm` and built using tokio async streams), it was blocking this task (running in
      parallel on a thread) as it was waiting for the input event to be processed by the app. The
      fix allows the main thread to simply spawn a new task (in parallel, on a thread) to process
      the input event. An `mpsc` channel is used in order for the async work done to signal to the
      main thread that it should break out of its infinite loop.

### v0.3.3 (2023-04-20)
<a id="markdown-v0.3.3-2023-04-20" name="v0.3.3-2023-04-20"></a>


- Added:
  - Add `ColorSupport` as a way to detect terminal emulator capabilities at runtime. This uses the
    [`concolor_query`](https://docs.rs/concolor-query/latest/concolor_query/) crate to detect
    terminal emulator capabilities at runtime.
  - At the `RenderOps` level, update `to_crossterm_color()` so that it uses `ColorSupport` to
    determine the best color to use based on terminal emulator capabilities at runtime. It can
    automatically convert from truecolor to ANSI 256 to grayscale. Note that if a color is specified
    as truecolor, then it will automatically be downgraded. If it is specified as ANSI or grayscale
    then it will not be downgraded.
  - Add `ColorWheel` as a way to consolidate all gradient related coloring. Gradients can be
    specified in truecolor, ANSI 256, or grayscale. The `ColorWheel` will automatically use the
    correct colors based on the terminal emulator capabilities at runtime using `ColorSupport`.
  - Add new Markdown parser written using [`nom`](https://crates.io/crates/nom) crate called
    `parse_markdown()`.
    - This parser not only parses regular Markdown but it also supports R3BL extensions for notes
      (metadata: tags, title, authors, date).
    - And it also supports smart lists (ordered and unordered). Smart lists also have support for
      todos (in the form of checked and unchecked items).
  - Add a new syntax highlighting engine for the new Markdown parser, in the `EditorComponent`
    called `try_parse_and_highlight()`.
    - It formats headings using different gradients for each heading levels 1-6. It also has elegant
      fallbacks for ANSI256 and grayscale.
    - It formats metadata (tags, title, authors, date) using different fg and bg colors.
    - Smart lists are formatted using different fg and bg colors. Ordered and unordered lists are
      formatted differently. Checked and unchecked items are formatted differently.
    - For code blocks, the `syntect` crate is used to do syntax highlighting based on the correct
      language of the code block. Since the R3BL theme `r3bl.tmTheme` specifies colors in truecolor,
      they will automatically be downgraded to ANSI256 or grayscale based on terminal emulator
      capabilities at runtime thanks to `to_crossterm_color()`.
  - To make console log debugging nicer, some new traits have been added `ConsoleLogInColor`,
    `PrettyPrintDebug`. These traits work together. If a struct implements `PrettyPrintDebug` then
    it gets the implementation of `ConsoleLogInColor` for free (which gives it the ability to print
    using fg and bg colors to the console).

### v0.3.2 (2023-03-06)
<a id="markdown-v0.3.2-2023-03-06" name="v0.3.2-2023-03-06"></a>


- Fixed:
  - Bug when trying to render an app that's taller than the offscreen buffer / terminal height

### v0.3.1 (2023-03-06)
<a id="markdown-v0.3.1-2023-03-06" name="v0.3.1-2023-03-06"></a>


- Added:
  - First changelog entry.
  - Remove dependency on ansi-parser crate:
    [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/91).
  - Make lolcat code better: [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/76).
    - Add `ColorSupport` as a way to detect terminal emulator capabilities at runtime.
    - Add `ColorWheel` as a way to consolidate all gradient related coloring. Use `ColorSupport` as
      a way to fallback from truecolor, to ANSI 256, to grayscale gracefully based on terminal
      emulator capabilities at runtime.
  - Provide for ANSI 256 color fallback for MacOS terminal app:
    [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/79)
- Removed: <a id="markdown-removed%3A" name="removed%3A"></a>
  - Removed lolcat example from demo.
- Changed:
  - The first demo example (`ex_app_no_layout`) now has support for animation. It automatically
    increments the state every second and the gradient color wheel is updated accordingly.

## More info on changelogs
<a id="markdown-more-info-on-changelogs" name="more-info-on-changelogs"></a>


- https://keepachangelog.com/en/1.0.0/
- https://co-pilot.dev/changelog
