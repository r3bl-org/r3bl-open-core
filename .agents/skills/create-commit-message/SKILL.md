---
name: create-commit-message
description: Rules and guidelines for creating well-formatted commit messages, including 72-char limits, scope prefixes, and trailer blocks for tasks, PR closing, and attribution.
---
# Create Commit Message Skill

You are an expert at writing clean, standard, and highly descriptive git commit messages. 
When asked to create a commit message, follow these exact guidelines:

## 1. Scope Prefix (Mandatory)
Start the subject line with a scope prefix in brackets, e.g., `[tui]`, `[core]`, `[macro]`.
- Always scan the recent `git log` history for the files you are modifying to determine the appropriate prefix. For example: `git log --oneline -- <file_path>` will show you what prefix is typically used for that component.

## 2. Formatting & Length Constraints
- **Subject Line:** Keep the subject line (including the scope prefix) to a maximum of 72 characters.
- **Body:** Wrap the commit message body to 72 characters per line.
- **Tone:** Use the imperative mood in the subject line (e.g., "Add feature" not "Added feature").

## 3. Trailer Block (Footers)
All trailers MUST be grouped together in a single contiguous block at the absolute end of the commit message. 
- There must be **exactly one blank line** before the start of the trailer block.
- There must be **zero blank lines** between the trailers themselves.

### Required / Optional Trailers
1. **Task Tracking (`Task:`)**: 
   - When a commit implements work from a `task/*.md` file, add a `Task:` trailer.
   - Do NOT include any directory prefixes (like `task/` or `task/done/`) in the filenames.
   - For multiple tasks, list them on separate lines with a comma ending each line except the last.
     ```text
     Task: one.md,
           two.md
     ```
2. **Issue/PR Closing (`Closes`)**: 
   - If the commit fully resolves a GitHub Issue or PR, include `Closes #XXX`.
3. **Attribution (`Co-authored-by:`)**:
   - If you are integrating work from another contributor (e.g., merging a community PR manually), include their attribution: `Co-authored-by: Name <email@example.com>`.

## Example
```text
[tui] Add DECCKM tracking and refactor parser state

This commit integrates the DECCKM (Cursor Key Mode) tracking logic from
PR #470, adding the missing `cursor_key_mode` field to the terminal
state and validating it with a new conformance integration test.

Task: prepare-v0.8.0-meta-task.md,
      pty-mux-bracketed-paste.md
Closes #470
Co-authored-by: Cecile Tonglet <cecile.tonglet@cecton.com>
```

## 4. Copying to Clipboard
If the user asks you to copy the commit message to the clipboard (e.g., they will handle the actual git commit step themselves), execute these exact steps:
1. Write the commit message to `/tmp/commit.msg.txt` using the `write_to_file` tool.
2. Execute the following in the `run_command` tool: `fish -c "cat /tmp/commit.msg.txt | setclip"`
3. Execute the following in the `run_command` tool: `rm /tmp/commit.msg.txt`
