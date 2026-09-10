# Task: Build-Infra Spawny Systemd-nspawn Machine Manager

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Task: Build-Infra Spawny Systemd-nspawn Machine Manager](#task-build-infra-spawny-systemd-nspawn-machine-manager)
- [Overview](#overview)
  - [Systemd-nspawn Clean-Room Testing](#systemd-nspawn-clean-room-testing)
  - [Standardized mkosi Image Pipeline](#standardized-mkosi-image-pipeline)
  - [Privilege Model & Execution Safety](#privilege-model--execution-safety)
  - [Explicit Stateless vs Stateful CLI](#explicit-stateless-vs-stateful-cli)
    - [Command Overview](#command-overview)
      - [1. Setup, Status & Lifecycle](#1-setup-status--lifecycle)
      - [2. Stateless Subcommands (Clean-Room Testing)](#2-stateless-subcommands-clean-room-testing)
      - [3. Stateful Subcommands (Interactive Sandbox)](#3-stateful-subcommands-interactive-sandbox)
- [Lifecycle Flowcharts & Mental Model](#lifecycle-flowcharts--mental-model)
  - [1. Storage & Zygote Mental Model](#1-storage--zygote-mental-model)
  - [2. Stateless Execution Flow (Clean-Room)](#2-stateless-execution-flow-clean-room)
  - [3. Stateful Execution Flow (Persistent Sandbox)](#3-stateful-execution-flow-persistent-sandbox)
  - [4. Interactive TUI Launcher Flow](#4-interactive-tui-launcher-flow)
- [Architecture](#architecture)
  - [1. mkosi Image Builder](#1-mkosi-image-builder)
  - [2. Machine, Zygote & System Info Engine](#2-machine-zygote--system-info-engine)
  - [3. CLI & Command Hierarchy](#3-cli--command-hierarchy)
  - [4. Interactive TUI Integration](#4-interactive-tui-integration)
  - [5. Spawny Binary Entry Point](#5-spawny-binary-entry-point)
- [Implementation Plan](#implementation-plan)
  - [Phase 1: mkosi Configuration & Prereq Checks](#phase-1-mkosi-configuration--prereq-checks)
  - [Phase 2: Core nspawn, Zygote & System Info Engine](#phase-2-core-nspawn-zygote--system-info-engine)
  - [Phase 3: CLI Parser & TUI Layer](#phase-3-cli-parser--tui-layer)
  - [Phase 4: Stateless & Stateful Runners](#phase-4-stateless--stateful-runners)
  - [Phase 5: Binary Integration & Workspace Gating](#phase-5-binary-integration--workspace-gating)
  - [Phase 6: Remove Legacy cmdr nspawn Scripts & Update run.fish](#phase-6-remove-legacy-cmdr-nspawn-scripts--update-runfish)
  - [Phase 7: External Test Suite Migration in ~/github/notes](#phase-7-external-test-suite-migration-in-~githubnotes)
  - [Phase 8: Verification & Testing](#phase-8-verification--testing)
- [Verification Matrix](#verification-matrix)
  - [Distro Coverage Matrix](#distro-coverage-matrix)
  - [Command Verification Checklist](#command-verification-checklist)

<!-- END mktoc -->
<!-- prettier-ignore-end -->

## Overview

Build `spawny`, a native Rust systemd-nspawn machine manager and clean-room test harness
inside `r3bl-build-infra`. `spawny` replaces fragile shell scripts with a type-safe,
general-purpose CLI tool that manages multi-distro Linux containers (Ubuntu, Fedora, Arch)
for automated release testing, installer script validation, and interactive debugging.

> **Why Ubuntu, Fedora, and Arch?**
>
> Together, these three distributions represent the canonical "Big Three" Linux ecosystem
> archetypes with native, first-class `mkosi` support:
>
> 1. **Ubuntu (Debian / `apt` family)**: Default environment for GitHub Actions CI
>    (`ubuntu-latest`) and dominant developer desktop base (Pop!\_OS, Mint). Testing on
>    Ubuntu guarantees `.deb`/`apt` compatibility across real-world developer setups.
> 2. **Fedora (Red Hat / `dnf` family)**: The reference platform for modern `systemd`,
>    `dnf5`, and RPM packaging, upstream of RHEL, CentOS Stream, and Rocky Linux.
> 3. **Arch Linux (Rolling / `pacman` family)**: Canonical upstream for `pacman` and
>    bleeding-edge glibc/toolchains. Testing against pure upstream Arch ensures
>    out-of-the-box `mkosi` compatibility and guarantees software runs on all downstream
>    derivatives (CachyOS, EndeavourOS, Manjaro) without depending on custom kernel or
>    repository optimizations.

### Systemd-nspawn Clean-Room Testing

**Native Linux Isolation with Instant Restores**: `spawny` leverages `systemd-nspawn` and
the Zygote pattern to deliver fast, daemon-less container testing:

- **Daemonless and Native**: Direct kernel namespaces and cgroups without Docker daemon
  overhead.
- **Two-Tier Instant Restores**:
    - **Stateless (Clean-Room)**: Uses `systemd-nspawn --ephemeral` (`-x`). On BTRFS, uses
      instant kernel subvolume snapshots. On non-BTRFS (ext4/XFS), uses kernel OverlayFS
      backed by disk storage, discarding changes on container exit.
    - **Stateful (Sandbox Reset)**: Fast file-level Copy-on-Write via
      `cp -a --reflink=auto` (BTRFS/XFS, `<1s`) or `rsync -aAX --delete` (ext4 fallback,
      10-30s).
- **Multi-Distro Validation**: Runs tests simultaneously across Ubuntu 24.04, Fedora 41,
  and Arch Linux.
- **General-Purpose Design**: Built to serve R3BL workspace testing first, then published
  as a reusable tool for any Rust or Linux project.

### Standardized mkosi Image Pipeline

**Declarative OS Image Generation with Embedded Configs**: Replaces legacy, ad-hoc image
download scripts with `mkosi` (official systemd project tool):

- **Declarative Distro Configs**: Standardized configuration directories
  (`build-infra/mkosi/mkosi.conf` and `mkosi.profiles/<distro>/mkosi.conf`).
- **Binary Embedding**: Config files and scripts are embedded directly into the `spawny`
  binary via `include_dir!`. When `spawny setup` runs outside the repo tree, it extracts
  them to `/var/lib/spawny/mkosi/` automatically (overridable via `--config-dir`).
- **Reliable User Provisioning**: User `tester` (UID 1000, shell `/usr/bin/fish`) is
  created explicitly with home directory `/home/tester` via `useradd -m` in
  `mkosi.postinst.chroot`, with passwordless sudo (`/etc/sudoers.d/tester`) and
  `/home/tester/.cargo/bin` pre-configured in `/etc/environment` and `$PATH`.
- **Hermetic Post-Installation**: Distro-specific runtime customization strictly isolated
  inside `mkosi.postinst.chroot` (guaranteed by `mkosi` to run inside container chroot
  namespaces, never touching host accounts).
- **Modern Rust via Rustup**: The Rust toolchain is installed via `rustup` inside
  `mkosi.postinst.chroot` on all distributions, guaranteeing Rust 1.85+ (Rust 2024 edition
  compatible) across Ubuntu, Fedora, and Arch. Base tools (`fish`, `curl`, `git`, `sudo`,
  `build-essential` / `base-devel`) are pre-installed in golden images.

### Privilege Model & Execution Safety

- **Unprivileged CLI Execution**: `spawny` is executed as a regular user (supporting
  `cargo spawny` without running Cargo as root). Internal privileged operations
  (`systemd-nspawn`, `machinectl`, `nsenter`, file operations in `/var/lib/`) are elevated
  via `sudo`.
- **Upfront Sudo Pre-Validation**: Validates sudo access upfront (`sudo -v` in interactive
  mode, `sudo -n true` in non-interactive/CI mode).
- **Non-Interactive Fail-Fast**: If a non-interactive shell is detected (or
  `--non-interactive`/`--ci` is passed) and passwordless sudo is unavailable, `spawny`
  fails fast immediately with a clear diagnostic message.
- **Mount Safety Before Deletion**: Before deleting any machine directory during reset,
  clean, or teardown, `spawny` inspects `/proc/mounts`, cleanly unmounts any remaining
  child mounts, cleans up `/run/systemd/nspawn/unix-export/`, and resets failed systemd
  scopes, ensuring `rm -rf` never traverses into host files.

### Explicit Stateless vs Stateful CLI

**Unambiguous Command Namespaces**: `spawny stateless` and `spawny stateful` makes the
mental model crystal clear. Both modes feature **unified auto-boot**
(`systemd-nspawn -b`), ensuring `systemd` is PID 1, system services and daemons operate
consistently, and commands execute via `nsenter`:

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
│ • install --cargo <crate> [--bin <b>] │   │ • shell [<distro>] (choose)     │
│ • run "<command>"                     │   │ • reset [<distro|all>]          │
│ • script <path>                       │   │ • clean [<distro|all>] (crash)  │
│                                       │   │ • start / stop <distro>         │
├───────────────────────────────────────┤   ├─────────────────────────────────┤
│ • Auto-boots with -b and -x           │   │ • Auto-boots if stopped         │
│ • Unique machine ID per run           │   │ • State accumulates from step 1 │
│ • Automatically powers off on exit    │   │ • Machine remains running       │
│ • Unified multi-distro spinner        │   │ • Ideal for step-by-step debug  │
└───────────────────────────────────────┘   └─────────────────────────────────┘
```

#### Command Overview

##### 1. Setup, Status & Lifecycle

- `spawny setup [--distro <ubuntu|fedora|arch|all>] [--force]`: Checks/installs host
  dependencies, extracts embedded `mkosi` configs if needed, builds root filesystems, and
  creates golden zygotes.
- `spawny teardown [--distro <ubuntu|fedora|arch|all>]`: Stops containers, unmounts any
  active mounts, unregisters machines, and cleans up disk artifacts.
- `spawny status` / `spawny list`: Displays formatted `r3bl_tui` table of all machines,
  runtime states (Running/Stopped), IPs, and golden zygote status (aliased under
  `stateful`).

##### 2. Stateless Subcommands (Clean-Room Testing)

- `spawny stateless install --script <path|url> [--bind <h:c>] [--env <K=V>] [-w <dir>]`:
  Auto-boots an ephemeral container with a unique machine ID -> executes installer ->
  verifies binary -> powers off and discards changes.
- `spawny stateless install --cargo <crate> [--bin <bin>] [--env <K=V>]`: Auto-boots an
  ephemeral container -> runs `cargo install <crate>` -> verifies `<bin> --version` (or
  auto-detected binary) -> powers off and cleans up.
- `spawny stateless run "<command>" [--bind <h:c>] [--env <K=V>] [-w <dir>]`: Executes
  command across distros in parallel auto-booted ephemeral containers -> captures output
  -> powers off and cleans up.
- `spawny stateless script <test_suite.sh> [--bind <h:c>] [--env <K=V>] [-w <dir>]`:
  Mounts script and required parent directories into clean container -> runs test suite ->
  powers off and captures report.

##### 3. Stateful Subcommands (Interactive Sandbox)

- `spawny stateful exec <distro> "<command>" [--bind <h:c>] [--env <K=V>] [-w <dir>]`:
  Runs command inside container (auto-booting if stopped); all changes persist on disk.
- `spawny stateful shell [<distro>]`: Opens interactive TTY login shell with PTY
  allocation (auto-booting if stopped; prompts via `r3bl_tui::choose()` if distro
  omitted).
- `spawny stateful reset [<distro|all>]`: Stops machine, unmounts dangling mounts, and
  reverts active machine(s) back to pristine golden zygote (`<1s` via reflink/BTRFS,
  10-30s via rsync).
- `spawny stateful clean [<distro|all>]`: Recovers from unrecoverable crashes: force-kills
  stuck processes, unmounts lingering mounts, resets systemd scopes, and unregisters
  broken machines.
- `spawny stateful list` / `status`: Shows machine and zygote status table.
- `spawny stateful start / stop <distro>`: Manually boots or halts the container daemon.

---

## Lifecycle Flowcharts & Mental Model

### 1. Storage & Zygote Mental Model

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SPAWNY STORAGE ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│   ┌────────────────────────┐                                                │
│   │ mkosi Build Pipeline   │ (Declarative mkosi.conf + mkosi.extra/ +       │
│   │ (With Rust Toolchains) │  sysusers.d + mkosi.postinst.chroot)           │
│   └───────────┬────────────┘                                                │
│               │                                                             │
│               ▼                                                             │
│   ┌────────────────────────────────────────┐                                │
│   │ Golden Zygote Template                 │ (Read-Only Template on Disk)   │
│   │ /var/lib/spawny/zygotes/<distro>/      │ Never loaded into RAM          │
│   └───────────┬────────────────────────────┘                                │
│               │                                                             │
│       ┌───────┴─────────────────────────────────────────────┐               │
│       │                                                     │               │
│       ▼ (Stateless Execution)                               ▼ (Stateful)    │
│   ┌────────────────────────────────────────┐   ┌────────────────────────┐   │
│   │ Ephemeral Container Mount              │   │ Working Machine Rootfs │   │
│   │ (systemd-nspawn -x / --ephemeral)      │   │ /var/lib/machines/     │   │
│   │ - BTRFS: instant kernel CoW snapshot   │   │   spawny-<distro>/     │   │
│   │ - ext4/XFS: kernel OverlayFS on disk   │   │ - Reflink CoW (<1s)    │   │
│   │ - 0 RAM consumed by zygote; discarded  │   │ - rsync fallback       │   │
│   │   100% on container exit               │   │ - Manual stateful reset│   │
│   └────────────────────────────────────────┘   └────────────────────────┘   │
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
│ 1. Upfront Sudo Pre-Validation & Capacity Deduction                         │
│    - Pre-validates sudo credentials (sudo -v / sudo -n true); fails fast    │
│      if non-interactive and passwordless sudo is unavailable                │
│    - Deduces --max-parallel (CPU cores & RAM available; avoids OOM)         │
│    - Interactivity check (check_is_terminal_interactive())                  │
│    - Prepares unified coordinator Spinner (interactive) or logs (CI)        │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 2. Ephemeral Container Auto-Boot & Execution                                │
│    - Generates unique machine ID (spawny-stateless-<distro>-<uuid>)         │
│    - Spawns `systemd-nspawn -b -x` with --resolv-conf=copy-uplink           │
│    - Mounts host paths via --bind / --bind-ro; sets --workdir               │
│    - Waits for systemd boot readiness                                       │
│    - Executes via nsenter (as user tester UID 1000): installer, cargo       │
│      install, or test command                                               │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 3. Automated Validation & Smoke Tests                                       │
│    - Verifies command exit status code == 0                                 │
│    - Smoke checks: `<bin> --version`, `<bin> --help` (auto-detects binary   │
│      name or uses --bin flag; ensures /home/tester/.cargo/bin in PATH)      │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 4. Clean-Room Teardown & Reporting                                          │
│    - Cleanly powers off container; ephemeral layer discarded on shutdown    │
│    - Renders unified summary table (Pass / Fail / Timings / Captured logs)  │
│    - Leaves ZERO persistent disk pollution or residual states               │
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
│ Step 1: Install Package/Script (Auto-Boots if Stopped, State Persists)       │
│ `spawny stateful exec ubuntu "./install.sh"`                                │
│ ├─► Detects if spawny-ubuntu is running; auto-boots if stopped (-b)         │
│ ├─► Executes installer via nsenter (as tester, working in --workdir)        │
│ └─► Binaries placed in `/home/tester/.cargo/bin/` REMAIN on disk            │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 2: Test Subsequent Actions on Accumulated State                        │
│ `spawny stateful exec ubuntu "giti status"`                                 │
│ ├─► Container is already booted and running from Step 1                     │
│ ├─► `giti` is already installed and in PATH                                 │
│ └─► Modifies state in-place; changes persist on disk                        │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 3: Interactive Container Shell                                         │
│ `spawny stateful shell ubuntu`                                              │
│ ├─► Auto-boots container if stopped; allocates PTY via script wrapper       │
│ ├─► Drops developer into interactive TTY shell as user tester               │
│ └─► Developer manually inspects logs, files, and environment                │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 4: Mount-Safe Reset OR Crash Recovery                                  │
│ `spawny stateful reset ubuntu` OR `spawny stateful clean ubuntu`            │
│ ├─► Checks /proc/mounts, unmounts lingering mounts, resets failed scopes    │
│ ├─► reset: Reverts `spawny-ubuntu` back to golden zygote (reflink/rsync)    │
│ └─► clean: Force-kills stuck nspawn procs, clears scopes/stale mounts       │
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
│ `check_is_terminal_interactive()` Gate                                      │
│ ├─► If Not Interactive: Fails fast with argument usage error                │
│ └─► If Interactive: Launches contextual `r3bl_tui::choose()` Selection:    │
│                                                                             │
│ ┌ Select an Action / Target Machine ────────────────────────────┐           │
│ │ > 1. Ubuntu 24.04 (Noble)   [Running: open shell]             │           │
│ │   2. Fedora 41              [Stopped: will boot and attach]   │           │
│ │   3. Arch Linux (Rolling)   [Running: open shell]             │           │
│ │   4. Run Clean-Room Release Test Suite across all distros     │           │
│ └───────────────────────────────────────────────────────────────┘           │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

## Architecture

### 1. mkosi Image Builder

**Declarative OS Image Definitions in `build-infra/mkosi/` (Embedded in Binary)**:

- `mkosi.conf`: Shared base configuration across all target distributions.
- `mkosi.profiles/`:
    - `ubuntu/mkosi.conf`: Ubuntu 24.04 LTS (Noble) package list and mirror configuration.
    - `fedora/mkosi.conf`: Fedora 41 package list and repository configuration.
    - `arch/mkosi.conf`: Arch Linux rolling release package list and pacman keyring
      configuration.
- `mkosi.postinst.chroot`: Hermetic post-installation script executed inside container
  chroot namespaces:
    - Creates user `tester` (UID 1000, shell `/usr/bin/fish`) explicitly via `useradd -m`.
    - Configures passwordless sudo in `/etc/sudoers.d/tester`.
    - Installs modern Rust via `rustup` for user `tester`, ensuring Rust 1.85+ (Rust 2024
      edition compatibility).
    - Sets up `/etc/environment` to include `/home/tester/.cargo/bin` and
      `/home/tester/.local/bin` in system and user `$PATH`.
- Pre-baked development tools in all images: Rust toolchain (via `rustup`), `fish`, `curl`,
  `git`, `sudo`, `build-essential` / `base-devel`.
- **Binary Embedding via `include_dir!`**: Config files are embedded into `spawny`. On
  `spawny setup`, files are unpacked to `/var/lib/spawny/mkosi/` if not running from the
  git repo root.

### 2. Machine, Zygote & System Info Engine

**Core Lifecycle Primitives in `build-infra/src/spawny/nspawn/`**:

- `distro.rs`: Distro enumeration (`Ubuntu`, `Fedora`, `Arch`), paths, and container
  metadata. Generates unique machine IDs for stateless runs (`spawny-stateless-<distro>-<uuid>`)
  and persistent machine names (`spawny-<distro>`) for stateful runs.
- `system_info.rs`: Unified host system introspection and hardware capacity engine:
    - Sudo pre-validation: Validates `sudo -v` (interactive) or `sudo -n true` (non-interactive).
      Fails fast if non-interactive and passwordless sudo is unavailable.
    - Host distribution detection: Parses `/etc/os-release` to identify host distro family
      (Arch, Debian, Fedora).
    - Hardware capacity deduction: Reads physical RAM (via `/proc/meminfo`) and CPU core
      count (`std::thread::available_parallelism`) to deduce safe container concurrency
      (~3 GB RAM and 2 cores per container, reserving 4 GB RAM and 1 core for the host,
      minimum floor of 1).
    - Concurrency validation and warning: Warns if requested concurrency exceeds hardware
      safety limits.
- `prereqs.rs`: System requirement checks and package manager orchestrator:
    - Validates `systemd-nspawn`, `machinectl`, and `mkosi`.
    - Distro-aware package manager detection (`debootstrap` for Ubuntu, `dnf5` for Fedora,
      `pacman` for Arch).
    - Automatically installs missing host packages when passwordless sudo is available, or
      prompts interactively in the terminal.
- `machine.rs`: Machine lifecycle state machine (`NotFound`, `Stopped`, `Running`):
    - Unified container auto-boot (`systemd-nspawn -b`) with `--resolv-conf=copy-uplink`.
    - Wait-for-boot polling with timeout.
    - Process execution via `nsenter` (dropping privileges to user `tester` UID/GID 1000,
      running inside `--workdir`).
    - Interactive shell with internal PTY wrapper (`/usr/bin/script -q /dev/null -c "/bin/bash --login"`)
      preventing terminal hangs and `.profile` tty errors.
    - Multi-stage shutdown escalation (`poweroff` -> `terminate` -> `kill` -> `pkill -9` -> scope reset).
    - Stale resource cleanup and mount safety: inspects `/proc/mounts`, unmounts lingering
      mounts, clears `/run/systemd/nspawn/unix-export/`, and resets failed systemd scopes
      before any directory deletion.
- `zygote.rs`: Golden snapshot management:
    - Stateless execution: Dispatches to `systemd-nspawn -b -x` with `--resolv-conf=copy-uplink`.
    - Stateful reset: Fast Copy-on-Write restoration via `cp -a --reflink=auto`
      (BTRFS/XFS, `<1s`) or `rsync -aAX --delete` (ext4 fallback, 10-30s) after verifying
      zero active mounts.
- `image_builder.rs`: Invokes `mkosi` with profile arguments using embedded or local configs.

### 3. CLI & Command Hierarchy

**Type-Safe Command Parser in `build-infra/src/spawny/cli/`**:

```text
spawny (cargo-spawny)
├── setup / build         [--distro <d|all>] [--force] [--config-dir <path>]
├── teardown              [--distro <d|all>] (stops machines and purges disk images)
├── status / list         (displays table of machine states, IPs, zygote health)
│
├── stateless             (100% clean-room, auto-boots with -b -x, zero leftover state)
│   ├── install           (--script <path|url> | --cargo <crate> [--bin <name>])
│   │                     [--distro <d|all>] [-j|--max-parallel <N>] [--bind <h:c>]
│   │                     [--bind-ro <h:c>] [--env <K=V>] [-w|--workdir <dir>]
│   ├── run               "<command>"
│   │                     [--distro <d|all>] [-j|--max-parallel <N>] [--bind <h:c>]
│   │                     [--bind-ro <h:c>] [--env <K=V>] [-w|--workdir <dir>]
│   └── script            <test_script_path>
│                         [--distro <d|all>] [-j|--max-parallel <N>] [--bind <h:c>]
│                         [--bind-ro <h:c>] [--env <K=V>] [-w|--workdir <dir>]
│
└── stateful              (100% persistent sandbox, auto-boots if stopped, manual reset)
    ├── list / status     (alias for top-level status / list)
    ├── exec              <distro> "<command>" [--bind <h:c>] [--bind-ro <h:c>]
    │                     [--env <K=V>] [--user <user>] [-w|--workdir <dir>]
    ├── shell             [<distro>] (interactive TTY shell; choose() if omitted)
    ├── reset             [<distro|all>] (mount-safe reset back to golden zygote)
    ├── clean             [<distro|all>] (crash recovery: force-kill, unmount, scope reset)
    ├── start             <distro> (manually boots container daemon)
    └── stop              <distro> (stops container daemon)
```

**Global Options**:

- `--max-parallel <N>` / `-j <N>`: Maximum concurrent container instances. Dynamically
  deduced from CPU cores and available RAM if omitted.
- `--bind <host[:container]>` / `--bind-ro <host[:container]>`: Bind mounts for local
  workspaces and test assets.
- `--env <KEY=VAL>` / `-e <KEY=VAL>`: Injects environment variables into containers.
- `--user <USER>`: Specifies execution user (defaults to `tester`, UID 1000).
- `-w <PATH>` / `--workdir <PATH>`: Working directory inside container (defaults to
  `/home/tester`).
- `--non-interactive` (alias `--ci`): Disables interactive menus and spinners, emitting
  plain sequential logs.
- Cargo Plugin Convention: Automatically detects and strips injected `args[1]` when
  invoked as `cargo spawny`.

### 4. Interactive TUI Integration

**Rich Terminal UX in `build-infra/src/spawny/tui/` Powered by `r3bl_tui`**:

- **Interactivity Gate**: Queries `check_is_terminal_interactive()` from
  `r3bl_tui::core::term::term_api`:
    - If interactive: Renders live unified coordinator `r3bl_tui::Spinner` reflecting
      all concurrent distro tasks, and launches `r3bl_tui::choose()` menus when required
      arguments are omitted.
    - If non-interactive (CI, pipes, scripts): Disables spinners, prints sequential
      timestamped log lines, and fails fast if sudo or required arguments are missing.
- **Unified Multi-Distro Spinner**: A single coordinator `Spinner` tracking status across
  all active containers (e.g., `[Ubuntu: running, Fedora: completed, Arch: running]`),
  preventing raw mode collisions from multiple concurrent spinners.
- **Interactive Selection**: Contextual `r3bl_tui::choose()` menus for selecting target
  distros or actions.
- **Formatted Status Tables**: Renders styled machine status, IP addresses, and zygote
  health tables. Captured logs from failed runs are formatted clearly below the summary
  table.

### 5. Spawny Binary Entry Point

**Binary Entry Point in `build-infra/src/bin/spawny.rs`**:

- Target Gating: Entire `spawny` module and binary gated with
  `#[cfg(target_os = "linux")]`. On non-Linux platforms, provides a clean stub that
  explains `spawny` is Linux-only, allowing `cargo check --target x86_64-pc-windows-msvc`
  and macOS checks in `./check.fish --full` to pass cleanly.
- Integrates with `r3bl-build-infra` package suite
  (`cargo install --path build-infra --force`).
- Note: Standalone self-upgrade (`--upgrade`) will be wired across all binaries as part of
  `task/binaries-self-upgrade-support.md`.

---

## Implementation Plan

#### Phase 1: mkosi Configuration, Module Wiring & Prereq Checks

**Declarative image definitions, incremental compilation setup, and environment validation**:

- [ ] Add `include_dir` dependency to `build-infra/Cargo.toml`.
- [ ] Export `spawny` module in `build-infra/src/lib.rs` gated by `#[cfg(target_os = "linux")]`
      with submodules declared so `./check.fish --check` compiles and checks every phase.
- [ ] Create `build-infra/mkosi/mkosi.conf` with shared base configuration.
- [ ] Create `build-infra/mkosi/mkosi.profiles/ubuntu/mkosi.conf` for Ubuntu 24.04 LTS.
- [ ] Create `build-infra/mkosi/mkosi.profiles/fedora/mkosi.conf` for Fedora 41.
- [ ] Create `build-infra/mkosi/mkosi.profiles/arch/mkosi.conf` for Arch Linux rolling.
- [ ] Create `build-infra/mkosi/mkosi.postinst.chroot` for hermetic setup:
      - Creates user `tester` explicitly with home directory (`useradd -m -s /usr/bin/fish -u 1000 -U tester`).
      - Configures passwordless sudo in `/etc/sudoers.d/tester`.
      - Installs Rust toolchain via `rustup` for user `tester` (ensuring Rust 1.85+ / edition 2024).
      - Configures `/etc/environment` to ensure `/home/tester/.cargo/bin` is in `$PATH`.
- [ ] Implement `build-infra/src/spawny/nspawn/system_info.rs`:
      - Sudo pre-validation: checks `sudo -v` (interactive) or `sudo -n true` (non-interactive);
        fails fast with diagnostic error if non-interactive and passwordless sudo is unavailable.
      - Host distribution detection via `/etc/os-release`.
      - Hardware capacity deduction (RAM from `/proc/meminfo`, cores from `available_parallelism`).
- [ ] Implement `build-infra/src/spawny/prereqs.rs` to validate `systemd-nspawn`,
      `machinectl`, `mkosi`, disk space (>= 15 GB free), and terminal interactivity.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/Cargo.toml`
    - [ ] `build-infra/src/lib.rs`
    - [ ] `build-infra/mkosi/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/ubuntu/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/fedora/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/arch/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.postinst.chroot`
    - [ ] `build-infra/src/spawny/nspawn/system_info.rs`
    - [ ] `build-infra/src/spawny/prereqs.rs`

### Phase 2: Core nspawn, Zygote & System Info Engine

**Low-level machine, snapshot, mount safety, and hardware capacity primitives**:

- [ ] Implement `build-infra/src/spawny/nspawn/distro.rs`:
      - Defines supported distributions, image paths, and machine name mappings.
      - Generates unique machine instance IDs for stateless runs (`spawny-stateless-<distro>-<uuid>`).
- [ ] Implement `build-infra/src/spawny/nspawn/machine.rs`:
      - Container auto-boot (`systemd-nspawn -b`) with `--resolv-conf=copy-uplink`.
      - Wait-for-boot readiness polling.
      - Process execution via `nsenter` (as user `tester` UID 1000, working in `--workdir`).
      - Interactive shell with internal PTY wrapper (`/usr/bin/script -q /dev/null -c "/bin/bash --login"`).
      - Multi-stage shutdown escalation (`poweroff` -> `terminate` -> `kill` -> `pkill -9` -> scope reset).
      - Mount safety: inspects `/proc/mounts`, cleanly unmounts any remaining child mounts,
        clears `/run/systemd/nspawn/unix-export/`, and resets failed scopes before deletion.
- [ ] Implement `build-infra/src/spawny/nspawn/zygote.rs` managing 2-tier storage:
      `systemd-nspawn -b -x` for stateless runs, and `cp -a --reflink=auto` / `rsync -aAX --delete`
      for stateful resets after mount safety check.
- [ ] Implement `build-infra/src/spawny/nspawn/image_builder.rs`:
      - Unpacks embedded `mkosi` configs to `/var/lib/spawny/mkosi/` if running outside repo root.
      - Invokes `mkosi` with profile arguments.
- [ ] Add unit tests for distro mapping, hardware capacity calculations, and state transitions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/nspawn/distro.rs`
    - [ ] `build-infra/src/spawny/nspawn/machine.rs`
    - [ ] `build-infra/src/spawny/nspawn/zygote.rs`
    - [ ] `build-infra/src/spawny/nspawn/image_builder.rs`

### Phase 3: CLI Parser & TUI Layer

**Command-line configuration and interactive TUI components**:

- [ ] Implement `build-infra/src/spawny/cli/` using `clap` derive:
      - `stateless` and `stateful` subcommands, plus top-level `status` / `list`.
      - Flags: `--max-parallel` / `-j`, `--bind`, `--bind-ro`, `--env`, `--user`,
        `-w` / `--workdir` (defaults to `/home/tester`), and `--non-interactive` / `--ci`.
      - `--bin <name>` optional flag for `install --cargo` with auto-detection fallback.
- [ ] Implement Cargo plugin convention argument stripping for `cargo spawny`.
- [ ] Implement `build-infra/src/spawny/tui/choose_menu.rs` wrapping contextual `r3bl_tui::choose()`.
- [ ] Implement `build-infra/src/spawny/tui/status_table.rs` formatting machine and zygote state tables.
- [ ] Implement `build-infra/src/spawny/tui/progress.rs` rendering live parallel multi-distro
      progress via a single unified coordinator `r3bl_tui::Spinner` when interactive, or
      sequential logs when non-interactive.
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
      - `install --script`: Auto-boots ephemeral container with unique machine ID, executes
        installer via `nsenter`, validates binary, cleanly powers off, discards changes.
      - `install --cargo`: Auto-boots ephemeral container, executes `cargo install`, validates
        `<bin> --version` / `--help`, powers off.
      - `run "<command>"`: Executes commands across distros in parallel auto-booted ephemeral
        containers with unified coordinator spinner, captures output, powers off.
      - `script <path>`: Mounts script and parent directories, runs tests, powers off, captures report.
- [ ] Implement `build-infra/src/spawny/runner/stateful_runner.rs`:
      - `exec <distro> "<command>"`: Auto-boots container if stopped, runs command via `nsenter`,
        keeps container running with persistent changes.
      - `shell [<distro>]`: Auto-boots container if stopped, opens interactive TTY shell with PTY.
      - `reset [<distro|all>]`: Mount-safe reset reverting container back to golden zygote.
      - `clean [<distro|all>]`: Crash recovery force-killing containers, unmounting lingering mounts,
        and resetting systemd scopes.
- [ ] Implement signal handling (`tokio::signal::ctrl_c`) to ensure child containers and mounts
      are cleaned up on abort without disrupting interactive shell sessions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/runner/stateless_runner.rs`
    - [ ] `build-infra/src/spawny/runner/stateful_runner.rs`
    - [ ] `build-infra/src/spawny/runner/mod.rs`

### Phase 5: Binary Integration & Workspace Gating

**Binary entry point, packaging, and documentation**:

- [ ] Add `spawny` binary to `build-infra/Cargo.toml` (`[[bin]]`).
- [ ] Implement `build-infra/src/bin/spawny.rs` with `#[cfg(target_os = "linux")]` gating
      and non-Linux platform stubs.
- [ ] Install binary locally via `cargo install --path build-infra --force` and verify
      `spawny --help` and `cargo spawny --help`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs` documentation to include
      `spawny` features, command hierarchy, and usage alongside `cargo-rustdoc-fmt`.
- [ ] Update root workspace `README.md` to showcase `spawny` in the binary tool suite.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/Cargo.toml`
    - [ ] `build-infra/src/bin/spawny.rs`
    - [ ] `build-infra/README.md`
    - [ ] `README.md`

### Phase 6: Remove Legacy cmdr nspawn Scripts & Update run.fish

**Clean up obsolete workspace shell scripts**:

- [ ] Remove `cmdr/systemd-nspawn/` directory from the workspace.
- [ ] Remove `build-infra/reference/nspawn-scripts/` directory if no longer needed.
- [ ] Refactor `test-cmdr-install-on-all-linux-distros` in workspace root `run.fish` to
      invoke `spawny stateless install --cargo r3bl-cmdr` instead of legacy scripts.
- [ ] Update `build-infra/AGENTS.md` and documentation to reference `spawny`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/` (verify `systemd-nspawn/` removed)
    - [ ] `run.fish` (verify `test-cmdr-install-on-all-linux-distros` uses `spawny`)
    - [ ] `build-infra/AGENTS.md`

### Phase 7: External Test Suite Migration in ~/github/notes

**Migrate host test runner in `~/github/notes/files/scripts/tests/` to invoke spawny**:

- [ ] Refactor the host test runner in `/home/nazmul/github/notes/files/scripts/tests/run.fish`
      to invoke `spawny` CLI commands with required bind mounts (`/scripts`, `/synced-data`,
      `/host-home`) instead of sourcing `lib/nspawn.fish`.
- [ ] Verify that the in-container test scripts (`e2e/01-test-fresh-install-stateless.fish`
      and `e2e/02-test-fresh-install-stateful.fish`) execute cleanly inside containers
      managed by `spawny`.
- [ ] Retire obsolete fish helper scripts in `/home/nazmul/github/notes/files/scripts/tests/`
      (`setup.fish`, `teardown.fish`, `lib/nspawn.fish`).
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `/home/nazmul/github/notes/files/scripts/tests/` (verify host runner migrated to `spawny`)

### Phase 8: Verification & Testing

**Comprehensive validation of spawny across distros**:

- [ ] Run `spawny setup --distro all` and verify `mkosi` builds Arch, Ubuntu, and Fedora
      images with pre-baked `rustup` toolchains.
- [ ] Run `spawny status` and verify all machines and zygotes are registered.
- [ ] Run `spawny stateful exec ubuntu "echo hello"` and verify auto-boot and state persistence.
- [ ] Run `spawny stateful reset ubuntu` and verify mount safety check and instant restore.
- [ ] Run `spawny stateful clean ubuntu` and verify crash recovery.
- [ ] Run `spawny stateless run --distro all "uname -a"` and verify parallel execution
      with unique machine IDs and unified coordinator spinner.
- [ ] Verify DNS resolution (`--resolv-conf=copy-uplink`) inside containers by running `curl`
      and `cargo --version`.
- [ ] Run `./check.fish --check`, `--build`, `--clippy`, `--test` across the workspace.
- [ ] Run `./check.fish --full` to verify Windows cross-compilation with platform gating.
- [ ] Run migrated test suite in `/home/nazmul/github/notes/files/scripts/tests/` powered by `spawny`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/build-infra-spawny.md`

---

## Verification Matrix

### Distro Coverage Matrix

| Distribution   | Version / Release | Backend Tool            | Default User | Package Manager | Zygote Path                       |
| :------------- | :---------------- | :---------------------- | :----------- | :-------------- | :-------------------------------- |
| **Ubuntu**     | 24.04 LTS (Noble) | `mkosi` + `debootstrap` | `tester`     | `apt`           | `/var/lib/spawny/zygotes/ubuntu/` |
| **Fedora**     | 41                | `mkosi` + `dnf5`        | `tester`     | `dnf`           | `/var/lib/spawny/zygotes/fedora/` |
| **Arch Linux** | Rolling           | `mkosi` + `pacman`      | `tester`     | `pacman`        | `/var/lib/spawny/zygotes/arch/`   |

### Command Verification Checklist

- [ ] `spawny setup`: Builds all 3 distros using `mkosi` and creates golden snapshots.
- [ ] `spawny teardown`: Stops running containers and removes images.
- [ ] `spawny stateless install --script <path>`: Clean-room installer verification with
      `--ephemeral`.
- [ ] `spawny stateless install --cargo <crate>`: Clean-room crates.io build verification
      with pre-baked toolchain.
- [ ] `spawny stateless run "<command>"`: Clean-room command runner across all distros
      with `--max-parallel`.
- [ ] `spawny stateful exec <distro> "<command>"`: Persistent command execution in living
      sandbox.
- [ ] `spawny stateful shell <distro>`: Interactive TTY container shell.
- [ ] `spawny stateful reset <distro>`: Instant reset to golden snapshot (`reflink` or
      `rsync`).
- [ ] `spawny stateful clean <distro>`: Crash recovery clearing stuck processes, scopes,
      and mounts.
- [ ] `spawny stateful list`: Status table rendering via `r3bl_tui`.
- [ ] `cargo spawny`: Verifies Cargo plugin argument invocation convention.
- [ ] Cross-platform verification: `./check.fish --full` verifies Windows
      cross-compilation passes via `#[cfg(target_os = "linux")]` gating.
- [ ] Migrate `~/github/notes/files/scripts/tests/` to use `spawny` and retire legacy fish
      test scripts.

<!-- cspell:words postinst Rootfs machinectl nsenter debootstrap userland sysusers -->
<!-- cspell:words reflink Reflink procs overlayfs Passwordless poweroff passwordless chrooted -->
