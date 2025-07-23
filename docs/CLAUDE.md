<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Where to find the latest flamegraph analysis](#where-to-find-the-latest-flamegraph-analysis)
- [How to update the Markdown files](#how-to-update-the-markdown-files)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Where to find the latest flamegraph analysis

You can find the latest flamegraph analysis in the `tui/flamegraph.perf-folded` file.

# How to update the Markdown files

When you make updates to the Markdown files in this folder, please ensure that:

1. Place the latest analysis at the top, and keep the previous analysis below it for historical
   reference.
2. And update the Table of Contents (TOC) accordingly. You can use the `doctoc` tool to generate the
   TOC automatically. Eg, if you want to update the TOC for `docs/task_tui_perf_optimize.md`, run
   the command:
   ```bash
   doctoc --github docs/task_tui_perf_optimize.md
   ```
3. To pretty-print the Markdown files, you can use the `prettier` tool. Eg, if you want to
   pretty-print `docs/task_tui_perf_optimize.md`, run the command:
   ```bash
   prettier --write docs/task_tui_perf_optimize.md
   ```
