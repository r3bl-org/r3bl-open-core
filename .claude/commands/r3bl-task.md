# Manage a task 1) [create, update, load] 2) [task_name]

## Arguments $1 and $2

$1 is the command to execute: "create", "update", "load"

$2 is "do_something.md" or "do_something", the file you create is
"./task/do_something.md"

## Algorithm to execute $1 and $2

if $1 is "create" then:
- If you have a plan then continue, otherwise there is nothing to do and
  ask user to create a plan before using this command
- Take your detailed todo list (from your planning tool) and save it to a file
  in task/$2 (if $2 ends with .md use it as-is, otherwise append .md)
- The filename should start with "task_" prefix, so if $2 is "foo" or "foo.md",
  create "task/task_foo.md"
- Inside of this file, create a comprehensive markdown document with:
  - Title (# Task: [Feature Name])
  - Overview section explaining what needs to be done
  - Detailed step-by-step implementation plan from your todo list
  - Any relevant technical considerations
  - Links to related files or documentation
- After creating the file, suggest that the user create a task space in VS Code
  to link to this file using Alt+Shift+T or the command palette

else if $1 is "update" then:
- If you have completed some tasks in your todo list that is related to task/$2
  then find the appropriate step in the md file and update its progress with whatever
  status code is appropriate (e.g., mark sections as COMPLETE, WORK_IN_PROGRESS, BLOCKED, DEFERRED)
- Then save the file
- If all the steps in the single task file are completed, then it
  is time to move this md file into the task/done/ folder and update
  any related todo.md and done.md files if necessary (if they contain links to this
  single md file)
- Suggest to the user that they can delete the task space in VS Code (Alt+Shift+T)
  which will automatically archive the task file to task/done/

else if $1 is "load" then:
- Make sure that you are in plan mode and your context is clear. If not then ask the
  user that these 2 things are requirements to run this command. There is nothing to do.
- Make sure the task/$2 file exists (with task_ prefix), and if it does not then tell the user
  that this file needs to exist for this slash command to work. There is nothing to do.
- Read the task/$2 file, then locate the step heading which is marked "WORK_IN_PROGRESS"
  if this exists. And resume executing the tasks in that step. If nothing is marked "WORK_IN_PROGRESS"
  then pick the first step that is not marked "COMPLETE" or "DEFERRED" or "BLOCKED" and ask the user
  if they want to work on it. If they do, then begin work on that step.
- Remind the user to switch to the corresponding task space in VS Code (Alt+Shift+T)
  to have the right files open for this task

## Notes

- This command works in conjunction with the R3BL Task Management VS Code extension
- Task files are stored in the `task/` directory
- Completed tasks are archived to `task/done/` directory
- The VS Code extension can create task spaces linked to these task files
- Use Alt+Shift+T in VS Code to manage task spaces
