# Task: Build-Infra Spawny Systemd-nspawn Machine Manager

<!-- prettier-ignore-start -->
<!-- BEGIN mktoc -->

- [Task: Build-Infra Spawny Systemd-nspawn Machine Manager](#task-build-infra-spawny-systemd-nspawn-machine-manager)
- [Overview](#overview)
    - [Systemd-nspawn Clean-Room Testing](#systemd-nspawn-clean-room-testing)
  - [Standardized mkosi Image Pipeline](#standardized-mkosi-image-pipeline)
  - [Privilege Model & Execution Safety](#privilege-model--execution-safety)
  - [Unified CLI Architecture & Command Overview](#unified-cli-architecture--command-overview)
    - [Command Overview](#command-overview)
      - [1. Lifecycle & Image Management](#1-lifecycle--image-management)
      - [2. Command & Script Execution](#2-command--script-execution)
      - [3. Interactive Shell & Inspection](#3-interactive-shell--inspection)
      - [4. Sandbox Maintenance & Recovery](#4-sandbox-maintenance--recovery)
- [Lifecycle Flowcharts & Mental Model](#lifecycle-flowcharts--mental-model)
  - [1. Storage & Zygote Mental Model](#1-storage--zygote-mental-model)
  - [2. Ephemeral Execution Flow (Clean-Room)](#2-ephemeral-execution-flow-clean-room)
  - [3. Persistent Execution Flow (Living Sandbox)](#3-persistent-execution-flow-living-sandbox)
    - [4. Interactive TUI Launcher Flow](#4-interactive-tui-launcher-flow)
- [Architecture](#architecture)
  - [1. mkosi Image Builder](#1-mkosi-image-builder)
  - [2. Machine, Zygote & System Info Engine](#2-machine-zygote--system-info-engine)
  - [3. CLI & Command Hierarchy](#3-cli--command-hierarchy)
  - [4. Container & Lifecycle Runners](#4-container--lifecycle-runners)
  - [5. Interactive TUI Integration](#5-interactive-tui-integration)
  - [6. Spawny Binary Entry Point](#6-spawny-binary-entry-point)
- [Implementation Plan](#implementation-plan)
    - [Phase 1: mkosi Configuration, Module Wiring & Prereq Checks](#phase-1-mkosi-configuration-module-wiring--prereq-checks)
  - [Phase 2: Core nspawn, Zygote & System Info Engine](#phase-2-core-nspawn-zygote--system-info-engine)
  - [Phase 3: CLI Parser & TUI Layer](#phase-3-cli-parser--tui-layer)
  - [Phase 4: Container & Command Runners](#phase-4-container--command-runners)
  - [Phase 5: Binary Integration & Workspace Gating](#phase-5-binary-integration--workspace-gating)
  - [Phase 6: Comprehensive Verification & Testing](#phase-6-comprehensive-verification--testing)
  - [Phase 7: Remove Legacy cmdr nspawn Scripts & Update run.fish](#phase-7-remove-legacy-cmdr-nspawn-scripts--update-runfish)
  - [Phase 8: External Test Suite Migration in ~/github/notes](#phase-8-external-test-suite-migration-in-~githubnotes)
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

#### Systemd-nspawn Clean-Room Testing

**Native Linux Isolation with Instant Restores**: `spawny` leverages `systemd-nspawn` and
the Zygote pattern to deliver fast, daemon-less container testing:

- **Daemonless and Native**: Direct kernel namespaces and cgroups without Docker daemon
  overhead.
- **Exclusively Raw GPT Disk Images with CoW Reflinks**:
    - Container templates are exclusively built as single-file raw GPT disk images
      (`Format=disk`), booted via `systemd-nspawn -i /path/to/image.raw`. A single `.raw`
      file completely eliminates all BTRFS nested subvolume bugs (where `rm -rf` fails
      with `EPERM` on nested subvolume roots created by systemd or package managers) and
      eliminates directory traversal overhead.
- **Two-Tier Instant Restores**:
    - **Ephemeral (Clean-Room)**: Uses `systemd-nspawn -i <image.raw> --ephemeral` (`-x`)
      with an RAII supervisor guard. Creates an instant kernel CoW overlay layer,
      discarding all changes on container exit.
    - **Persistent (Sandbox Reset)**: Fast single-file Copy-on-Write via
      `cp --reflink=auto` (BTRFS/XFS, `<0.01s`) or sparse copying (ext4, ~1-2s), reverting
      working machine images back to golden zygote state.
- **Multi-Distro Validation**: Runs tests simultaneously across Ubuntu 24.04, Fedora 41,
  and Arch Linux.
- **General-Purpose Design**: Built to serve R3BL workspace testing first, then published
  as a reusable tool for any Rust or Linux project.

### Standardized mkosi Image Pipeline

**Declarative OS Image Generation with Embedded Configs**: Replaces legacy, ad-hoc image
download scripts with `mkosi` (official systemd project tool):

- **Declarative Distro Configs**: Standardized configuration directories
  (`build-infra/mkosi/mkosi.conf` and `mkosi.profiles/<distro>/mkosi.conf`). `mkosi.conf`
  specifies `Format=disk` and `WithNetwork=yes`.
- **Binary Embedding with Ephemeral Extraction**: Config files and scripts are embedded
  directly into the `spawny` binary via `include_dir!`. When `spawny setup` (or `build`)
  runs, it extracts the embedded configuration directory into an ephemeral temporary
  directory (`tempfile::tempdir()`), ensuring executable scripts receive `0o755`
  permissions. `mkosi` is executed pointing to this temporary directory, and the directory
  is automatically cleaned up on completion, avoiding stale cache issues and host
  filesystem pollution (overridable for local iteration via `--config-dir`).
- **Reliable User Provisioning**: User `tester` (UID 1000, shell `/usr/bin/fish`) is
  created explicitly with home directory `/home/tester` via `useradd -m` in
  `mkosi.postinst.chroot`, with passwordless sudo (`/etc/sudoers.d/tester`) and
  `/home/tester/.cargo/bin` pre-configured in `/etc/environment` and `$PATH`.
- **Hermetic Post-Installation**: Distro-specific runtime customization strictly isolated
  inside `mkosi.postinst.chroot` (guaranteed by `mkosi` to run inside container chroot
  namespaces, never touching host accounts).
- **Modern Rust via Rustup with Network Access**: With `WithNetwork=yes` configured in
  `mkosi.conf`, the Rust toolchain is installed via `rustup` inside
  `mkosi.postinst.chroot` on all distributions, guaranteeing Rust 1.85+ (Rust 2024 edition
  compatible) across Ubuntu, Fedora, and Arch. Base tools (`fish`, `curl`, `git`, `sudo`,
  `build-essential` / `base-devel`) are pre-installed in golden images.

### Privilege Model & Execution Safety

- **Unprivileged CLI Execution**: `spawny` is executed as a regular user from any
  terminal. Internal privileged operations (`systemd-nspawn`, `machinectl`, `nsenter`,
  file operations in `/var/lib/`) are elevated via `sudo`.
- **Unconditional Passwordless Sudo Requirement**: Passwordless sudo (`sudo -n true`) is
  an unconditional requirement for running `spawny` across both interactive and
  non-interactive modes. At the very start of execution, `spawny` runs `sudo -n true`. If
  passwordless sudo is not configured on the host, `spawny` fails fast immediately with a
  clear diagnostic error instructing the user how to configure `NOPASSWD: ALL` in
  `/etc/sudoers.d/spawny`. The user is never prompted for a password during execution,
  eliminating sudo credential timeouts and terminal spinner corruption.
- **Defensive Path Validation**: Before executing any privileged deletion (`rm -rf`),
  `spawny` validates that the canonical target path strictly matches
  `/var/lib/machines/spawny-*`, `/var/lib/spawny/zygotes/*`, or
  `/run/systemd/nspawn/unix-export/spawny-*`. Deletion is strictly prohibited if the path
  is empty or resolves to parent directories such as system root (`/`), `/var`,
  `/var/lib`, `/var/lib/machines`, `/var/lib/spawny`, `/run`, or
  `/run/systemd/nspawn/unix-export`.
- **Mount Safety Before Deletion**: Before deleting any machine image or directory during
  reset, clean, or teardown, `spawny` inspects `/proc/mounts`, cleanly unmounts any
  remaining child mounts, cleans up stale container sockets in
  `/run/systemd/nspawn/unix-export/spawny-*`, and resets failed systemd scopes, ensuring
  `rm -rf` never traverses into host files.

### Unified CLI Architecture & Command Overview

**Flat, Intuitive Command Model**: `spawny` provides a single unified command interface
where clean-room execution versus persistent sandbox execution is controlled via the
standard `--ephemeral` (`-x`) flag. Both modes feature **unified auto-boot**
(`systemd-nspawn -b -i <image.raw>`) with `--capability=CAP_NET_ADMIN`, ensuring `systemd`
is PID 1, system services and daemons operate consistently, and commands execute via
`nsenter`:

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│                                   SPAWNY                                    │
╰─────────────────────────────────────────────────────────────────────────────╯
        │                                                     │
        ▼ (Execution & Testing)                               ▼ (Sandbox & Lifecycle)
┌───────────────────────────────────────┐   ┌──────────────────────────────────┐
│        `spawny run / script`          │   │  `spawny shell / reset / clean`  │
├───────────────────────────────────────┤   ├──────────────────────────────────┤
│ • run [--ephemeral|-x] "<command>"    │   │ • shell [<distro>] (login PTY)   │
│ • script [--ephemeral|-x] <path>      │   │ • ps [<distro>] (process tree)   │
│ • status / list [-v] (ps trees)       │   │ • reset [<distro|all>] (reflink) │
│                                       │   │ • clean [<distro|all>] (crash)   │
│                                       │   │ • setup / teardown               │
│                                       │   │ • start / stop <distro>          │
├───────────────────────────────────────┤   ├──────────────────────────────────┤
│ • With -x: Ephemeral clean-room run   │   │ • Auto-boots if stopped          │
│   (boots zygote, discards on exit)    │   │ • State persists across runs     │
│ • Without -x: Persistent sandbox run  │   │ • Mount-safe CoW instant reset   │
│ • Parallel execution across distros   │   │ • Interactive shell with banner  │
└───────────────────────────────────────┘   └──────────────────────────────────┘
```

#### Command Overview

##### 1. Lifecycle & Image Management

- `spawny setup [--distro <ubuntu|fedora|arch|all>] [--force] [--config-dir <path>]`:
  Checks/installs host dependencies, extracts embedded `mkosi` configs into an ephemeral
  temporary directory (with `0o755` permissions), builds raw GPT disk images (`.raw`), and
  creates golden zygotes in `/var/lib/spawny/zygotes/`.
- `spawny teardown [--distro <ubuntu|fedora|arch|all>]`: Stops containers, unmounts active
  mounts, unregisters machines, and removes disk images with defensive path checks.
- `spawny status` / `spawny list [-v|--verbose]`: Displays formatted `r3bl_tui` table of
  all machines, runtime states (Running/Stopped), IPs, and golden zygote status (`-v`
  displays container process trees via `nsenter`).

##### 2. Command & Script Execution

- `spawny run [--ephemeral|-x] [--distro <ubuntu|fedora|arch|all>] "<command>" [--bind <h:c>] [--bind-ro <h:c>] [--env <K=V>] [--user <user>] [-w <dir>] [--timeout <sec>] [--cap <cap>]`:
  Executes an arbitrary shell command inside container(s).
    - With `--ephemeral` (`-x`): Auto-boots an ephemeral container with a unique machine
      ID and RAII supervisor guard, executes command via `nsenter` (UID 1000 `tester` by
      default), powers off on completion, and discards all changes. Supports parallel
      execution across distros when `--distro all` is specified.
    - Without `--ephemeral`: Auto-boots or attaches to the persistent working container
      (`spawny-<distro>.raw`); all state changes persist on disk across commands.
- `spawny script [--ephemeral|-x] [--distro <ubuntu|fedora|arch|all>] <test_script_path> [--bind <h:c>] [--bind-ro <h:c>] [--env <K=V>] [--user <user>] [-w <dir>] [--timeout <sec>] [--cap <cap>]`:
  Mounts script and required parent directories into container(s), runs test suite, and
  captures report (with `--ephemeral` for clean-room testing or persistent for stateful
  debugging).

##### 3. Interactive Shell & Inspection

- `spawny shell [<distro>]`: Opens an interactive TTY login shell with PTY allocation
  (auto-booting persistent container if stopped; prompts via `r3bl_tui::choose()` if
  distro omitted). Displays banner with exit instructions (`exit` / `Ctrl+D`) and command
  cancel (`Ctrl+C`).
- `spawny ps [<distro>]`: Inspects running container(s) and displays live process tree
  (`ps f --forest` via `nsenter`), matching `run.fish ps` and `status.fish -v`.

##### 4. Sandbox Maintenance & Recovery

- `spawny reset [<distro|all>]`: Stops machine, unmounts dangling mounts, and reverts
  active machine(s) back to pristine golden zygote (`<0.01s` via single-file
  `cp --reflink=auto` on BTRFS/XFS, ~1-2s sparse copy on ext4).
- `spawny clean [<distro|all>]`: Recovers from unrecoverable crashes: force-kills stuck
  processes, unmounts lingering mounts, resets systemd scopes, and cleans orphaned
  ephemeral snapshots.
- `spawny start <distro>`: Manually boots container daemon in the background without
  attaching a shell.
- `spawny stop <distro>`: Gracefully halts running container.

---

## Lifecycle Flowcharts & Mental Model

### 1. Storage & Zygote Mental Model

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                         SPAWNY STORAGE ARCHITECTURE                         │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌────────────────────────┐                                                 │
│  │ mkosi Build Pipeline   │ (Declarative mkosi.conf + mkosi.extra/ +        │
│  │ (With Rust Toolchains) │  WithNetwork=yes + mkosi.postinst.chroot)       │
│  └───────────┬────────────┘                                                 │
│              │                                                              │
│              ▼                                                              │
│  ┌────────────────────────────────────────┐                                 │
│  │ Golden Zygote Disk Image (.raw)        │ (Single Read-Only File on Disk) │
│  │ /var/lib/spawny/zygotes/<distro>.raw   │ No nested subvolumes; instant   │
│  └───────────┬────────────────────────────┘                                 │
│              │                                                              │
│      ┌───────┴─────────────────────────────────────────────┐                │
│      │                                                     │                │
│      ▼ (Ephemeral Clean-Room Execution)                    ▼ (Persistent)   │
│  ┌────────────────────────────────────────┐   ┌────────────────────────┐    │
│  │ Ephemeral Container Mount              │   │ Working Machine Image  │    │
│  │ (systemd-nspawn -i <distro>.raw -x)    │   │ /var/lib/machines/     │    │
│  │ - Kernel CoW loopback device           │   │   spawny-<distro>.raw  │    │
│  │ - RAII supervisor reaps on drop        │   │ - Single-file reflink  │    │
│  │ - 0 RAM consumed by zygote; discarded  │   │   (<0.01s CoW pointer) │    │
│  │   100% on container exit               │   │ - Mount-safe reset     │    │
│  └────────────────────────────────────────┘   └────────────────────────┘    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

### 2. Ephemeral Execution Flow (Clean-Room)

```text
╭─────────────────────────────────────────────────────────────────────────────────╮
│ User Invokes: `spawny run --ephemeral "<command>"` OR `spawny script -x <path>` │
╰─────────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 1. Upfront Passwordless Sudo Validation & Capacity Deduction                │
│    - Verifies passwordless sudo (sudo -n true); fails fast if not configured│
│    - Deduces --max-parallel (CPU cores & RAM available; avoids OOM)         │
│    - Interactivity check (check_is_terminal_interactive())                  │
│    - Prepares unified coordinator Spinner (interactive) or logs (CI)        │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 2. Ephemeral Container Auto-Boot & RAII Supervisor Attachment               │
│    - Generates unique machine ID (spawny-ephemeral-<distro>-<uuid>)         │
│    - Spawns `systemd-nspawn -b -x -i <image.raw>`                           │
│      with --resolv-conf=copy-uplink and --capability=CAP_NET_ADMIN          │
│    - Attaches RAII process supervisor guard to guarantee termination        │
│    - Mounts host paths via --bind / --bind-ro (ArgAction::Append); sets -w  │
│    - Waits for systemd boot readiness                                       │
│    - Executes via nsenter (as user tester UID 1000): command or test script │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 3. Automated Validation & Smoke Tests                                       │
│    - Verifies command exit status code == 0                                 │
│    - Captures stdout/stderr streams with timestamps                         │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ 4. Clean-Room Teardown & Reporting                                          │
│    - RAII supervisor cleanly powers off container; ephemeral layer dropped  │
│    - Renders unified summary table (Pass / Fail / Timings / Captured logs)  │
│    - Leaves ZERO persistent disk pollution or residual states               │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

### 3. Persistent Execution Flow (Living Sandbox)

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│ Cumulative Step-by-Step Developer Workflow                                  │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 1: Install Package/Script (Auto-Boots if Stopped, State Persists)      │
│ `spawny run ubuntu "./install.sh"`                                          │
│ ├─► Detects if spawny-ubuntu is running; auto-boots if stopped (-b -i)      │
│ ├─► Executes installer via nsenter (as tester, working in --workdir)        │
│ └─► Binaries placed in `/home/tester/.cargo/bin/` REMAIN on disk            │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 2: Test Subsequent Actions on Accumulated State                        │
│ `spawny run ubuntu "giti status"`                                           │
│ ├─► Container is already booted and running from Step 1                     │
│ ├─► `giti` is already installed and in PATH                                 │
│ └─► Modifies state in-place; changes persist on disk                        │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 3: Interactive Container Shell                                         │
│ `spawny shell ubuntu`                                                       │
│ ├─► Auto-boots container if stopped; allocates PTY via script wrapper       │
│ ├─► Displays banner: exit ('exit' / Ctrl+D), command cancel (Ctrl+C)        │
│ ├─► Standard PTY pass-through: Ctrl+C cancels running in-container commands │
│ └─► Developer manually inspects logs, files, and environment                │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭─────────────────────────────────────────────────────────────────────────────╮
│ Step 4: Mount-Safe Reset OR Crash Recovery                                  │
│ `spawny reset ubuntu` OR `spawny clean ubuntu`                              │
│ ├─► Checks /proc/mounts, unmounts lingering mounts, resets failed scopes    │
│ ├─► reset: Reverts `spawny-ubuntu.raw` back to zygote (<0.01s reflink)      │
│ └─► clean: Force-kills stuck nspawn procs, clears scopes/stale mounts       │
╰─────────────────────────────────────────────────────────────────────────────╯
```

---

#### 4. Interactive TUI Launcher Flow

```text
╭─────────────────────────────────────────────────────────────────────────────╮
│ User runs `spawny` with no arguments OR omits target `--distro`             │
╰─────────────────────────────────────────────────────────────────────────────╯
                                      │
                                      ▼
╭──────────────────────────────────────────────────────────────────────────────╮
│ `check_is_terminal_interactive()` Gate                                       │
│ ├─► If Not Interactive (CI / Script):                                        │
│ │   Fails fast with actionable error requiring `--distro <d|all>`            │
│ └─► If Interactive: Launches contextual `r3bl_tui::choose()`:                │
│                                                                              │
│   • Batch Commands (`run`, `script`, `setup`, `teardown`, `reset`, `clean`): │
│     ┌ Select Target Distribution(s) (Space: toggle, Enter: confirm) ──┐      │
│     │ [x] Ubuntu 24.04 LTS (Noble)                                    │      │
│     │ [x] Fedora 41                                                   │      │
│     │ [ ] Arch Linux (Rolling)                                        │      │
│     └─────────────────────────────────────────────────────────────────┘      │
│                                                                              │
│   • Single-Machine (`shell`):                                                │
│     ┌ Select Target Container for Interactive Shell ──────────────────┐      │
│     │ > 1. Ubuntu 24.04 LTS (Noble)                                   │      │
│     │   2. Fedora 41                                                  │      │
│     │   3. Arch Linux (Rolling)                                       │      │
│     └─────────────────────────────────────────────────────────────────┘      │
│                                                                              │
│   • Bare `spawny` (no command): Action Launcher Menu                         │
╰──────────────────────────────────────────────────────────────────────────────╯
```

---

## Architecture

### 1. mkosi Image Builder

**Declarative OS Image Definitions in `build-infra/mkosi/` (Embedded in Binary)**:

- `mkosi.conf`: Shared base configuration specifying `Format=disk` (raw GPT disk image),
  `WithNetwork=yes` (network access enabled for chroot toolchain installation), and
  virtual disk capacity `OutputSize=15G`. Annotated with an explanatory comment
  referencing the host runtime disk space check `MIN_HOST_FREE_DISK_BYTES` (20 GB) in
  `build-infra/src/spawny/constants.rs`.
- `mkosi.profiles/`:
    - `ubuntu/mkosi.conf`: Ubuntu 24.04 LTS (Noble) package list and mirror configuration.
    - `fedora/mkosi.conf`: Fedora 41 package list and repository configuration.
    - `arch/mkosi.conf`: Arch Linux rolling release package list and pacman keyring
      configuration.
- `mkosi.postinst.chroot`: Hermetic post-installation script executed inside container
  chroot namespaces:
    - Creates user `tester` (UID 1000, shell `/usr/bin/fish`) explicitly via `useradd -m`.
    - Configures passwordless sudo in `/etc/sudoers.d/tester`.
    - Installs modern Rust via `rustup` for user `tester` over the enabled network,
      ensuring Rust 1.85+ (Rust 2024 edition compatibility).
    - Sets up `/etc/environment` to include `/home/tester/.cargo/bin` and
      `/home/tester/.local/bin` in system and user `$PATH`.
- Pre-baked development tools in all images: Rust toolchain (via `rustup`), `fish`,
  `curl`, `git`, `sudo`, `build-essential` / `base-devel`.
- **Binary Embedding via `include_dir!` with Ephemeral Extraction**: Config files are
  embedded into `spawny`. On `spawny setup`, files are unpacked into an ephemeral
  temporary directory (`tempfile::tempdir()`), with executable permissions (`0o755`) set
  on scripts like `mkosi.postinst.chroot`. `mkosi` is invoked pointing to this temporary
  directory, which is automatically deleted after image generation.
- Output artifacts are saved as single raw GPT disk images:
  `/var/lib/spawny/zygotes/<distro>.raw`.

### 2. Machine, Zygote & System Info Engine

**Core Lifecycle Primitives in `build-infra/src/spawny/`**:

- `constants.rs`: Centralized constants and system defaults organized into namespaced
  modules:
    - `container_defaults`: Non-root test runner username (`DEFAULT_USER = "tester"`),
      numeric IDs (`DEFAULT_UID: u32 = 1000`, `DEFAULT_GID: u32 = 1000`), string
      representations (`DEFAULT_UID_STR: &str = "1000"`, `DEFAULT_GID_STR: &str = "1000"`)
      preventing repeated heap allocations in command loops, default home
      (`DEFAULT_HOME = "/home/tester"`), default working directory
      (`DEFAULT_WORKDIR = "/home/tester"`), and default interactive shell
      (`DEFAULT_SHELL = "/bin/bash"`).
    - `host_defaults`: Minimum required host physical free disk space
      (`MIN_HOST_FREE_DISK_BYTES: u64 = 20 * 1024 * 1024 * 1024`, 20 GB). Its rustdoc
      explicitly cross-references the container virtual capacity (`OutputSize=15G`)
      configured in `build-infra/mkosi/mkosi.conf`, explaining that while each container
      image has a 15 GB virtual ceiling, raw GPT images are sparse files (~2.5-3.0 GB
      initial physical footprint), and the host requires 20 GB of physical free space to
      safely build all three distro templates simultaneously without risking host disk
      exhaustion.
    - `host_paths`: Host filesystem locations (`ZYGOTES_DIR = "/var/lib/spawny/zygotes"`,
      `MACHINES_DIR = "/var/lib/machines"`, `EPHEMERAL_CONFIG_PREFIX = "spawny-mkosi-"`).
- `distro.rs`: Distro enumeration (`Ubuntu`, `Fedora`, `Arch`), paths
  (`/var/lib/spawny/zygotes/<distro>.raw` and `/var/lib/machines/spawny-<distro>.raw`),
  and container metadata. Generates unique machine IDs for ephemeral runs
  (`spawny-ephemeral-<distro>-<uuid>`) and persistent machine names (`spawny-<distro>`)
  for stateful runs.
- `system_info.rs`: Unified host system introspection and hardware capacity engine:
    - Passwordless sudo validation: Unconditionally validates `sudo -n true` at startup
      across both interactive and non-interactive modes. Fails fast immediately if
      passwordless sudo is not configured on the host, ensuring zero interactive password
      prompts, zero credential timeouts, and zero TUI spinner corruption.
    - Host distribution detection: Detects host distribution via a type-safe
      `HostDistroFamily` enum (`Arch`, `Debian`, `Fedora`) providing `package_manager()`
      and `install_cmd()`. Resolves distribution via a 3-tier pipeline:
        1. Tier 1 (High Priority: `ID_LIKE`): Tokenizes `ID_LIKE` to identify derivative
           bases (`arch` maps to `Arch`; `debian` or `ubuntu` maps to `Debian`; `fedora`,
           `rhel`, or `centos` maps to `Fedora`). Ensures Arch derivatives like CachyOS
           (`ID=cachyos`, `ID_LIKE=arch`) map cleanly to `Arch`.
        2. Tier 2 (Fallback: `ID`): Fallback for upstream hosts lacking `ID_LIKE` (`arch`
           maps to `Arch`, `debian` or `ubuntu` maps to `Debian`, `fedora` maps to
           `Fedora`).
        3. Tier 3 (Defense-in-depth: Host `$PATH` Probe): Probes `pacman`,
           `apt-get`/`apt`, or `dnf` on the host `$PATH` if `/etc/os-release` is missing
           or unrecognized. Captures `PRETTY_NAME` (e.g. "CachyOS Linux") for informative
           logs and TUI banners.
    - Hardware capacity deduction: Reads physical RAM (via `/proc/meminfo`) and CPU core
      count (`std::thread::available_parallelism`) to deduce safe container concurrency
      (~3 GB RAM and 2 cores per container, reserving 4 GB RAM and 1 core for the host,
      minimum floor of 1).
    - Host free disk space validation: Queries physical filesystem free space via
      `statvfs` on `/var/lib` and verifies it meets `MIN_HOST_FREE_DISK_BYTES` (20 GB).
    - Concurrency validation and warning: Warns if requested concurrency exceeds hardware
      safety limits.
- `prereqs.rs`: System requirement checks and package manager orchestrator:
    - Validates core host utilities: `systemd-nspawn`, `machinectl`, and `mkosi`.
    - Uses `HostDistroFamily` to distinguish **host package managers** (`pacman`, `apt`,
      `dnf`) and generate install commands.
    - Distinguishes host package managers from **guest bootstrapping tools** (`apt` and
      `ubuntu-keyring` for Ubuntu, `dnf5` for Fedora, `pacman` for Arch).
    - Includes comprehensive rustdoc explaining why `apt` and `dnf5` are required on the
      host by `mkosi`: they act purely as cross-installation unpackers targeting the raw
      disk image (`-o RootDir=...`, `--installroot=...`), never touch host system
      packages, and are strictly build-time tools (only needed during `spawny setup`).
    - Reports missing host packages with clear install instructions for the detected
      distro family (or installs them non-interactively via passwordless sudo).
- `machine.rs`: Machine lifecycle state machine (`NotFound`, `Stopped`, `Running`):
    - Unified container auto-boot (`systemd-nspawn -b -i <path.raw>`) with
      `--capability=CAP_NET_ADMIN` and `--resolv-conf=copy-uplink`.
    - Wait-for-boot polling with timeout.
    - Process execution via `nsenter` (dropping privileges to user `tester` using
      `DEFAULT_UID_STR` and `DEFAULT_GID_STR`, running inside `--workdir`). Injects
      `HOME`, `USER`, and `PATH` via container `/usr/bin/env` to bypass host sudo
      `env_reset` and ensure `cargo` resolves. Documented with both function-level
      rustdocs explaining the pipeline and inline comments warning against host
      `Command::env()`.
    - Interactive shell with internal PTY wrapper
      (`/usr/bin/script -q /dev/null -c "/bin/bash --login"`) preventing terminal hangs
      and `.profile` tty errors. Includes detailed rustdoc documentation on `exec_shell()`
      explaining why `machinectl shell` hangs, why plain `nsenter` lacks a container PTY
      (causing `.profile` errors), and how `script` creates a container-local PTY pair.
      Displays introductory banner with exit instructions (`exit` / `Ctrl+D`) and command
      cancel (`Ctrl+C`). Standard PTY pass-through allows `Ctrl+C` to cancel running
      in-container foreground processes normally without terminating the interactive shell
      session.
    - Container process tree inspection: queries leader PID and executes `ps f --forest`
      via `nsenter` (powers `spawny ps` and `spawny status -v`).
    - Multi-stage shutdown escalation (`poweroff` -> `terminate` -> `kill` -> `pkill -9`
      -> scope reset).
    - Defensive path validation: verifies target paths strictly match
      `/var/lib/machines/spawny-*`, `/var/lib/spawny/zygotes/*`, or
      `/run/systemd/nspawn/unix-export/spawny-*` before executing privileged removals
      (`rm -rf`).
    - Stale resource cleanup and mount safety: inspects `/proc/mounts`, unmounts lingering
      mounts, clears container sockets in `/run/systemd/nspawn/unix-export/spawny-*`, and
      resets failed systemd scopes before any disk artifact deletion.
- `zygote.rs`: Golden snapshot management:
    - Ephemeral execution: Dispatches to `systemd-nspawn -b -x -i <image.raw>` with
      `--capability=CAP_NET_ADMIN` and `--resolv-conf=copy-uplink`, managed via an RAII
      supervisor guard.
    - Mount-safe reset: Fast single-file Copy-on-Write restoration via `cp --reflink=auto`
      (BTRFS/XFS, `<0.01s`) or sparse copy (ext4, ~1-2s) after verifying zero active
      mounts and defensive path safety.
- `image_builder.rs`: Extracts embedded configs to an ephemeral temporary directory
  (`tempfile::tempdir()`), sets `0o755` permissions on scripts, invokes `mkosi` with
  profile arguments, and saves output to `/var/lib/spawny/zygotes/<distro>.raw`
  (overridable via `--config-dir`).

### 3. CLI & Command Hierarchy

**Type-Safe Command Parser in `build-infra/src/spawny/cli/`**:

```text
spawny
├── setup / build         [--distro <d|all>] [--force] [--config-dir <path>]
├── teardown              [--distro <d|all>] (stops machines and purges disk images)
├── status / list         [-v|--verbose] (displays table of machine states, IPs, zygote health; -v shows ps trees)
│
├── run                   [--ephemeral|-x] [--distro <d|all>] "<command>"
│                         [--bind <h:c>] [--bind-ro <h:c>] [--env <K=V>]
│                         [--user <user>] [-w|--workdir <dir>] [--timeout <sec>]
│                         [--cap <cap>] [-j|--max-parallel <N>]
├── script                [--ephemeral|-x] [--distro <d|all>] <test_script_path>
│                         [--bind <h:c>] [--bind-ro <h:c>] [--env <K=V>]
│                         [--user <user>] [-w|--workdir <dir>] [--timeout <sec>]
│                         [--cap <cap>] [-j|--max-parallel <N>]
│
├── shell                 [<distro>] (interactive TTY shell; choose() if omitted)
├── ps                    [<distro>] (displays container process tree via nsenter; matches run.fish ps)
├── reset                 [<distro|all>] (mount-safe reset back to golden zygote via reflink)
├── clean                 [<distro|all>] (crash recovery: force-kill, unmount, scope reset)
├── start                 <distro> [--bind <h:c>] [--bind-ro <h:c>] (manually boots container daemon in background)
├── stop                  <distro> (stops container daemon)
├── copy-to               <distro> <host_src> <container_dst> (copies file/dir into container via machinectl)
└── copy-from             <distro> <container_src> <host_dst> (copies file/dir from container via machinectl)
```

**Global Options**:

- `--distro <ubuntu|fedora|arch|all>`: Target Linux distribution(s).
    - If omitted in interactive mode:
        - For batch commands (`run`, `script`, `setup`, `teardown`, `reset`, `clean`,
          `start`, `stop`): launches `r3bl_tui::choose()` with `HowToChoose::Multiple`
          presenting checkboxes for the 3 supported distros (`Ubuntu 24.04 LTS`,
          `Fedora 41`, `Arch Linux`), omitting any redundant "all" items.
        - For single-machine commands (`shell`): launches `r3bl_tui::choose()` with
          `HowToChoose::Single` to select exactly one container.
    - If omitted in non-interactive mode (CI, automated scripts, pipes): Fails fast
      immediately with an actionable error instructing the user to pass `--distro all` or
      `--distro <name>`.
    - Exception: Read-only inspection commands (`status`, `ps`) default to displaying all
      machines.
- `--ephemeral` / `-x`: Runs commands or scripts in an ephemeral container, discarding all
  state changes on container exit.
- `--max-parallel <N>` / `-j <N>`: Maximum concurrent container instances. Dynamically
  deduced from CPU cores and available RAM if omitted.
- `--bind <host[:container]>` / `--bind-ro <host[:container]>`: Bind mounts for local
  workspaces and test assets (configured via `ArgAction::Append`; single path `/path` maps
  to identical container path `/path:/path`). Mount reconciliation in persistent mode: if
  a container is already running and requested mounts differ from its active mounts in
  `/proc/<pid>/mounts`, Spawny emits a notice to stdout
  (`Notice: Restarting container 'spawny-<distro>' to apply updated bind mounts...`),
  gracefully reboots the container with the updated mounts, and executes the command.
  Explicit status messages are emitted across all 3 lifecycle conditions: fresh boot,
  reboot to apply updated mounts, and reusing existing running container.
- `--env <KEY=VAL>` / `-e <KEY=VAL>`: Injects environment variables into containers
  (configured via `ArgAction::Append`).
- `--timeout <SECONDS>`: Maximum execution timeout for container commands before
  triggering graceful poweroff.
- `--cap <CAPABILITY>`: Grants additional Linux capabilities (defaults include
  `CAP_NET_ADMIN`).
- `--user <USER>`: Specifies execution user (defaults to `tester`, UID 1000).
- `-w <PATH>` / `--workdir <PATH>`: Working directory inside container (defaults to
  `/home/tester`).
- `--non-interactive` (alias `--ci`): Disables interactive menus and spinners, emitting
  plain sequential logs.

### 4. Container & Lifecycle Runners

**Execution Orchestrators in `build-infra/src/spawny/runner/`**:

- `command_runner.rs`: Coordinates `run` and `script` subcommands:
    - Boots container instances (ephemeral via `-x` or persistent working copies).
    - Lifecycle status reporting: Emits an explicit status message for all 3 lifecycle
      conditions (rendered in `r3bl_tui::Spinner` when interactive, or printed as a clean
      log line in non-interactive/CI mode):
        1. Condition 1 (Fresh Boot): `Booting container 'spawny-<distro>'...` (or
           `Booting ephemeral container 'spawny-ephemeral-<distro>-<uuid>'...`).
        2. Condition 2 (Reboot for Mounts):
           `Restarting container 'spawny-<distro>' to apply updated bind mounts...`.
        3. Condition 3 (Reusing Container):
           `Reusing running container 'spawny-<distro>' (mounts match)...`.
    - Mount reconciliation in persistent mode: inspects `/proc/<leader_pid>/mounts`. If
      requested `--bind` or `--bind-ro` mounts are already present, proceeds immediately
      (Condition 3). If missing or mismatched, emits the restart notice (Condition 2),
      gracefully restarts the container with the updated mounts, and executes the command.
    - Injects `HOME`, `USER`, and `DEFAULT_PATH` via container `/usr/bin/env` wrapper.
    - Manages RAII process supervisor guard to guarantee container shutdown on exit.
    - Coordinates concurrent multi-distro runs with parallel output capture and progress
      spinners.
- `shell_runner.rs`: Coordinates `shell` subcommand:
    - Auto-boots target container if stopped.
    - Drops privileges to user `tester` via `nsenter`.
    - Spawns `/usr/bin/script -q /dev/null -c "/bin/bash --login"` allocating an internal
      PTY pair.
    - Displays banner and supports standard terminal exit (`exit` / `Ctrl+D`) and command
      cancel (`Ctrl+C`).
- `lifecycle_runner.rs`: Coordinates container and image lifecycle operations:
    - `setup` / `build`: Invokes `image_builder` across requested distros with progress
      tracking.
    - `teardown`: Powers off machines and mount-safe deletion of disk artifacts.
    - `status` / `list`: Collects machine, IP, and zygote health status and bridges to
      `status_table::render()`.
    - `ps`: Queries container leader PID and executes `ps f --forest` via `nsenter`.
    - `start` & `stop`: Boots or stops persistent background containers (`start` accepts
      optional `--bind` and `--bind-ro` to pre-configure daemon mounts).
    - `copy-to` & `copy-from`: Copies files or directories into or out of a running
      container by resolving `<distro>` to `spawny-<distro>` and delegating to
      `machinectl copy-to` / `machinectl copy-from` via sudo.
    - `reset`: Fast single-file Copy-on-Write restoration via `cp --reflink=auto`.
    - `clean`: Crash recovery clearing lingering mounts, scopes, and stuck processes.

### 5. Interactive TUI Integration

**Rich Terminal UX in `build-infra/src/spawny/tui/` Powered by `r3bl_tui`**:

- **Interactivity Gate**: Queries `check_is_terminal_interactive()` from
  `r3bl_tui::core::term::term_api`:
    - If interactive: Renders live unified coordinator `r3bl_tui::Spinner` reflecting all
      concurrent distro tasks, and launches `r3bl_tui::choose()` menus when required
      arguments are omitted.
    - If non-interactive (CI, pipes, scripts): Disables spinners, prints sequential
      timestamped log lines, and fails fast if sudo or required arguments are missing.
- **Unified Multi-Distro Spinner**: A single coordinator `Spinner` tracking status across
  all active containers (e.g., `[Ubuntu: running, Fedora: completed, Arch: running]`),
  preventing raw mode collisions from multiple concurrent spinners.
- **Interactive Selection**: Contextual `r3bl_tui::choose()` menus:
    - Multiselect via `HowToChoose::Multiple` for batch commands (`run`, `script`,
      `setup`, `teardown`, `reset`, `clean`, `start`, `stop`), presenting checkboxes for
      the 3 distributions (`Ubuntu`, `Fedora`, `Arch`) without redundant "all" items.
    - Single-select via `HowToChoose::Single` for `shell` container selection.
    - Action launcher menu when `spawny` is invoked with no arguments.
- **Formatted Status Tables**: Renders styled machine status, IP addresses, and zygote
  health tables. Captured logs from failed runs are formatted clearly below the summary
  table.

### 6. Spawny Binary Entry Point

**Binary Entry Point in `build-infra/src/bin/spawny.rs`**:

- Target Gating & Import Isolation: The entire `spawny` module in `build-infra/src/lib.rs`
  is gated with `#[cfg(target_os = "linux")]`. To prevent Windows cross-compilation errors
  (`cargo check --target x86_64-pc-windows-msvc` in `./check.fish --full`), `spawny.rs`
  must NOT contain un-gated top-level `use r3bl_build_infra::spawny::*;` imports. Instead,
  all library imports are scoped inside a `#[cfg(target_os = "linux")] mod linux { ... }`
  block. On non-Linux platforms, `spawny.rs` compiles a clean terminal stub:

    ```rust
    #[cfg(target_os = "linux")]
    mod linux {
        use r3bl_build_infra::spawny::{cli, runner};

        pub async fn main() -> miette::Result<()> {
            // Linux container orchestration logic
            Ok(())
        }
    }

    #[cfg(target_os = "linux")]
    #[tokio::main]
    async fn main() -> miette::Result<()> {
        linux::main().await
    }

    #[cfg(not(target_os = "linux"))]
    fn main() {
        eprintln!("spawny is only supported on Linux (requires systemd-nspawn).");
        std::process::exit(1);
    }
    ```

- Integrates with `r3bl-build-infra` package suite
  (`cargo install --path build-infra --force`).
- Note: Standalone self-upgrade (`--upgrade`) will be wired across all binaries as part of
  `task/binaries-self-upgrade-support.md`.

---

## Implementation Plan

#### Phase 1: mkosi Configuration, Module Wiring & Prereq Checks

**Declarative image definitions, incremental compilation setup, and environment
validation**:

- [ ] Add `include_dir` and `uuid` (with `v4` feature) to `[dependencies]` in
      `build-infra/Cargo.toml`, and move `tempfile` from `[dev-dependencies]` to
      `[dependencies]` for ephemeral config extraction.
- [ ] Export `spawny` module in `build-infra/src/lib.rs` gated by
      `#[cfg(target_os = "linux")]`, creating stub module files for all submodules so
      `./check.fish --check` compiles and checks cleanly across every phase.
- [ ] Create `build-infra/src/spawny/mod.rs` containing comprehensive module-level rustdoc
      documentation (`//!`) detailing the complete architecture:
      1. Mental model and container landscape taxonomy: System containers
      (`systemd-nspawn`) vs Application containers (Docker / OCI) vs Fleet orchestrators
      (Kubernetes). Explains why system containers booting authentic `systemd` as PID 1
      are required for installer and package testing, including a comparison table (QEMU/VM
      vs Docker vs systemd-nspawn).
      2. Background on `mkosi` ("Make Operating System Image", official systemd project
      tool): declarative build-time image factory (`mkosi.conf` vs `Dockerfile`), how it
      assembles bootable `.raw` GPT disk images directly from upstream package repositories
      (`apt`, `dnf5`, `pacman`) without running a daemon or VM, and the separation of
      concerns between build-time image generation (`mkosi`) and runtime sandbox execution
      (`systemd-nspawn`).
      3. Single-file raw GPT disk image storage architecture (`Format=disk`,
      `<distro>.raw`), BTRFS CoW reflinks, and elimination of nested subvolume deletion
      issues. Ephemeral clean-room throwaway (via `--ephemeral`) vs
      persistent sandbox execution workflows with ASCII flowcharts. Privilege model and
      execution safety (unprivileged CLI, unconditional passwordless sudo requirement via
      `sudo -n true`, defensive path validation, and mount safety before deletion).
      Environment variable injection and shell independence architecture (why sudo host
      `env_reset` strips caller environment, why `nsenter` bypasses PAM and ignores
      `/etc/environment`, why relying on `.profile`/`.bashrc` causes fragile execution
      failures, and how Spawny guarantees shell-agnostic determinism by injecting `HOME`,
      `USER`, and `DEFAULT_PATH` via container `/usr/bin/env`, merging user-supplied
      `--env` flags). Multi-distro zygote bootstrapping architecture (why `mkosi` drives
      host-native tools `apt`, `dnf5`, and `pacman` as cross-installation root unpackers
      to construct guest disk images without heavy VM or Docker daemons, why host `apt`
      never touches host system files, and how build-time zygote creation differs from
      run-time container execution). Submodule coordinator declarations and public
      re-exports (`cli`, `constants`, `nspawn`, `runner`, `tui`).
- [ ] Implement `build-infra/src/spawny/constants.rs`: Centralized constants and defaults
      organized into namespaced modules (`container_defaults` for `DEFAULT_UID`,
      `DEFAULT_UID_STR`, `DEFAULT_GID`, `DEFAULT_GID_STR`, `DEFAULT_USER`, `DEFAULT_HOME`,
      `DEFAULT_WORKDIR`, `DEFAULT_SHELL`; `host_defaults` for `MIN_HOST_FREE_DISK_BYTES`
      set to 20 GB, with rustdocs cross-referencing `OutputSize=15G` in `mkosi.conf`;
      `host_paths` for `ZYGOTES_DIR`, `MACHINES_DIR`, `UNIX_EXPORT_DIR`,
      `EPHEMERAL_CONFIG_PREFIX`, `CONTAINER_NAME_PREFIX`, and `FORBIDDEN_DELETION_PATHS`
      array, with rustdocs on `UNIX_EXPORT_DIR` explaining the host-container socket
      bridge and why unmounting/cleanup is required after abrupt container termination).
      Eliminates magic numbers/strings and avoids runtime string allocations.
- [ ] Create `build-infra/mkosi/mkosi.conf` with shared base configuration specifying
      `Format=disk` (raw GPT disk image), `WithNetwork=yes` (network access enabled for
      chroot toolchain installation), and `OutputSize=15G` (preceded by a comment
      referencing `MIN_HOST_FREE_DISK_BYTES` in `constants.rs`).
- [ ] Create `build-infra/mkosi/mkosi.profiles/ubuntu/mkosi.conf` for Ubuntu 24.04 LTS.
- [ ] Create `build-infra/mkosi/mkosi.profiles/fedora/mkosi.conf` for Fedora 41.
- [ ] Create `build-infra/mkosi/mkosi.profiles/arch/mkosi.conf` for Arch Linux rolling.
- [ ] Create `build-infra/mkosi/mkosi.postinst.chroot` for hermetic setup: Creates user
      `tester` explicitly with home directory
      (`useradd -m -s /usr/bin/fish -u 1000 -U tester`). Configures passwordless sudo in
      `/etc/sudoers.d/tester`. Installs Rust toolchain via `rustup` for user `tester` over
      enabled network (ensuring Rust 1.85+ / edition 2024). Configures `/etc/environment`
      to ensure `/home/tester/.cargo/bin` is in `$PATH`.
- [ ] Implement `build-infra/src/spawny/nspawn/system_info.rs`: Sudo pre-validation:
      unconditionally validates passwordless sudo via `sudo -n true`; fails fast with
      actionable error (`/etc/sudoers.d/spawny`) if disabled. Host distribution detection:
      defines `HostDistroFamily` enum (`Arch`, `Debian`, `Fedora`) providing
      `package_manager()` and `install_cmd()`. Resolves host distribution via 3-tier
      pipeline (Tier 1 prioritizes `ID_LIKE` tokens e.g. `ID_LIKE=arch` for CachyOS, Tier
      2 falls back to `ID` for pure upstream distros lacking `ID_LIKE`, and Tier 3 probes
      host `$PATH` for `pacman`/`apt-get`/`dnf` as defense-in-depth). Captures
      `PRETTY_NAME` for display logs. Hardware capacity deduction (RAM from
      `/proc/meminfo`, cores from `available_parallelism`). Free disk space validation:
      queries `/var/lib` via `statvfs` against `MIN_HOST_FREE_DISK_BYTES` (20 GB).
- [ ] Implement `build-infra/src/spawny/nspawn/prereqs.rs`: Validates core host utilities:
      `systemd-nspawn`, `machinectl`, and `mkosi`. Uses `HostDistroFamily` to distinguish
      host package managers (`pacman`, `apt`, `dnf`) and generate install commands.
      Distinguishes host package managers from guest bootstrapping tools (`apt` and
      `ubuntu-keyring` for Ubuntu, `dnf5` for Fedora, `pacman` for Arch). Includes
      comprehensive rustdocs explaining why `apt` + `ubuntu-keyring` and `dnf5` are
      required on non-Debian/non-Fedora hosts (e.g. CachyOS): `mkosi` invokes them as
      cross-installation unpackers (`-o RootDir=...`, `--installroot=...`) to assemble
      target `.raw` images without touching host packages, and they are strictly
      build-time prerequisites (only invoked during `spawny setup`). Validates terminal
      interactivity.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/Cargo.toml`
    - [ ] `build-infra/src/lib.rs`
    - [ ] `build-infra/src/spawny/mod.rs`
    - [ ] `build-infra/src/spawny/constants.rs`
    - [ ] `build-infra/mkosi/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/ubuntu/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/fedora/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.profiles/arch/mkosi.conf`
    - [ ] `build-infra/mkosi/mkosi.postinst.chroot`
    - [ ] `build-infra/src/spawny/nspawn/system_info.rs`
    - [ ] `build-infra/src/spawny/nspawn/prereqs.rs`

### Phase 2: Core nspawn, Zygote & System Info Engine

**Low-level machine, snapshot, mount safety, and hardware capacity primitives**:

- [ ] Implement `build-infra/src/spawny/nspawn/distro.rs`: Defines supported
      distributions, image paths (`/var/lib/spawny/zygotes/<distro>.raw` and
      `/var/lib/machines/spawny-<distro>.raw`), and machine name mappings. Generates
      unique machine instance IDs for ephemeral runs (`spawny-ephemeral-<distro>-<uuid>`).
- [ ] Implement `build-infra/src/spawny/nspawn/machine.rs`: Container auto-boot
      (`systemd-nspawn -b -i <path.raw>`) with `--capability=CAP_NET_ADMIN` and
      `--resolv-conf=copy-uplink`. Dynamically applies resource limits via
      `--property=CPUQuota=...`, `--property=MemoryMax=...`, and `--private-network` if
      provided. Wait-for-boot readiness polling with timeout. Process
      execution via `nsenter` (dropping privileges using `DEFAULT_UID_STR` and
      `DEFAULT_GID_STR` from `constants::container_defaults`, injecting `HOME`, `USER`,
      and `DEFAULT_PATH` via container `/usr/bin/env`, working in `--workdir`).
      Interactive shell with internal PTY wrapper
      (`/usr/bin/script -q /dev/null -c "/bin/bash --login"`). Displays entry banner with
      exit instructions (`exit` / `Ctrl+D`) and command cancel (`Ctrl+C`). Multi-stage
      shutdown escalation (`poweroff` -> `terminate` -> `kill` -> `pkill -9` -> scope
      reset). Defensive path validation: verifies target paths strictly match
      `/var/lib/machines/spawny-*`, `/var/lib/spawny/zygotes/*`, or
      `/run/systemd/nspawn/unix-export/spawny-*` before executing privileged removals
      (`rm -rf`). Mount safety: inspects `/proc/mounts`, cleanly unmounts lingering child
      mounts, clears container sockets in `/run/systemd/nspawn/unix-export/spawny-*`, and
      resets failed scopes before deletion. Includes comprehensive rustdocs documenting
      the lifecycle of UNIX runtime sockets, why abrupt process termination leaves
      lingering mounts, and how `validate_deletion_path` prevents catastrophic host
      deletion.
- [ ] Implement `build-infra/src/spawny/nspawn/zygote.rs`: Ephemeral execution: dispatches
      to `systemd-nspawn -b -x -i <image.raw>` with `--capability=CAP_NET_ADMIN` and
      `--resolv-conf=copy-uplink` (including resource limits `--property=CPUQuota=...`,
      `--property=MemoryMax=...`, and `--private-network` if provided), managed via an RAII
      supervisor guard. Mount-safe reset: fast single-file Copy-on-Write restoration via `cp --reflink=auto` (BTRFS/XFS,
      `<0.01s`) or sparse copy (ext4, ~1-2s) after mount safety and defensive path checks.
- [ ] Implement `build-infra/src/spawny/nspawn/image_builder.rs`: Unpacks embedded `mkosi`
      configs to an ephemeral temporary directory (`tempfile::tempdir()`) with `0o755`
      permissions on scripts (or uses `--config-dir` override), invokes `mkosi` with
      profile arguments and `--runtime-size=<disk-size>`, and saves output to
      `/var/lib/spawny/zygotes/<distro>.raw`.
- [ ] Add unit tests for distro mapping, hardware capacity calculations, and state
      transitions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/nspawn/distro.rs`
    - [ ] `build-infra/src/spawny/nspawn/machine.rs`
    - [ ] `build-infra/src/spawny/nspawn/zygote.rs`
    - [ ] `build-infra/src/spawny/nspawn/image_builder.rs`

### Phase 3: CLI Parser & TUI Layer

**Command-line configuration and interactive TUI components**:

- [ ] Implement `build-infra/src/spawny/cli/` using `clap` derive: Flat subcommands `run`,
      `script`, `shell`, `ps`, `reset`, `clean`, `start` (accepting `--bind` and
      `--bind-ro`), `stop`, `copy-to`, `copy-from`, `status` (alias `list`), `setup`, and
      `teardown`. Flags: `--ephemeral` / `-x`, `--distro`, `--max-parallel` / `-j`,
      `--bind`, `--bind-ro`, `--env` (all using `ArgAction::Append`), `--timeout`,
      `--cap`, `--user` (defaults to `DEFAULT_USER`), `-w` / `--workdir` (defaults to
      `DEFAULT_WORKDIR`), `--cpu-quota`, `--memory-max`, `--private-network`,
      `--disk-size` (default `20G` for `setup`), and `--non-interactive` / `--ci`. Validates `--distro`: in
      non-interactive mode (CI/scripts), fails fast with actionable error if `--distro` is
      omitted on execution/batch commands; in interactive mode, delegates to
      `choose_menu`.
- [ ] Implement `build-infra/src/spawny/tui/choose_menu.rs` wrapping `r3bl_tui::choose()`:
      `choose_distros_multiselect()` using `HowToChoose::Multiple` presenting checkboxes
      for the 3 distributions (`Ubuntu 24.04 LTS`, `Fedora 41`, `Arch Linux`) without
      redundant "all" items for batch commands; `choose_distro_single()` using
      `HowToChoose::Single` for `shell`; and action launcher menu for bare `spawny`.
- [ ] Implement `build-infra/src/spawny/tui/status_table.rs` formatting machine and zygote
      state tables.
- [ ] Implement `build-infra/src/spawny/tui/progress.rs` rendering live parallel
      multi-distro progress via a single unified coordinator `r3bl_tui::Spinner` when
      interactive, or sequential logs when non-interactive.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/cli/mod.rs`
    - [ ] `build-infra/src/spawny/cli/args.rs`
    - [ ] `build-infra/src/spawny/tui/choose_menu.rs`
    - [ ] `build-infra/src/spawny/tui/status_table.rs`
    - [ ] `build-infra/src/spawny/tui/progress.rs`

### Phase 4: Container & Command Runners

**Execution orchestrators for test and sandbox workflows**:

- [ ] Implement `build-infra/src/spawny/runner/command_runner.rs`: Unified command and
      script execution with RAII process supervisor guard. When `--ephemeral` (`-x`) is
      specified, auto-boots ephemeral container(s) with unique machine IDs
      (`systemd-nspawn -b -x -i <zygote.raw>`), executes command or test script via
      `nsenter` (dropping privileges using `DEFAULT_UID_STR` and `DEFAULT_GID_STR`,
      injecting `HOME`, `USER`, and `DEFAULT_PATH` via container `/usr/bin/env` wrapper),
      powers off, and discards all changes. When `--ephemeral` is omitted, auto-boots or
      attaches to the persistent working container (`spawny-<distro>.raw`), with changes
      persisting on disk. Lifecycle status reporting: emits explicit status messages
      across all 3 lifecycle conditions (Condition 1: fresh boot, Condition 2: reboot for
      mount reconciliation, Condition 3: reusing running container with matching mounts).
      Mount reconciliation: when running in persistent mode on an already-running
      container with missing or mismatched `--bind` / `--bind-ro` mounts, emits
      `Notice: Restarting     container 'spawny-<distro>' to apply updated bind mounts...`,
      gracefully restarts the container with the updated mounts, and executes the command.
      Includes comprehensive function rustdocs and inline guardrail comments on
      `Command::new("sudo")` explaining why host `Command::env()` fails due to sudo
      `env_reset` and how `/usr/bin/env` injects container-local PATH. Supports parallel
      execution across distros with unified coordinator spinner and output capture.
- [ ] Implement `build-infra/src/spawny/runner/shell_runner.rs`: Opens interactive TTY
      shell in persistent container via `nsenter` with PTY allocation (auto-booting if
      stopped), dropping privileges using `DEFAULT_UID_STR` and `DEFAULT_GID_STR`,
      displaying banner and supporting standard terminal exit (`exit` / `Ctrl+D`) and
      command cancel (`Ctrl+C`). Includes comprehensive rustdoc documentation on
      `exec_shell()` detailing why `machinectl shell` hangs, why `nsenter` alone lacks a
      container PTY device causing `.profile` ttyname errors, and how
      `/usr/bin/script -q /dev/null` allocates a container-internal PTY pair with zero
      warnings.
- [ ] Implement `build-infra/src/spawny/runner/lifecycle_runner.rs`: Orchestrates
      container and image lifecycle operations: `setup` / `build` (invoking
      `image_builder` across requested distros with coordinator progress spinner and
      passing `--disk-size`),
      `teardown` (powering off machines and mount-safe deletion of disk artifacts),
      `status` / `list` (gathering machine and zygote states, bridging to
      `status_table::render()`), `ps` (querying leader PID and displaying container
      process tree via `machine::show_process_tree`), `start` and `stop` (manual container
      daemon management, with `start` accepting optional `--bind` and `--bind-ro`),
      `copy-to` and `copy-from` (delegating file and directory transfers to
      `machinectl copy-to` / `machinectl copy-from` via sudo), `reset` (mount-safe CoW
      restore via `cp --reflink=auto`), and `clean` (crash recovery clearing lingering
      mounts, scopes, and processes).
- [ ] Implement signal handling (`tokio::signal::ctrl_c`) for non-interactive runners,
      detaching signal handlers during interactive shell sessions.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/src/spawny/runner/command_runner.rs`
    - [ ] `build-infra/src/spawny/runner/shell_runner.rs`
    - [ ] `build-infra/src/spawny/runner/lifecycle_runner.rs`
    - [ ] `build-infra/src/spawny/runner/mod.rs`

### Phase 5: Binary Integration & Workspace Gating

**Binary entry point, packaging, and documentation**:

- [ ] Add `spawny` binary to `build-infra/Cargo.toml` (`[[bin]]`), updating package
      `description` and `keywords` to feature `spawny`.
- [ ] Implement `build-infra/src/bin/spawny.rs` with `#[cfg(target_os = "linux")]` gating
      and non-Linux platform stubs. All `spawny` library imports must be scoped inside an
      inner `#[cfg(target_os = "linux")] mod linux { ... }` block to prevent
      `E0432     unresolved import` failures during Windows cross-compilation in
      `./check.fish --full`.
- [ ] Install binary locally via `cargo install --path build-infra --force` and verify
      `spawny --help`.
- [ ] Update `build-infra/README.md` and `build-infra/src/lib.rs` documentation to include
      `spawny` features, command hierarchy, and usage with intra-doc links to
      `[`spawny`](crate::spawny)` alongside `cargo-rustdoc-fmt`.
- [ ] Update root workspace `README.md` to showcase `spawny` in the binary tool suite.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `build-infra/Cargo.toml`
    - [ ] `build-infra/src/bin/spawny.rs`
    - [ ] `build-infra/src/lib.rs`
    - [ ] `build-infra/README.md`
    - [ ] `README.md`

### Phase 6: Comprehensive Verification & Testing

**Comprehensive validation of spawny across distros before legacy retirement**:

- [ ] Run `spawny setup --distro all` and verify `mkosi` builds Arch, Ubuntu, and Fedora
      raw images (`.raw`) with pre-baked `rustup` toolchains.
- [ ] Run `spawny status` and verify all machines and zygotes are registered.
- [ ] Run `spawny run ubuntu "echo hello"` and verify auto-boot and state persistence.
- [ ] Run `spawny reset ubuntu` and verify mount safety check and instant `<0.01s`
      single-file reflink restore.
- [ ] Run `spawny clean ubuntu` and verify crash recovery.
- [ ] Run `spawny run --ephemeral --distro all "uname -a"` and verify parallel execution
      with unique machine IDs and unified coordinator spinner.
- [ ] Run
      `spawny run --ephemeral --distro all "cargo install r3bl-cmdr && giti --version"`
      and verify clean-room installation testing across distributions.
- [ ] Verify DNS resolution (`--resolv-conf=copy-uplink`) inside containers by running
      `curl` and `cargo --version`.
- [ ] Run `./check.fish --check`, `--build`, `--clippy`, `--test` across the workspace.
- [ ] Run `./check.fish --full` to verify Windows cross-compilation with platform gating.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `task/build-infra-spawny.md`

### Phase 7: Remove Legacy cmdr nspawn Scripts & Update run.fish

**Clean up obsolete workspace shell scripts after verified spawny operation**:

- [ ] Remove `cmdr/systemd-nspawn/` directory from the workspace.
- [ ] Refactor `test-cmdr-install-on-all-linux-distros` in workspace root `run.fish` to
      invoke
      `spawny run --ephemeral all "cargo install r3bl-cmdr && giti --version && edi --version"`
      instead of legacy scripts.
- [ ] Update `build-infra/AGENTS.md` and documentation to reference `spawny`.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `cmdr/` (verify `systemd-nspawn/` removed)
    - [ ] `run.fish` (verify `test-cmdr-install-on-all-linux-distros` uses `spawny`)
    - [ ] `build-infra/AGENTS.md`

### Phase 8: External Test Suite Migration in ~/github/notes

**Migrate host test runner in `~/github/notes/files/scripts/tests/` to invoke spawny**:

- [ ] Refactor the host test runner in
      `/home/nazmul/github/notes/files/scripts/tests/run.fish` to invoke `spawny` CLI
      commands with required bind mounts (`/scripts`, `/synced-data`, `/host-home`)
      instead of sourcing `lib/nspawn.fish`.
- [ ] Preserve unit test runner execution (running pure function tests without state
      reset).
- [ ] Preserve E2E test setup fixtures safely: Host `/etc/ssh` is strictly isolated and
      must NEVER be mounted or copied into the container; each container possesses its own
      private, writable `/etc/ssh` inside its `.raw` disk image (generating any dummy keys
      locally via `ssh-keygen -A`). Input fixtures (`/synced-data`) are mounted read-only
      via `--bind-ro /path/to/synced-data:/synced-data`, guaranteeing zero host mutation
      risks; in-container test scripts create a local copy
      (`cp -a /synced-data ~/synced-data`) when writable fixtures are required. Use
      `spawny copy-to` for any imperative fixture injection into running containers.
      Execute all E2E test scripts (`00` through `05`).
- [ ] Verify that all in-container test scripts execute cleanly inside containers managed
      by `spawny`.
- [ ] Retire obsolete fish helper scripts in
      `/home/nazmul/github/notes/files/scripts/tests/` (`setup.fish`, `teardown.fish`,
      `lib/nspawn.fish`) only after the entire test suite passes under `spawny`.
- [ ] Remove `build-infra/reference/nspawn-scripts/` directory from the workspace now that
      both internal and external test suites have been successfully migrated and verified.
- [ ] **Mandatory manual review:** Verify every file modified in this phase for correct
      implementation and ensure no regressions.
    - [ ] `/home/nazmul/github/notes/files/scripts/tests/` (verify host runner migrated to
          `spawny`)
    - [ ] `build-infra/reference/nspawn-scripts/` (verify directory removed after full
          migration)

---

## Verification Matrix

### Distro Coverage Matrix

| Distribution   | Version / Release | Backend Tool       | Default User | Package Manager | Zygote Path                          |
| :------------- | :---------------- | :----------------- | :----------- | :-------------- | :----------------------------------- |
| **Ubuntu**     | 24.04 LTS (Noble) | `mkosi` + `apt`    | `tester`     | `apt`           | `/var/lib/spawny/zygotes/ubuntu.raw` |
| **Fedora**     | 41                | `mkosi` + `dnf5`   | `tester`     | `dnf`           | `/var/lib/spawny/zygotes/fedora.raw` |
| **Arch Linux** | Rolling           | `mkosi` + `pacman` | `tester`     | `pacman`        | `/var/lib/spawny/zygotes/arch.raw`   |

### Command Verification Checklist

- [ ] `spawny setup`: Builds all 3 distros using `mkosi` and creates golden `.raw` images.
- [ ] `spawny teardown`: Stops running containers and removes images with defensive path
      checks.
- [ ] `spawny run --ephemeral "<command>"`: Clean-room command runner across all distros
      with `--max-parallel`, `--timeout`, and `--cap`.
- [ ] `spawny script --ephemeral <path>`: Clean-room test suite script execution.
- [ ] `spawny run <distro> "<command>"`: Persistent command execution in living sandbox.
- [ ] `spawny shell <distro>`: Interactive TTY container shell with standard terminal exit
      (`exit` / `Ctrl+D`) and command cancel (`Ctrl+C`).
- [ ] `spawny ps <distro>`: Live process tree inspection (`ps f --forest` via `nsenter`).
- [ ] `spawny reset <distro>`: Instant single-file reset to golden snapshot (`<0.01s` via
      `reflink` or sparse copy).
- [ ] `spawny clean <distro>`: Crash recovery clearing stuck processes, scopes, mounts,
      and orphaned snapshots.
- [ ] `spawny status`: Status table rendering via `r3bl_tui` (with `-v` for process
      trees).
- [ ] `spawny copy-to <distro> <host_src> <container_dst>` and
      `spawny copy-from <distro> <container_src> <host_dst>`: File and directory transfer
      verification between host and running container.
- [ ] Cross-platform verification: `./check.fish --full` verifies Windows
      cross-compilation passes via `#[cfg(target_os = "linux")]` gating.
- [ ] Clean up legacy cmdr scripts in `cmdr/systemd-nspawn/` and update `run.fish`.
- [ ] Migrate `~/github/notes/files/scripts/tests/` to use `spawny` (unit and E2E
      `00`-`05`) and retire legacy fish test scripts.

<!-- cspell:words postinst Rootfs machinectl nsenter debootstrap userland sysusers -->
<!-- cspell:words reflink Reflink procs overlayfs Passwordless poweroff passwordless chrooted -->
<!-- cspell:words resolv chrooted reflinks eperm statvfs ttyname nopasswd cachyos installroot -->
