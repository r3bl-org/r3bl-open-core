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
  in @task/$2
- Inside of this file make sure that you follow the rules in the @task/CLAUDE.md file
  to create it

else if $1 is "update" then:
- If you have completed some tasks in your todo list that is related to @task/$2
  then find the appropriate step in the md file and update its progress with whatever
  status code is appropriate. Then save the file.

else if $1 is "load" then:
- Make sure that you are in plan mode and your context is clear. If not then ask the
  user that these 2 things are requirements to run this command. There is nothing to do.
- Make sure the @task/$2 file exists, and if it does not then tell the user
  that this file needs to exist for this slash command to work. There is nothing to do.
- Read the @task/$2 file, then locate the step heading which is marked "WORK_IN_PROGRESS"
  if this exists. And resume executing the tasks in that step. If nothing is marked "WORK_IN_PROGRESS"
  then pick the first step that is not marked "COMPLETE" or "DEFERRED" or "BLOCKED" and ask the user 
  if they want to work on it. If they do, then being work on that step.
