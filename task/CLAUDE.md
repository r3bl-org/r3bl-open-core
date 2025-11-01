# Rules for managing claude context and (long running) tasks

Tasks are detailed instructions to guide claude code to implement things. One long running task is
contained in a single .md file.

## Structure of a "single task" md file

```md
<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Overview

<The task overview and other architecture and high level details and the "why" go here.

# Implementation plan

<All the detailed execution / implementation steps go here>
```

Use doctoc to make sure the table of contents stays up to date. And use prettier to make sure the md
file is formatted correctly.

## How to manage the Implementation plan section

This task md file must be organized to make it easy to add, remove, update tasks and sub tasks and
sub sub tasks, etc. So the following organization approach is required, of which this is an example:

```md
# Implementation plan

## Step 0: Do FOO

Details about FOO

### Step 0.0: Do BAR

Details about BAR

### Step 0.1: Do BAZ

Details about BAZ

## Step 1: Do FOOBAR

Details about FOOBAR

### Step 1.1: Do FOOBARBAZ

Details about FOOBARBAZ
```

This organization of headings makes it easy to inject new sub tasks and mark sub tasks. It also
makes it easy to mark each sub task or task with the following status codes added at the "Step xyx"
heading levels: COMPLETE or BLOCKED or DEFERRED or WORK_IN_PROGRESS.

Here's an example w/ status codes added:

```md
# Implementation plan

## Step 0: Do FOO [COMPLETE]

Details about FOO

### Step 0.0: Do BAR [COMPLETE]

Details about BAR

### Step 0.1: Do BAZ [BLOCKED]

Details about BAZ

## Step 1: Do FOOBAR [WORK_IN_PROGRESS]

Details about FOOBAR

### Step 1.1: Do FOOBARBAZ [DEFERRED]

Details about FOOBARBAZ
```

This makes it very easy to understand what is being worked on now, what work is completed, and how
much is left to do. This makes handing off work from one agent to another or one developer to
another seamless.

## Folder structure

There are other folders in the ./task/ folder:

- "archive/" - This is where tasks which we have decided not work on but want to retain for
  historical reasons are moved to. If we don't care about retaining history for a task we don't
  intend to work on then we can just delete it.
- "pending/" - This is where tasks which we are NOT currently working on, but do intend to work on
  in the future are stored. This ensures that the files in ./task/ folder are those which we
  (agents, subagents, and other developers) are are currently working on.
- "done/" - This is where task which have been completed are moved to.
