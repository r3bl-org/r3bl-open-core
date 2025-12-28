<!-- START doctoc generated TOC please keep comment here to allow auto update -->
<!-- DON'T EDIT THIS SECTION, INSTEAD RE-RUN doctoc TO UPDATE -->

- [Testing r3bl-cmdr Installation](#testing-r3bl-cmdr-installation)
  - [Supported Distributions](#supported-distributions)
  - [Quick Start](#quick-start)
  - [Understanding systemd-nspawn](#understanding-systemd-nspawn)
    - [What Is It?](#what-is-it)
    - [The Mental Model](#the-mental-model)
    - [Key Differences from Docker](#key-differences-from-docker)
    - [Key Differences from QEMU/VMs](#key-differences-from-qemuvms)
    - [Why systemd-nspawn for This Use Case?](#why-systemd-nspawn-for-this-use-case)
  - [How the Scripts Work](#how-the-scripts-work)
    - [create-containers.fish](#create-containersfish)
    - [run-test.fish](#run-testfish)
    - [cleanup.fish](#cleanupfish)
    - [Entry Point: run.fish](#entry-point-runfish)
  - [Commands Reference](#commands-reference)
    - [Create Containers](#create-containers)
    - [Run Tests](#run-tests)
    - [Clean Up](#clean-up)
    - [Direct systemd-nspawn Commands](#direct-systemd-nspawn-commands)
  - [What Gets Tested](#what-gets-tested)
  - [Requirements](#requirements)
  - [Disk Usage](#disk-usage)

<!-- END doctoc generated TOC please keep comment here to allow auto update -->

# Testing r3bl-cmdr Installation

This directory contains scripts to test `r3bl-cmdr` installation on multiple Linux distributions
using systemd-nspawn containers.

## Supported Distributions

| Distribution | Version       | Container Name |
| ------------ | ------------- | -------------- |
| Ubuntu       | 24.04 (noble) | cmdr-ubuntu    |
| Fedora       | 41            | cmdr-fedora    |
| Arch Linux   | Rolling       | cmdr-arch      |

## Quick Start

```bash
# 1. Create containers (one-time setup, ~5-10 min total)
./create-containers.fish all

# 2. Run installation test on all distros
./run-test.fish all

# 3. Clean up when done
./cleanup.fish
```

Or from the workspace root:

```bash
fish run.fish test-cmdr-install-on-all-linux-distros
```

## Understanding systemd-nspawn

If you're familiar with Docker or QEMU/VMs, here's how systemd-nspawn fits in:

### What Is It?

systemd-nspawn is a lightweight container tool built into systemd. Think of it as a "chroot on
steroids" - it provides process and filesystem isolation without the overhead of full virtualization
or a container daemon.

| Aspect              | QEMU/VM             | Docker           | systemd-nspawn        |
| ------------------- | ------------------- | ---------------- | --------------------- |
| **Startup time**    | 10-60 seconds       | 1-5 seconds      | Instant (&lt;100ms)   |
| **Kernel**          | Separate (emulated) | Shared with host | Shared with host      |
| **Disk format**     | qcow2, raw images   | Layered images   | Plain directories     |
| **Daemon required** | qemu process        | dockerd (always) | None                  |
| **Resource @ idle** | RAM reserved        | Minimal          | Zero                  |
| **Isolation level** | Hardware            | Namespace        | Namespace             |
| **Create from**     | ISO installer       | Dockerfile       | debootstrap / tarball |
| **Networking**      | Virtual NIC         | Bridge/overlay   | Host namespace        |

### The Mental Model

Unlike Docker's layered images or QEMU's disk images, systemd-nspawn containers are just **plain
directories** containing a complete Linux filesystem:

```
Your Host System (e.g., Ubuntu 25.10)
│
└── /var/lib/machines/
    │
    ├── cmdr-ubuntu/          ← Complete Ubuntu 24.04 filesystem
    │   ├── bin/bash
    │   ├── etc/os-release    → ID=ubuntu
    │   ├── usr/bin/apt       → Package manager
    │   └── ...
    │
    ├── cmdr-fedora/          ← Complete Fedora 41 filesystem
    │   ├── bin/bash
    │   ├── etc/os-release    → ID=fedora
    │   ├── usr/bin/dnf       → Package manager
    │   └── ...
    │
    └── cmdr-arch/            ← Complete Arch Linux filesystem
        ├── bin/bash
        ├── etc/os-release    → ID=arch
        ├── usr/bin/pacman    → Package manager
        └── ...
```

When you run `systemd-nspawn -D /var/lib/machines/cmdr-ubuntu`, you get a shell that:

- Sees `/var/lib/machines/cmdr-ubuntu` as its root (`/`)
- Has its own process namespace (can't see host processes)
- Has its own network namespace (optional)
- Shares the host kernel (no emulation overhead)

### Key Differences from Docker

| Docker Concept        | systemd-nspawn Equivalent                             |
| --------------------- | ----------------------------------------------------- |
| `docker run`          | `systemd-nspawn -D /path`                             |
| `docker run --rm`     | `systemd-nspawn --ephemeral -D /path`                 |
| `-v /host:/container` | `--bind=/host:/container` or `--bind-ro=` (read-only) |
| Dockerfile            | debootstrap / manual setup / tarball extraction       |
| Image layers          | None - just a directory (copy/rsync to duplicate)     |
| Docker daemon         | None needed                                           |
| `docker images`       | `ls /var/lib/machines/`                               |
| `docker rmi`          | `rm -rf /var/lib/machines/container-name`             |

### Key Differences from QEMU/VMs

| VM Concept         | systemd-nspawn Equivalent             |
| ------------------ | ------------------------------------- |
| qcow2 disk image   | Plain directory on host filesystem    |
| Boot process       | None - runs command directly          |
| Guest kernel       | Uses host kernel                      |
| Hardware emulation | None - native execution               |
| virsh/virt-manager | machinectl (optional)                 |
| Snapshots          | `--ephemeral` or filesystem snapshots |

### Why systemd-nspawn for This Use Case?

For testing installation scripts across distros, systemd-nspawn is ideal because:

1. **Speed**: Tests start instantly - no container startup overhead
2. **Ephemeral mode**: Each test run starts completely clean (like a fresh VM)
3. **Simplicity**: No daemon, no registry, no Dockerfile to maintain
4. **Native**: Built into systemd, already present on most Linux systems
5. **Accurate**: Tests run in a real distro environment, not a minimal Docker image

## How the Scripts Work

### create-containers.fish

Creates complete Linux filesystems in `/var/lib/machines/`:

| Distro     | Method                 | What Happens                                                  |
| ---------- | ---------------------- | ------------------------------------------------------------- |
| **Ubuntu** | `debootstrap`          | Downloads packages from Ubuntu mirrors, builds minimal system |
| **Fedora** | Official container tar | Downloads Fedora Container Base image, extracts layer         |
| **Arch**   | Bootstrap tarball      | Downloads bootstrap image, initializes pacman keyring         |

**Ubuntu example** (uses debootstrap):

```fish
sudo debootstrap \
    --include=curl,gcc,make,ca-certificates \
    noble \                              # Ubuntu 24.04 codename
    /var/lib/machines/cmdr-ubuntu \      # Target directory
    http://archive.ubuntu.com/ubuntu     # Mirror
```

**Fedora/Arch**: After extraction, the script runs setup commands _inside_ the new container:

```fish
# Install build dependencies in Fedora container
sudo systemd-nspawn -D /var/lib/machines/cmdr-fedora \
    dnf install -y curl gcc gcc-c++ make ca-certificates
```

### run-test.fish

The core testing script. Key concepts:

**Basic invocation:**

```fish
sudo systemd-nspawn -D /var/lib/machines/cmdr-ubuntu /bin/bash /app/install.bash
```

**With options:**

```fish
sudo systemd-nspawn \
    -D /var/lib/machines/cmdr-ubuntu \   # Container root
    --ephemeral \                         # Discard changes on exit
    --bind-ro=$script_dir:/app \          # Mount scripts read-only
    /bin/bash /app/install.bash           # Command to run
```

**Flag reference:**

| Flag                 | Purpose                             | Docker Equivalent    |
| -------------------- | ----------------------------------- | -------------------- |
| `-D <path>`          | Container root filesystem           | Image name           |
| `--ephemeral`        | Overlay filesystem, discard on exit | `--rm` + fresh image |
| `--bind=src:dest`    | Read-write bind mount               | `-v src:dest`        |
| `--bind-ro=src:dest` | Read-only bind mount                | `-v src:dest:ro`     |

**Ephemeral mode** creates an overlay filesystem - all writes go to a temporary layer that's
discarded when the container exits. The base container remains unchanged, making it perfect for
repeatable testing.

### cleanup.fish

Since containers are just directories, cleanup is straightforward:

```fish
sudo rm -rf /var/lib/machines/cmdr-ubuntu
rm -f /tmp/Fedora-Container-Base-*.tar.xz
rm -f /tmp/archlinux-bootstrap-*.tar.zst
```

The `--dry-run` flag shows what would be deleted without actually deleting.

### Entry Point: run.fish

From the workspace root, `fish run.fish test-cmdr-install-on-all-linux-distros`:

```fish
function test-cmdr-install-on-all-linux-distros
    # 1. Verify we're on Linux (systemd-nspawn is Linux-only)
    if test (uname) = "Darwin"
        echo "Error: systemd-nspawn is Linux-only."
        return 1
    end

    # 2. Create containers if they don't exist (one-time setup)
    for container in cmdr-ubuntu cmdr-fedora cmdr-arch
        if not test -d "/var/lib/machines/$container"
            ./create-containers.fish all
            break
        end
    end

    # 3. Run tests in ephemeral mode (clean-room)
    ./run-test.fish --ephemeral all
end
```

## Commands Reference

### Create Containers

```bash
# Create all containers
./create-containers.fish all

# Create specific distro
./create-containers.fish ubuntu
./create-containers.fish fedora
./create-containers.fish arch
```

Containers are stored in `/var/lib/machines/` and persist until deleted.

### Run Tests

```bash
# Test on all distros
./run-test.fish all

# Test on specific distro
./run-test.fish ubuntu

# Clean-room test (discard changes after)
./run-test.fish --ephemeral all

# Interactive shell for debugging
./run-test.fish --shell fedora
```

### Clean Up

```bash
# Preview what would be deleted
./cleanup.fish --dry-run

# Delete containers and cached downloads
./cleanup.fish
```

### Direct systemd-nspawn Commands

```bash
# Run a command in container
sudo systemd-nspawn -D /var/lib/machines/cmdr-ubuntu cat /etc/os-release

# Interactive shell
sudo systemd-nspawn -D /var/lib/machines/cmdr-ubuntu

# Mount host directory into container
sudo systemd-nspawn -D /var/lib/machines/cmdr-ubuntu \
    --bind-ro=/path/on/host:/path/in/container \
    /bin/bash

# Ephemeral (discard changes on exit)
sudo systemd-nspawn --ephemeral -D /var/lib/machines/cmdr-ubuntu
```

## What Gets Tested

The `install.bash` script:

1. Detects the package manager (apt/dnf/pacman)
2. Installs build dependencies (curl, gcc, make)
3. Installs Rust via rustup
4. Installs r3bl-cmdr from crates.io
5. Verifies `edi`, `giti`, and `rc` commands work

## Requirements

- Linux with systemd (Ubuntu, Fedora, Arch, etc.)
- `systemd-container` package (for systemd-nspawn)
- `debootstrap` package (for creating Ubuntu containers)
- sudo access

On Ubuntu/Debian:

```bash
sudo apt install systemd-container debootstrap
```

## Disk Usage

| Container | Approximate Size |
| --------- | ---------------- |
| Ubuntu    | ~300-400 MB      |
| Fedora    | ~400-500 MB      |
| Arch      | ~500-600 MB      |

Cached download images in `/tmp/` can be deleted after container creation.
