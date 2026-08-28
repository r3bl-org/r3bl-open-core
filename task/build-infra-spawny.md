# Task: Build-Infra Spawny Systemd-nspawn Machine Manager

<!-- BEGIN mktoc -->

- [Overview](#overview)
    - [Systemd-nspawn Clean-Room Testing](#systemd-nspawn-clean-room-testing)
    - [Standardized mkosi Image Pipeline](#standardized-mkosi-image-pipeline)
    - [Explicit Stateless vs Stateful CLI](#explicit-stateless-vs-stateful-cli)
- [Lifecycle Flowcharts & Mental Model](#lifecycle-flowcharts--mental-model)
    - [1. Storage & Zygote Mental Model](#1-storage--zygote-mental-model)
    - [2. Stateless Execution Flow (Clean-Room)](#2-stateless-execution-flow-clean-room)
    - [3. Stateful Execution Flow (Persistent Sandbox)](#3-stateful-execution-flow-persistent-sandbox)
    - [4. Interactive TUI Launcher Flow](#4-interactive-tui-launcher-flow)
- [Architecture](#architecture)
    - [1. mkosi Image Builder](#1-mkosi-image-builder)
    - [2. Machine & Zygote Engine](#2-machine--zygote-engine)
    - [3. CLI & Command Hierarchy](#3-cli--command-hierarchy)
    - [4. Interactive TUI Integration](#4-interactive-tui-integration)
    - [5. Spawny Binary Entry Point](#5-spawny-binary-entry-point)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: mkosi Configuration & Prereq Checks](#phase-1-mkosi-configuration--prereq-checks)
    - [Phase 2: Core nspawn & Zygote Engine](#phase-2-core-nspawn--zygote-engine)
    - [Phase 3: CLI Parser & TUI Layer](#phase-3-cli-parser--tui-layer)
    - [Phase 4: Stateless & Stateful Runners](#phase-4-stateless--stateful-runners)
    - [Phase 5: Binary Integration & Self-Upgrade](#phase-5-binary-integration--self-upgrade)
    - [Phase 6: Remove Legacy cmdr nspawn Scripts](#phase-6-remove-legacy-cmdr-nspawn-scripts)
    - [Phase 7: Migrate & Clean Up External Test Suite](#phase-7-migrate--clean-up-external-test-suite)
    - [Phase 8: Verification & Testing](#phase-8-verification--testing)
- [Verification Matrix](#verification-matrix)
    - [Distro Coverage Matrix](#distro-coverage-matrix)
    - [Command Verification Checklist](#command-verification-checklist) <!-- END mktoc -->

## Overview

Build `spawny`, a native Rust systemd-nspawn machine manager and clean-room test harness
inside `r3bl-build-infra`. `spawny` replaces fragile shell scripts with a type-safe,
general-purpose CLI tool that manages multi-distro Linux containers (Ubuntu, Fedora, Arch)
for automated release testing, installer script validation, and interactive debugging.

### Systemd-nspawn Clean-Room Testing

**Native Linux Isolation with Instant Restores**: `spawny` leverages `systemd-nspawn` and
the Zygote pattern to deliver fast, daemon-less container testing:

- **Daemonless and Native**: Direct kernel namespaces and cgroups without Docker daemon
  overhead.
- **Instant Golden Snapshot Restores**: Uses BTRFS subvolume snapshots or rsync to restore
  clean machine root filesystems in under 1 second.
- **Multi-Distro Validation**: Runs tests simultaneously across Ubuntu 24.04, Fedora 41,
  and Arch Linux.
- **General-Purpose Design**: Built to serve R3BL workspace testing first, then published
  as a reusable tool for any Rust or Linux project.

### Standardized mkosi Image Pipeline

**Declarative OS Image Generation**: Replaces legacy, ad-hoc image download scripts with
`mkosi` (official systemd project tool):

- **Declarative Distro Configs**: Standardized `.conf` files (`mkosi.ubuntu.conf`,
  `mkosi.fedora.conf`, `mkosi.arch.conf`).
- **Unified Post-Installation**: Shared post-installation hook (`mkosi.postinst`) to
  configure users, mirrors, and base development utilities cleanly.
- **Zero URL Brittleness**: Eliminates fragile date-guessing scrapers and custom tarball
  extraction hacks.

### Explicit Stateless vs Stateful CLI

**Unambiguous Command Namespaces**: `spawny stateless` and `spawny stateful` makes the
mental model crystal clear:

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│                                   SPAWNY                                    │
╰─────────────────────────────────────────────────────────────────────────────╯
        │                                                     │
        ▼                                                     ▼
┌───────────────────────────────────────┐   ┌─────────────────────────────────┐
│          `spawny stateless`           │   │        `spawny stateful`        │
│   (100% Clean-Room & Throwaway)       │   │    (100% Persistent Sandbox)    │
├───────────────────────────────────────┤   ├─────────────────────────────────┤
│ • install --script <path|url>         │   │ • exec <distro> "<command>"     │
│ • install --cargo <crate>             │   │ • shell [<distro>] (choose)     │
│ • run "<command>"                     │   │ • reset [<distro|all>]          │
│ • script <path>                       │   │ • list / status                 │
│                                       │   │ • start / stop <distro>         │
├───────────────────────────────────────┤   ├─────────────────────────────────┤
│ • ALWAYS auto-restores clean zygote   │   │ • NEVER auto-restores           │
│ • Parallel across distros by default  │   │ • State accumulates from step 1 │
│ • Leaves zero disk residue            │   │ • Ideal for step-by-step debug  │
└───────────────────────────────────────┘   └─────────────────────────────────┘
```

#### Command Overview

##### 1. Setup & Lifecycle

- `spawny setup [--distro <ubuntu|fedora|arch|all>]`: Runs `mkosi` to build root
  filesystems and creates golden zygotes.
- `spawny teardown [--distro <ubuntu|fedora|arch|all>]`: Stops containers, unregisters
  machines, and cleans up disk artifacts.

##### 2. Stateless Subcommands (Clean-Room Testing)

- `spawny stateless install --script <path|url>`: Restores clean zygotes in parallel ->
  runs installer -> verifies binary -> cleans up.
- `spawny stateless install --cargo <crate>`: Restores clean zygotes ->
  `cargo install <crate>` -> verifies `<bin> --version` / `--help` -> cleans up.
- `spawny stateless run "<command>"`: Restores clean zygotes -> executes command in
  parallel -> captures output -> cleans up.
- `spawny stateless script <test_suite.sh>`: Restores clean zygotes -> runs test suite ->
  captures report.

##### 3. Stateful Subcommands (Interactive Sandbox)

- `spawny stateful exec <distro> "<command>"`: Runs command inside active container; all
  changes persist on disk.
- `spawny stateful shell [<distro>]`: Opens interactive TTY login shell (launches
  `r3bl_tui::choose()` if distro is omitted).
- `spawny stateful reset [<distro|all>]`: Explicitly reverts active machine(s) back to the
  pristine golden zygote in `<1s`.
- `spawny stateful list`: Renders a formatted `r3bl_tui` table of all machines, runtime
  states (Running/Stopped), IPs, and zygotes.
- `spawny stateful start / stop <distro>`: Boots or halts the `systemd-nspawn` container
  daemon.

---

## Lifecycle Flowcharts & Mental Model

### 1. Storage & Zygote Mental Model

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SPAWNY STORAGE ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌────────────────────────┐                                                │
│   │ mkosi Build Pipeline   │                                                │
│   │ (Declarative .conf)    │                                                │
│   └───────────┬────────────┘                                                │
│               │                                                             │
│               ▼                                                             │
│   ┌────────────────────────────────────────┐                                │
│   │ Golden Zygote Snapshot                 │ (Read-Only / Pristine Template)│
│   │ /var/lib/spawny/zygotes/<distro>/      │                                │
│   └───────────┬────────────────────────────┘                                │
│               │                                                             │
│       Instant │ Restore (<1s via BTRFS subvolume snapshot / rsync)          │
│               ▼                                                             │
│   ┌────────────────────────────────────────┐                                │
│   │ Active Working Machine                 │ (Mutable Working Rootfs)       │
│   │ /var/lib/machines/spawny-<distro>/     │ Discovered by machinectl       │
│   └────────────────────────────────────────┘                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### 2. Stateless Execution Flow (Clean-Room)

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│ User Invokes: `spawny stateless [install | run | script]`                   │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 1. Instant Golden Restore (Parallel across Ubuntu, Fedora, Arch)            │
│    - Reverts `/var/lib/machines/spawny-<distro>` from `/var/lib/spawny/...` │
│    - Guarantees 100% clean, untouched root filesystem                       │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 2. Parallel Container Boot & Execution                                      │
│    - Spawns `systemd-nspawn` containers concurrently                        │
│    - Renders live `r3bl_tui::Spinner` progress for each distro              │
│    - Executes: installer script, `cargo install`, or test command           │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 3. Automated Validation & Smoke Tests                                       │
│    - Verifies exit status code == 0                                         │
│    - Executes smoke checks: `<binary> --version`, `<binary> --help`         │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 4. Automatic Teardown & Clean-Room Guarantee                                │
│    - Stops containers and purges execution artifacts                        │
│    - Renders summary table (Pass / Fail / Timings)                          │
│    - Leaves ZERO persistent disk pollution                                  │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

### 3. Stateful Execution Flow (Persistent Sandbox)

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│ Cumulative Step-by-Step Developer Workflow                                  │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 1: Install Package/Script (State Persists)                             │
│ `spawny stateful exec ubuntu "./install.sh"`                                │
│ ├─► Executes installer inside live `spawny-ubuntu` container                │
│ └─► Binaries placed in `/home/user/.cargo/bin/` REMAIN on disk              │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 2: Test Subsequent Actions on Accumulated State                        │
│ `spawny stateful exec ubuntu "giti --upgrade"`                              │
│ ├─► `giti` is already installed from Step 1!                                │
│ └─► Upgrades binary in-place; new version persists on disk                  │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 3: Interactive Container Shell                                         │
│ `spawny stateful shell ubuntu`                                              │
│ ├─► Drops developer into interactive TTY shell in container                 │
│ └─► Developer manually inspects logs, files, and environment                │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 4: Explicit Manual Reset (When Developer is Ready)                     │
│ `spawny stateful reset ubuntu`                                              │
│ └─► Instantly reverts `spawny-ubuntu` back to golden zygote                 │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

### 4. Interactive TUI Launcher Flow

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│ User runs `spawny` with no arguments OR omits target distro                 │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ `r3bl_tui::choose()` Interactive Selection Menu                             │
│                                                                             │
│ ┌ Select an Action / Target Machine ────────────────────────────┐           │
│ │ > 1. Ubuntu 24.04 (Noble)   [Running]                         │           │
│ │   2. Fedora 41              [Stopped - will boot]             │           │
│ │   3. Arch Linux (Rolling)   [Running]                         │           │
│ │   4. Run Clean-Room Release Test Suite                        │           │
│ └───────────────────────────────────────────────────────────────┘           │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

## Architecture

### 1. mkosi Image Builder

**Declarative OS Image Definitions in `build-infra/mkosi/`**:

- `mkosi.ubuntu.conf`: Ubuntu 24.04 LTS (Noble) package list and mirror configuration.
- `mkosi.fedora.conf`: Fedora 41 package list and repository configuration.
- `mkosi.arch.conf`: Arch Linux rolling release package list and pacman keyring
  configuration.
- `mkosi.postinst`: Shared post-install script creating standard test users, shell
  configuration, and base tools (`fish`, `curl`, `git`, `sudo`, `build-essential` /
  `base-devel`).

### 2. Machine & Zygote Engine

**Core Lifecycle Primitives in `build-infra/src/spawny/nspawn/`**:

- `distro.rs`: Distro enumeration (`Ubuntu`, `Fedora`, `Arch`), paths, and metadata.
- `machine.rs`: Machine lifecycle state machine (`NotFound`, `Stopped`, `Running`),
  starting, stopping, and process execution via `machinectl shell` / `nsenter`.
- `zygote.rs`: Golden snapshot management. Handles instant snapshot creation and
  restoration via BTRFS subvolumes (or rsync fallback) between `/var/lib/spawny/zygotes/`
  and `/var/lib/machines/spawny-<distro>/`.
- `prereqs.rs`: System requirement checks (`systemd-nspawn`, `machinectl`, `mkosi`,
  root/sudo).

### 3. CLI & Command Hierarchy

**Type-Safe Command Parser in `build-infra/src/spawny/cli/`**:

```text
spawny (cargo-spawny)
├── setup / build         [--distro <ubuntu|fedora|arch|all>] (builds images via mkosi and creates zygotes)
├── teardown              [--distro <ubuntu|fedora|arch|all>] (stops machines and purges disk images)
│
├── stateless             (100% clean-room, auto-restores zygote before run, zero leftover state)
│   ├── install           (--script <path|url> | --cargo <crate>) [--distro <d|all>]
│   ├── run               "<command>" [--distro <d|all>]
│   └── script            <test_script_path> [--distro <d|all>]
│
└── stateful              (100% persistent sandbox, state accumulates across steps, manual reset)
    ├── list / status     (displays table of machine states, IPs, zygote health)
    ├── exec              <distro> "<command>" (executes command; state persists)
    ├── shell             [<distro>] (interactive TTY login shell; prompts via choose() if omitted)
    ├── reset             [<distro|all>] (explicitly resets machine back to golden zygote)
    ├── start             <distro> (boots the machine daemon)
    └── stop              <distro> (stops the machine daemon)
```

### 4. Interactive TUI Integration

**Rich Terminal UX in `build-infra/src/spawny/tui/` Powered by `r3bl_tui`**:

- **Interactive Selection**: Uses `r3bl_tui::choose()` when commands are invoked without
  required arguments (e.g., selecting a distro for `spawny stateful shell`).
- **Interactive Launcher**: Invoking `spawny` with no arguments launches an interactive
  TUI dashboard.
- **Live Parallel Progress**: Uses `r3bl_tui::Spinner` and status line formatters to
  render real-time multi-distro test execution progress.
- **Formatted Status Tables**: Uses `r3bl_tui` styling to render machine status, IP
  addresses, and zygote health tables.

### 5. Spawny Binary Entry Point

**Binary Entry Point in `build-infra/src/bin/spawny.rs`**:

- Integrates with `r3bl-build-infra` package suite.
- Supports standalone `--upgrade` CLI flag wired to the shared
  `r3bl_tui::core::script::upgrade` engine, ensuring `spawny` seamlessly self-upgrades
  alongside `cargo-rustdoc-fmt`.

---

## Implementation Plan

### Phase 1: mkosi Configuration & Prereq Checks

**Declarative image definitions and environment validation**:

- [ ] Create `build-infra/mkosi/mkosi.ubuntu.conf` for Ubuntu 24.04 LTS.
- [ ] Create `build-infra/mkosi/mkosi.fedora.conf` for Fedora 41.
- [ ] Create `build-infra/mkosi/mkosi.arch.conf` for Arch Linux rolling.
- [ ] Create `build-infra/mkosi/mkosi.postinst` shared post-install setup script.
- [ ] Implement `build-infra/src/spawny/prereqs.rs` to validate `systemd-nspawn`,
      `machinectl`, `mkosi`, disk space, and root/sudo permissions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/mkosi/mkosi.ubuntu.conf`
    - [ ] `build-infra/mkosi/mkosi.fedora.conf`
    - [ ] `build-infra/mkosi/mkosi.arch.conf`
    - [ ] `build-infra/mkosi/mkosi.postinst`
    - [ ] `build-infra/src/spawny/prereqs.rs`

### Phase 2: Core nspawn & Zygote Engine

**Low-level machine and snapshot primitives**:

- [ ] Implement `build-infra/src/spawny/nspawn/distro.rs` defining supported
      distributions, paths, and machine name mappings.
- [ ] Implement `build-infra/src/spawny/nspawn/machine.rs` managing machine start, stop,
      reboot, status checking, and command execution (`nsenter` / `machinectl shell`).
- [ ] Implement `build-infra/src/spawny/nspawn/zygote.rs` managing golden snapshot
      creation and instant restore via BTRFS subvolumes (with rsync fallback).
- [ ] Implement `build-infra/src/spawny/nspawn/image_builder.rs` invoking `mkosi` to build
      root filesystems.
- [ ] Add unit tests for distro mapping, path resolution, and state transitions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/nspawn/distro.rs`
    - [ ] `build-infra/src/spawny/nspawn/machine.rs`
    - [ ] `build-infra/src/spawny/nspawn/zygote.rs`
    - [ ] `build-infra/src/spawny/nspawn/image_builder.rs`

### Phase 3: CLI Parser & TUI Layer

**Command-line configuration and interactive TUI components**:

- [ ] Implement `build-infra/src/spawny/cli/` using `clap` derive with `stateless` and
      `stateful` subcommand hierarchies and `--upgrade` support.
- [ ] Implement `build-infra/src/spawny/tui/choose_menu.rs` wrapping `r3bl_tui::choose()`
      for interactive distro and action selection.
- [ ] Implement `build-infra/src/spawny/tui/status_table.rs` formatting machine state
      tables.
- [ ] Implement `build-infra/src/spawny/tui/progress.rs` rendering live parallel
      multi-distro test progress via `r3bl_tui::Spinner`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/cli/mod.rs`
    - [ ] `build-infra/src/spawny/cli/args.rs`
    - [ ] `build-infra/src/spawny/tui/choose_menu.rs`
    - [ ] `build-infra/src/spawny/tui/status_table.rs`
    - [ ] `build-infra/src/spawny/tui/progress.rs`

### Phase 4: Stateless & Stateful Runners

**Execution orchestrators for test and sandbox workflows**:

- [ ] Implement `build-infra/src/spawny/runner/stateless_runner.rs`:
    - `install --script`: Restores clean zygotes, runs installer script, validates
      binaries, cleans up.
    - `install --cargo`: Restores clean zygotes, runs `cargo install`, validates smoke
      tests, cleans up.
    - `run "<command>"`: Executes arbitrary commands across distros in parallel with live
      status reporting.
    - `script <path>`: Runs test suite scripts inside clean containers.
- [ ] Implement `build-infra/src/spawny/runner/stateful_runner.rs`:
    - `exec <distro> "<command>"`: Executes command inside active container with
      persistent state.
    - `shell [<distro>]`: Opens interactive TTY shell.
    - `reset [<distro|all>]`: Reverts active container back to golden zygote state.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/runner/stateless_runner.rs`
    - [ ] `build-infra/src/spawny/runner/stateful_runner.rs`
    - [ ] `build-infra/src/spawny/runner/mod.rs`

### Phase 5: Binary Integration & Self-Upgrade

**Binary entry point and packaging**:

- [ ] Add `spawny` binary to `build-infra/Cargo.toml` (`[[bin]]`).
- [ ] Implement `build-infra/src/bin/spawny.rs`.
- [ ] Wire `--upgrade` CLI argument interceptor to `r3bl_tui::core::script::upgrade::*`.
- [ ] Export `spawny` module in `build-infra/src/lib.rs`.
- [ ] Install binary locally via `cargo install --path build-infra --force` and verify
      `spawny --help`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/Cargo.toml`
    - [ ] `build-infra/src/bin/spawny.rs`
    - [ ] `build-infra/src/lib.rs`

### Phase 6: Remove Legacy cmdr nspawn Scripts

**Clean up obsolete workspace shell scripts**:

- [ ] Remove `cmdr/systemd-nspawn/` directory from the workspace.
- [ ] Remove `build-infra/reference/nspawn-scripts/` directory if no longer needed.
- [ ] Update `build-infra/AGENTS.md` and documentation to reference `spawny`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/` (verify `systemd-nspawn/` removed)
    - [ ] `build-infra/AGENTS.md`

### Phase 7: Migrate & Clean Up External Test Suite

**Migrate `~/github/notes/files/scripts/tests/` to use pre-installed `spawny`**:

- [ ] Migrate `/home/nazmul/github/notes/files/scripts/tests/` (and its E2E test scripts
      like `e2e/01-test-fresh-install-stateless.fish` and
      `e2e/02-test-fresh-install-stateful.fish`) to invoke the pre-installed `spawny`
      binary directly (installed via `fresh-install` /
      `cargo install --path build-infra --force`).
- [ ] Remove and retire the legacy fish test harness scripts in
      `/home/nazmul/github/notes/files/scripts/tests/` (`setup.fish`, `run.fish`,
      `teardown.fish`, `lib/nspawn.fish`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `/home/nazmul/github/notes/files/scripts/tests/` (verify migrated to `spawny`
          CLI calls)

### Phase 8: Verification & Testing

**Comprehensive validation of spawny across distros**:

- [ ] Run `spawny setup --distro all` and verify `mkosi` builds Arch, Ubuntu, and Fedora
      images.
- [ ] Run `spawny stateful list` and verify all machines and zygotes are registered.
- [ ] Run `spawny stateful exec ubuntu "echo hello"` and test state persistence.
- [ ] Run `spawny stateful reset ubuntu` and verify instant restore.
- [ ] Run `spawny stateless run --distro all "uname -a"` and verify parallel execution.
- [ ] Run `./check.fish --check`, `--build`, `--clippy`, `--test` across the workspace.
- [ ] Run migrated test suite in `/home/nazmul/github/notes/files/scripts/tests/` powered
      by `spawny`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/build-infra-spawny.md`

---

## Verification Matrix

### Distro Coverage Matrix

| Distribution   | Version / Release | Backend Tool            | Default User | Package Manager | Zygote Path                       |
| :------------- | :---------------- | :---------------------- | :----------- | :-------------- | :-------------------------------- |
| **Ubuntu**     | 24.04 LTS (Noble) | `mkosi` + `debootstrap` | `user`       | `apt`           | `/var/lib/spawny/zygotes/ubuntu/` |
| **Fedora**     | 41                | `mkosi` + `dnf`         | `user`       | `dnf`           | `/var/lib/spawny/zygotes/fedora/` |
| **Arch Linux** | Rolling           | `mkosi` + `pacman`      | `user`       | `pacman`        | `/var/lib/spawny/zygotes/arch/`   |

### Command Verification Checklist

- [ ] `spawny setup`: Builds all 3 distros using `mkosi` and creates golden snapshots.
- [ ] `spawny teardown`: Stops running containers and removes images.
- [ ] `spawny stateless install --script <path>`: Clean-room installer verification.
- [ ] `spawny stateless install --cargo <crate>`: Clean-room crates.io build verification.
- [ ] `spawny stateless run "<command>"`: Clean-room command runner across all distros.
- [ ] `spawny stateful exec <distro> "<command>"`: Persistent command execution in living
      sandbox.
- [ ] `spawny stateful shell <distro>`: Interactive TTY container shell.
- [ ] `spawny stateful reset <distro>`: Instant `<1s` reset to golden snapshot.
- [ ] `spawny stateful list`: Status table rendering via `r3bl_tui`.
- [ ] `spawny --upgrade`: Binary self-upgrade execution.
- [ ] Migrate `~/github/notes/files/scripts/tests/` to use `spawny` and purge obsolete
      fish test scripts.

<!-- cspell:words postinst Rootfs machinectl nsenter debootstrap -->
