# Analyze Log Files

Invoke the `analyze-log-files` skill to analyze `log.txt` (or other log files with ANSI escape sequences).

## Default Target

Analyze the `log.txt` file in the project root.

## Instructions

1. Strip ANSI escape sequences using `ansifilter`
2. Read the cleaned log file
3. Report errors, warnings, and patterns found
