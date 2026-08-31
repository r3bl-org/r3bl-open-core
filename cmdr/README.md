# r3bl-cmdr

## Why R3BL?

<img
src="https://raw.githubusercontent.com/r3bl-org/r3bl-open-core/main/cmdr/r3bl-cmdr-eap.svg?raw=true"
height="256px">

<!-- R3BL TUI library & suite of apps focused on developer productivity -->

<span style="color:#FD2F53">R</span><span style="color:#FC2C57">3</span><span
style="color:#FB295B">B</span><span style="color:#FA265F">L</span><span
style="color:#F92363"> </span><span style="color:#F82067">T</span><span
style="color:#F61D6B">U</span><span style="color:#F51A6F">I</span><span
style="color:#F31874"> </span><span style="color:#F11678">l</span><span
style="color:#EF137C">i</span><span style="color:#ED1180">b</span><span
style="color:#EB0F84">r</span><span style="color:#E90D89">a</span><span
style="color:#E60B8D">r</span><span style="color:#E40A91">y</span><span
style="color:#E10895"> </span><span style="color:#DE0799">&amp;</span><span
style="color:#DB069E"> </span><span style="color:#D804A2">s</span><span
style="color:#D503A6">u</span><span style="color:#D203AA">i</span><span
style="color:#CF02AE">t</span><span style="color:#CB01B2">e</span><span
style="color:#C801B6"> </span><span style="color:#C501B9">o</span><span
style="color:#C101BD">f</span><span style="color:#BD01C1"> </span><span
style="color:#BA01C4">a</span><span style="color:#B601C8">p</span><span
style="color:#B201CB">p</span><span style="color:#AE02CF">s</span><span
style="color:#AA03D2"> </span><span style="color:#A603D5">f</span><span
style="color:#A204D8">o</span><span style="color:#9E06DB">c</span><span
style="color:#9A07DE">u</span><span style="color:#9608E1">s</span><span
style="color:#910AE3">e</span><span style="color:#8D0BE6">d</span><span
style="color:#890DE8"> </span><span style="color:#850FEB">o</span><span
style="color:#8111ED">n</span><span style="color:#7C13EF"> </span><span
style="color:#7815F1">d</span><span style="color:#7418F3">e</span><span
style="color:#701AF5">v</span><span style="color:#6B1DF6">e</span><span
style="color:#6720F8">l</span><span style="color:#6322F9">o</span><span
style="color:#5F25FA">p</span><span style="color:#5B28FB">e</span><span
style="color:#572CFC">r</span><span style="color:#532FFD"> </span><span
style="color:#4F32FD">p</span><span style="color:#4B36FE">r</span><span
style="color:#4739FE">o</span><span style="color:#443DFE">d</span><span
style="color:#4040FE">u</span><span style="color:#3C44FE">c</span><span
style="color:#3948FE">t</span><span style="color:#354CFE">i</span><span
style="color:#324FFD">v</span><span style="color:#2E53FD">i</span><span
style="color:#2B57FC">t</span><span style="color:#285BFB">y</span>

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

- 📜 `env-source` (Released 🚀) - A fast cross-platform environment loader that evaluates
  shell scripts in a native subshell and exports environment variable diffs for your
  interactive shell.
  - Fast (~1.8 ms execution time, over 100x faster than [Python-based
    alternatives](https://github.com/edc/bass)).
  - Evaluates POSIX `sh` or Bash scripts on Linux/macOS and `.bat`/`cmd.exe` scripts
    on Windows.
  - Multiple output formats: Fish (`set -gx`), PowerShell (`$env:`), JSON, and Dotenv
    (`.env`).
  - Seamless shell startup integration (pipe output directly into `source`).

- 🐒 `rc` (Under Construction 🚧) - A unified developer hub and interactive command
  launcher for the `r3bl-cmdr` suite.
  - This is currently just a placeholder, pointing to [issue
    #363](https://github.com/r3bl-org/r3bl-open-core/issues/363).

## Installation

To install `r3bl-cmdr` on your system, run the following command, assuming you have
`cargo` on your system:

```bash
cargo install r3bl-cmdr
```

If you don't have `cargo` on your system, you can either:

1. Follow these [instructions](https://rustup.rs/) to install `cargo` on your system
   first (available for Linux, macOS, and Windows). Then run `cargo install r3bl-cmdr`
   to install this crate.

2. Build the binaries from the crate's source code. First clone this
   [repo](https://github.com/r3bl-org/r3bl-open-core/). Then, run:

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

Please check out the
[changelog](https://github.com/r3bl-org/r3bl-open-core/blob/main/CHANGELOG.md#r3bl-cmdr)
to see how the crate has evolved over time.

## Learn how these crates are built, provide feedback

To learn how we built this crate and explore the broader ecosystem:
- Read the main
  [README.md](https://github.com/r3bl-org/r3bl-open-core/blob/main/README.md) of the
  `r3bl-open-core` monorepo and workspace to get a better understanding of the context
  in which this crate is meant to exist.
- If you like consuming video content, here's our [YT
  channel](https://www.youtube.com/@developerlifecom). Please consider
  [subscribing](https://www.youtube.com/channel/UCMcsxfCwzwDevc3NRqFgfEg?sub_confirmation=1).
- If you like consuming written content, here's our developer
  [site](https://developerlife.com/).

License: Apache-2.0
