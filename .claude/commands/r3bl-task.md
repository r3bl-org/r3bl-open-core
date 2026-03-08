# Manage a task 1) [create, update, load, done] 2) [name]

## Arguments $1 and $2

$1 is the command to execute: "create", "update", "load", "done"

$2 is the task name, e.g., "feature-auth" or "feature-auth.md". The file will be created
at "./task/$2.md" (or "./task/$2" if $2 already ends with .md)

## Algorithm to execute $1 and $2

if $1 is "create" then:

- If you have a plan then continue, otherwise there is nothing to do and ask user to
  create a plan before using this command
- Take your todo list (from your planning tool) and save it to a file in task/$2 (if $2
  ends with .md use it as-is, otherwise append .md)
- Follow the format in task/CLAUDE.md: short overview, then flat checkboxes grouped under
  phase headers. Keep it concise - one line per step, no code blocks in the plan.
- After creating the file, suggest that the user create a task space in VS Code to link to
  this file using Alt+Shift+T or the command palette

else if $1 is "update" then:

- If you have completed some tasks in your todo list that is related to task/$2 then find
  the appropriate step in the md file and mark it `[x]` for complete or `[ ]` for pending
- Then save the file
- If all the steps in the single task file are completed, then it is time to move this md
  file into the task/done/ folder and update any related todo.md and done.md files if
  necessary (if they contain links to this single md file)
- Suggest to the user that they can delete the task space in VS Code (Alt+Shift+T) which
  will automatically archive the task file to task/done/

else if $1 is "load" then:

- Make sure that you are in plan mode and your context is clear. If not then ask the user
  that these 2 things are requirements to run this command. There is nothing to do.
- Make sure the task/$2 file exists (append .md if $2 doesn't end with .md), and if it
  does not then tell the user that this file needs to exist for this slash command to
  work. There is nothing to do.
- Read the task/$2 file, then find the first unchecked (`[ ]`) step and resume work from
  there. Ask the user if they want to work on it. If they do, then begin work on that
  step.
- Remind the user to switch to the corresponding task space in VS Code (Alt+Shift+T) to
  have the right files open for this task

else if $1 is "done" then:

- Ask the user which approach they prefer:

    **Option A: Use VS Code extension (recommended if you have a task space)**
    - Deleting the task space automatically archives the file and handles cleanup

    **Option B: Move file directly**
    - I'll move task/$2 to task/done/ right now
    - You'll need to manually clean up any VS Code task space later

- If user chooses Option A:
    1. Press Alt+Shift+T (or Command Palette → "R3BL Task Management: Manage Task Spaces")
    2. Find the task space linked to task/$2 in the dialog
    3. Click the delete button (🗑️) next to that task space
    4. Confirm the deletion
    - Explain that deleting the task space will automatically:
        - Archive the task file to task/done/
        - Handle filename collisions (adds numeric suffix if needed)
        - Clean up the task space properly

- If user chooses Option B:
    - Move task/$2 to task/done/ (create task/done/ if it doesn't exist)
    - Handle filename collisions by adding numeric suffix if needed (e.g., task-name-1.md)
    - Confirm the move was successful
    - Remind user to delete any associated VS Code task space manually

## Notes

- This command works in conjunction with the R3BL Task Management VS Code extension
- Task files are stored in the `task/` directory
- Completed tasks are archived to `task/done/` directory
- The VS Code extension can create task spaces linked to these task files
- Use Alt+Shift+T in VS Code to manage task spaces
