You can find the latest flamegraph analysis in the `tui/flamegraph.perf-folded` file.

When you make updates to the Markdown files in this folder, please ensure to:

1. Place the latest analysis at the top, and keep the previous analysis below it for historical
   reference.
2. And update the Table of Contents (TOC) accordingly. You can use the `doctoc` tool to generate the
   TOC automatically. If you want to update the TOC for `docs/task_tui_perf_optimize.md`, run the
   command:
   ```bash
   doctoc --github docs/task_tui_perf_optimize.md
   ```
3. To pretty-print the Markdown files, you can use the `prettier` tool. If you want to pretty-print
   `docs/task_tui_perf_optimize.md`, run the command:
   ```bash
   prettier --write docs/task_tui_perf_optimize.md
   ```
