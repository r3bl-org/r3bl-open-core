# r3bl-cmdr

## Why R3BL?

<img
src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/r3bl-cmdr-eap.svg?raw=true"
height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color: #FD2F53; background: linear-gradient(90deg, #FD2F53, #285BFB); -webkit-background-clip: text; background-clip: text; -webkit-text-fill-color: transparent;">R3BL TUI library &amp; suite of apps focused on developer productivity</span>

## Table of contents

<!-- TOC -->

- [Introduction](#introduction)
- [Installation](#installation)
- [Run `giti` binary target](#run-giti-binary-target)
- [Run `edi` binary target](#run-edi-binary-target)
- [Run `env-source` binary target](#run-env-source-binary-target)
- [Changelog](#changelog)
- [Learn how these crates are built, provide
  feedback](#learn-how-these-crates-are-built-provide-feedback)

<!-- /TOC -->

## Introduction

`r3bl-cmdr` is a suite of fully async TUI & CLI developer productivity tools built on
[`r3bl_tui`]. It is cross-platform and runs natively on Linux, macOS, and Windows.

The apps that comprise `r3bl-cmdr` will make you smile and make you more productive:

- 😺 `giti` (Early access preview 🐣) - An interactive git CLI app designed to give
  you more confidence and a better experience when working with git.
  - Fully async (never blocks the main thread, built on [`r3bl_tui`])
  - Visual branch selection
  - Streamlined commit workflows

- 🦜 `edi` (Early access preview 🐣) - A TUI Markdown editor that lets you edit
  Markdown files in your terminal in style.
  - Fully async (never blocks the main thread, built on [`r3bl_tui`])
  - Gradient colors and smart terminal capability detection (gracefully degrades)
  - Smart list formatting and full emoji support
  - Language-specific syntax highlighting inside fenced code blocks
  - SSH optimized (only repaints what's changed)
  - Zero-copy gap buffer for responsive editing even in large files

- 📜 `env-source` (Released 🚀) - A fast cross-platform environment loader that
  evaluates shell scripts in a native subshell and exports environment variable diffs
  for your interactive shell.
  - Fast (~1.8 ms execution time, over 100x faster than [Python-based alternatives]).
  - Evaluates POSIX `sh` or Bash scripts on Linux/macOS and `.bat`/`cmd.exe` scripts
    on Windows.
  - Multiple output formats: Fish (`set -gx`), PowerShell (`$env:`), JSON, and Dotenv
    (`.env`).
  - Seamless shell startup integration (pipe output directly into `source`).

## Installation

To install `r3bl-cmdr` on your system, run the following command, assuming you have
`cargo` on your system:

```bash
cargo install r3bl-cmdr
```

If you don't have `cargo` on your system, you can either:

1. Follow these [instructions] to install `cargo` on your system first (available for
   Linux, macOS, and Windows). Then run `cargo install r3bl-cmdr` to install this
   crate.

2. Build the binaries from the crate's source code. First clone this [repo]. Then,
   run:

   ```bash
   git clone https://github.com/r3bl-org/r3bl-open-core/ # clone the repo locally
   cd r3bl-open-core       # navigate to the repo root
   ./bootstrap.sh          # install all required tools
   cd cmdr/                # navigate to the cmdr crate
   cargo install --path .  # after install, the binaries are in ~/.cargo/bin
   ```

## Run `giti` binary target

<!--
giti branch video
Source: https://github.com/nazmulidris/developerlife.com/issues/5
Source mp4: https://github.com/nazmulidris/developerlife.com/assets/2966499/262f59d1-a95c-4af3-accf-c3d6cac6e586
-->
![giti
video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/giti.gif?raw=true)

- Run `giti` from anywhere on your system.
- Try `giti --help` to see the available commands.
- To delete one or more branches in your repo run `giti branch delete`.
- To checkout a branch run `giti branch checkout`.
- To create a new branch run `giti branch new`.
- If you want to generate log output for `giti`, run `giti -l`. For example, `giti -l
  branch delete`. To view this log output run `fish run.fish log`.

## Run `edi` binary target

<!--
edi video
Source: https://github.com/nazmulidris/developerlife.com/issues/6
Source mp4: https://github.com/nazmulidris/developerlife.com/assets/2966499/f2c4b07d-b5a2-4f41-af7a-06d1b6660c41
-->
![edi
video](https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/videos/edi.gif?raw=true)

- Run `edi` from anywhere on your system.
- Try `edi --help` to see the available commands.
- To open an existing file, run `edi <file_name>`. For example, `edi README.md`.
- If you want to generate log output for `edi`, run `edi -l`. For example, `edi -l
  README.md`. To view this log output run `fish run.fish log`.

## Run `env-source` binary target

- Run `env-source` from anywhere on your system.
- Try `env-source --help` to see the available options and flags.

## Changelog

Please check out the [changelog] to see how the crate has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate and explore the broader ecosystem:
- Read the main [README.md] of the `r3bl-open-core` monorepo and workspace to get a
  better understanding of the context in which this crate is meant to exist.
- If you like consuming video content, here's our [YT channel]. Please consider
  [subscribing].
- If you like consuming written content, here's our developer [site].

[changelog]: https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl-cmdr
[instructions]: https://rustup.rs/
[Python-based alternatives]: https://github.com/edc/bass
[README.md]: https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md
[repo]: https://github.com/r3bl-org/r3bl-open-core/
[site]: https://developerlife.com/
[subscribing]: https://www.youtube.com/channel/UCMcsxfCwzwDevc3NRqFgfEg?sub_confirmation=1
[YT channel]: https://www.youtube.com/@developerlifecom

License: Apache-2.0
