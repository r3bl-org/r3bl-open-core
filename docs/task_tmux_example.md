# Create a tmux example in `r3bl_tui` examples

## Objective

Use `pty` module (in `r3bl_tui`) to create an example in the `r3bl_tui` crate that can
multiplex terminal sessions like `tmux`.

This example will be in this folder `/home/nazmul/github/r3bl-open-core/tui/examples/` and
it should be named `tmux_example.rs`.

The example should be able to:
- Spawn multiple processes (`btop`, `iotop`, `gputop`) using `spawn_read_write()` in a
  single "real" terminal window.
- Allow the user to switch between them using `Ctrl+<number>` keys.
- Show a footer bar w/ the current process name, and keyboard shortcuts.

> For context, look at [`task_prd_chi`](docs/task_prd_chi.md) for where this will be used
> in the future.

## Implementation Steps

TODO: