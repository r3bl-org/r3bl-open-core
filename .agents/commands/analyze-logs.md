# Analyze Log Files

Invoke the `analyze-log-files` skill to analyze `/tmp/r3bl_tui/log.txt` (or other log files with ANSI escape sequences).

## Default Target

Analyze the `/tmp/r3bl_tui/log.txt` file in the project root.

## Instructions

1. Strip ANSI escape sequences using `ansifilter`
2. Read the cleaned log file
3. Report errors, warnings, and patterns found
