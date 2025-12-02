---
name: analyze-log-files
description: Analyze log files by stripping ANSI escape sequences first. Use when asked to process, handle, read, or analyze log files that may contain terminal escape codes.
---

# Analyze Log Files

## When to Use

- When user asks to "analyze log.txt", "read the log file", "process logs", "check the logs"
- When dealing with any `.log` or log-related files that may contain ANSI escape sequences
- When terminal output has been captured to a file and needs analysis
- When log files appear garbled or contain escape sequence artifacts

## Why This Matters

Log files captured from terminal sessions often contain ANSI escape sequences for:
- Colors (e.g., `\x1b[31m` for red)
- Cursor movements
- Text formatting (bold, underline)
- Screen clearing commands

These sequences make logs difficult to:
1. Read in plain text editors
2. Search with grep/ripgrep
3. Process with text analysis tools
4. Analyze accurately by LLMs

## Instructions

### Step 1: Strip ANSI Escape Sequences

Before analyzing any log file, first strip the ANSI sequences using `ansifilter`:

```bash
ansifilter -i log.txt -o /tmp/clean_log.txt
```

For other log file names, adjust accordingly:
```bash
ansifilter -i <input_file> -o /tmp/clean_log.txt
```

### Step 2: Analyze the Clean Log

Read and analyze `/tmp/clean_log.txt` instead of the original file:

```bash
# Use the Read tool on /tmp/clean_log.txt
```

### Step 3: Report Findings

When reporting findings to the user:
- Reference line numbers from the clean log
- Quote relevant sections
- Summarize errors, warnings, or patterns found

## Common Log File Locations

- `log.txt` - General purpose log in project root
- `target/` - Cargo build logs
- `/tmp/*.log` - Temporary logs

## Example Workflow

User: "Can you analyze log.txt and tell me what's wrong?"

1. Run: `ansifilter -i log.txt -o /tmp/clean_log.txt`
2. Read: `/tmp/clean_log.txt`
3. Analyze the content for errors, warnings, patterns
4. Report findings to user

## Troubleshooting

If `ansifilter` is not installed:
```bash
# Ubuntu/Debian
sudo apt-get install ansifilter

# macOS
brew install ansifilter

# Or run bootstrap.sh to install all dependencies
./bootstrap.sh
```

## Related Skills

- `check-code-quality` - For checking Rust code quality (may generate logs)
- `analyze-performance` - For performance analysis (generates flamegraph data)
