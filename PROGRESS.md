# PROGRESS.md

This document is meant to capture context after I've been away from this project for some time.

Table of contents:

<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Context](#context)
- [Why?](#why)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Context

- In the `copypaste` branch, I've been working on editor component upgrades related to select, copy,
  paste, cut, support. Here's the [issue](https://github.com/r3bl-org/r3bl_rs_utils/issues/86).
  - Done:
    - Shift up and down are done
    - Shift page up and down are done
    - Shift home and end are done
  - Resume work on this todo item (just search for the following or <kbd>Alt+T</kbd>): `00:`:
    - Then implement actions: copy, cut, delete, paste
    - Do an audit of dependencies and make sure they are all compatible licenses and use the
      `r3bl_ansi_color` crate as well.
    - Then merge this branch to main.
- The [TODO.todo](TODO.todo) file has a list of things that need to be done and that are currently
  under way.
- After these changes have landed, I would really like to have better documentation and then publish
  a first version of `r3bl_cmdr` to crates.io, which I can use every day as my MD editor of choice.

# Why?

Using the example of a personal project, shortlink, it might entail:

1. Create a `PROGRESS.md` document in the repo where some context around what is to be done next is
   stored. This document is meant to be ready when coming back after being away from this project
   for a long time.
   - The `README.md` is a file that describes what the project is at a high level w/ some basic
     instructions on how to use it.
   - The `PROGRESS.md` file is more about resuming work on this project after a long time.
2. High level information about what needs to be done can be stored in a `TODO.todo` file. There's a
   nice
   [VSCode extension](https://marketplace.visualstudio.com/items?itemName=fabiospampinato.vscode-todo-plus)
   that can do some special handling of this type of file format.
3. Low level details about what needs to be done can be captured in github issues and linked to the
   `TODO.todo` file. Alternatively there's also
   [project view](https://github.com/orgs/r3bl-org/projects/1/views/1) in github that might be
   useful for this.

The keys are:

1. Recognize that I am excited.
2. Take a step back and account for what the opportunity is.
3. Figure out a path to get there sustainably.
4. Chunk things out into small enough pieces that "progress" can be made and write them down:
   `PROGRESS.md`, `TODO.todo`, github issues, github project view.
5. Have faith that there will be more moments in the future when I am excited and there is no
   scarcity of excitement or moments when I can work on things I am excited about.
