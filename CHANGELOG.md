<!-- TOC -->

- [r3bl_tui](#r3bl_tui)
  - [Unreleased](#unreleased)
  - [v0.3.2 2023-03-06](#v032-2023-03-06)
  - [v0.3.1 2023-03-06](#v031-2023-03-06)
- [r3bl_rs_utils_core:](#r3bl_rs_utils_core)
  - [Unreleased](#unreleased)
  - [v0.9.1 2023-03-06](#v091-2023-03-06)
- [More info on changelogs](#more-info-on-changelogs)

<!-- /TOC -->

## r3bl_tui
<a id="markdown-r3bl_tui" name="r3bl_tui"></a>


### Unreleased
<a id="markdown-unreleased" name="unreleased"></a>


### v0.3.2 (2023-03-06)
<a id="markdown-v0.3.2-2023-03-06" name="v0.3.2-2023-03-06"></a>


- Fixed:
  - Bug when trying to render an app that's taller than the offscreen buffer / terminal height

### v0.3.1 (2023-03-06)
<a id="markdown-v0.3.1-2023-03-06" name="v0.3.1-2023-03-06"></a>


- Added:
  - First changelog entry.
  - Remove dependency on ansi-parser crate: <https://github.com/r3bl-org/r3bl_rs_utils/issues/91>
  - Make lolcat code better: <https://github.com/r3bl-org/r3bl_rs_utils/issues/76>
    - Add `ColorSupport` as a way to detect terminal emulator capabilities at runtime.
    - Add `ColorWheel` as a way to consolidate all gradient related coloring. Use `ColorSupport` as
      a way to fallback from truecolor, to ANSI 256, to grayscale gracefully based on terminal
      emulator capabilities at runtime.
  - Provide for ANSI 256 color fallback for MacOS terminal app:
    <https://github.com/r3bl-org/r3bl_rs_utils/issues/79>
- Removed: <a id="markdown-removed%3A" name="removed%3A"></a>
  - Removed lolcat example from demo.
- Changed:
  - The first demo example (ex_app_no_layout) now has support for animation. It automatically
    increments the state every second and the gradient color wheel is updated accordingly.

## r3bl_rs_utils_core:
<a id="markdown-r3bl_rs_utils_core%3A" name="r3bl_rs_utils_core%3A"></a>


### Unreleased
<a id="markdown-unreleased" name="unreleased"></a>


### v0.9.1 (2023-03-06)
<a id="markdown-v0.9.1-2023-03-06" name="v0.9.1-2023-03-06"></a>


- Added:
  - First changelog entry.
  - Move lolcat into `tui_core` crate.
- Removed:
  - ANSI escape sequences are no longer used internally in any intermediate format used by the TUI
    engine. It is reserved exclusively for output to stdout using (for now) crossterm. This opens
    the door for future support for GUI app (not just terminal emulators).

## More info on changelogs
<a id="markdown-more-info-on-changelogs" name="more-info-on-changelogs"></a>


- https://keepachangelog.com/en/1.0.0/
- https://co-pilot.dev/changelog