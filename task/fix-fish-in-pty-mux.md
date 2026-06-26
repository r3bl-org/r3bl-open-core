_Task: Fix fish shell integration issues in PTY Mux_

# User Story & Context

During manual testing of the VT100 pending wrap fix in `pty_mux_example`, we noticed that
while the wrap logic works correctly, there are still a few rendering/interaction glitches
specific to the `fish` shell inside the multiplexer.

For example:

- When pressing `Tab` to trigger autocompletion, `fish` adds unexpected underlines to the
  output.

This task is intended to collect and address all remaining visual or functional glitches
related to `fish` running inside `pty_mux`.

# Implementation plan

- [ ] Investigate why `Tab` autocompletion in `fish` renders with unintended underlines.
- [ ] Identify if there are any unsupported ANSI sequences or rendering bugs in the
      `pty_mux` parser that `fish` triggers.
- [ ] Implement fixes for the identified glitches.
- [ ] Manually verify fixes in `pty_mux_example` with `fish`.
