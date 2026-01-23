#!/usr/bin/env fish

# Create systemd-nspawn machines for testing
#
# Usage:
#   ./setup.fish              # Create all machines
#   ./setup.fish ubuntu       # Create ubuntu only
#   ./setup.fish fedora       # Create fedora only
#   ./setup.fish arch         # Create arch only

set tests_dir (dirname (status filename))
set scripts_dir (realpath $tests_dir/..)
source $tests_dir/lib/ensure-prereqs.fish
source $tests_dir/lib/nspawn.fish

# ============================================================================
# Machine Creation Functions
# ============================================================================

function create_fish_config -a machine_path home_dir
    set -l config_dir $machine_path$home_dir/.config/fish
    sudo mkdir -p $config_dir
    echo '# Minimal fish config that sources from bind-mounted /scripts/fish/
# This allows ~/.config/fish/ to remain writable for fish_variables

if test -f /scripts/fish/config.fish
    source /scripts/fish/config.fish
end' | sudo tee $config_dir/config.fish > /dev/null
end

function setup_user_configs -a machine_path
    echo "Creating user configs..."
    create_profile $machine_path /root
    create_profile $machine_path /home/tester
    create_fish_config $machine_path /root
    create_fish_config $machine_path /home/tester
    sudo chroot $machine_path chown -R tester:tester /home/tester
end

function create_profile -a machine_path home_dir
    echo '# ~/.profile: executed by the command interpreter for login shells.

# if running bash
if [ -n "$BASH_VERSION" ]; then
    if [ -f "$HOME/.bashrc" ]; then
        . "$HOME/.bashrc"
    fi
fi

# set PATH so it includes user private bin if it exists
if [ -d "$HOME/bin" ] ; then
    PATH="$HOME/bin:$PATH"
fi

if [ -d "$HOME/.local/bin" ] ; then
    PATH="$HOME/.local/bin:$PATH"
fi

# Load Rust cargo env vars if present
if [ -f "$HOME/.cargo/env" ] ; then
    . "$HOME/.cargo/env"
fi' | sudo tee $machine_path$home_dir/.profile > /dev/null
end

function create_ubuntu
    set machine_name (get_machine_name ubuntu)
    set machine_path (get_machine_path ubuntu)

    if sudo test -d $machine_path
        echo "$machine_name already exists"
        echo "To recreate: ./teardown.fish ubuntu && ./setup.fish ubuntu"
        return 0
    end

    echo "=== Creating Ubuntu Machine ==="
    echo "This will take several minutes..."
    echo ""

    # Download Ubuntu cloud image
    set ubuntu_version "25.10"
    set ubuntu_image "ubuntu-$ubuntu_version-server-cloudimg-amd64-root.tar.xz"
    set ubuntu_url "https://cloud-images.ubuntu.com/releases/$ubuntu_version/release/$ubuntu_image"

    if not test -f /tmp/$ubuntu_image
        echo "Downloading Ubuntu cloud image..."
        wget -q --show-progress -O /tmp/$ubuntu_image "$ubuntu_url" 2>&1
        if test $status -ne 0
            echo "Failed to download Ubuntu image"
            return 1
        end
    end

    echo "Extracting image..."
    sudo mkdir -p $machine_path
    sudo tar -xf /tmp/$ubuntu_image -C $machine_path

    cleanup_stale_mounts ubuntu

    echo "Installing packages..."
    sudo systemd-nspawn -D $machine_path \
        --bind-ro=/etc/resolv.conf:/etc/resolv.conf \
        --bind-ro=$scripts_dir:/scripts \
        --setenv=DEBIAN_FRONTEND=noninteractive \
        /scripts/local-backup-restore/fresh-install/bootstrap-steps/01-install-base-packages.bash

    if test $status -ne 0
        echo "Failed to create Ubuntu machine"
        return 1
    end

    echo "root:test" | sudo chroot $machine_path chpasswd
    sudo chroot $machine_path useradd -m -s /usr/bin/fish tester
    echo "tester:test" | sudo chroot $machine_path chpasswd
    echo "tester ALL=(ALL) NOPASSWD:ALL" | sudo tee $machine_path/etc/sudoers.d/tester > /dev/null

    # Suppress debconf dialogs in test environment
    echo "DEBIAN_FRONTEND=noninteractive" | sudo tee -a $machine_path/etc/environment > /dev/null

    setup_user_configs $machine_path

    echo "$machine_name created"

    # Save as zygote (golden image)
    save_zygote ubuntu
end

function create_fedora
    set machine_name (get_machine_name fedora)
    set machine_path (get_machine_path fedora)

    if sudo test -d $machine_path
        echo "$machine_name already exists"
        echo "To recreate: ./teardown.fish fedora && ./setup.fish fedora"
        return 0
    end

    echo "=== Creating Fedora Machine ==="
    echo "This will take several minutes..."
    echo ""

    set fedora_version 43
    set fedora_image "registry.fedoraproject.org/fedora:$fedora_version"

    echo "Pulling Fedora image with podman..."
    sudo mkdir -p $machine_path

    set oci_container_id (podman create $fedora_image)
    if test $status -ne 0
        echo "Failed to pull Fedora image"
        return 1
    end

    echo "Extracting filesystem..."
    podman export $oci_container_id | sudo tar -xf - -C $machine_path
    podman rm $oci_container_id >/dev/null

    cleanup_stale_mounts fedora

    echo "Configuring dnf speedups..."
    sudo systemd-nspawn -D $machine_path \
        --bind-ro=/etc/resolv.conf:/etc/resolv.conf \
        bash -c "echo 'fastestmirror=True' >> /etc/dnf/dnf.conf && echo 'max_parallel_downloads=10' >> /etc/dnf/dnf.conf"

    echo "Installing packages..."
    sudo systemd-nspawn -D $machine_path \
        --bind-ro=/etc/resolv.conf:/etc/resolv.conf \
        --bind-ro=$scripts_dir:/scripts \
        /scripts/local-backup-restore/fresh-install/bootstrap-steps/01-install-base-packages.bash

    if test $status -ne 0
        echo "Failed to create Fedora machine"
        return 1
    end

    echo "root:test" | sudo chroot $machine_path chpasswd
    sudo chroot $machine_path useradd -m -s /usr/bin/fish tester
    echo "tester:test" | sudo chroot $machine_path chpasswd
    echo "tester ALL=(ALL) NOPASSWD:ALL" | sudo tee $machine_path/etc/sudoers.d/tester > /dev/null

    setup_user_configs $machine_path

    echo "$machine_name created"

    # Save as zygote (golden image)
    save_zygote fedora
end

function create_arch
    set machine_name (get_machine_name arch)
    set machine_path (get_machine_path arch)

    if sudo test -d $machine_path
        echo "$machine_name already exists"
        echo "To recreate: ./teardown.fish arch && ./setup.fish arch"
        return 0
    end

    echo "=== Creating Arch Linux Machine ==="
    echo "This will take several minutes..."
    echo ""

    set arch_mirror "https://geo.mirror.pkgbuild.com"
    set arch_image "archlinux-bootstrap-x86_64.tar.zst"

    if not test -f /tmp/$arch_image
        echo "Downloading Arch bootstrap image..."
        for try_date in (date +%Y.%m.01) (date -d "1 month ago" +%Y.%m.01 2>/dev/null || date -v-1m +%Y.%m.01 2>/dev/null)
            set arch_url "$arch_mirror/iso/$try_date/$arch_image"
            wget -q --show-progress -O /tmp/$arch_image "$arch_url" 2>&1
            if test $status -eq 0
                break
            end
        end
        if not test -f /tmp/$arch_image
            echo "Failed to download Arch image"
            return 1
        end
    end

    echo "Extracting image..."
    sudo mkdir -p $machine_path
    sudo tar -xf /tmp/$arch_image -C $machine_path --strip-components=1

    cleanup_stale_mounts arch

    echo "Enabling pacman mirrors..."
    sudo sed -i 's/^#Server = https:\/\/geo.mirror.pkgbuild.com/Server = https:\/\/geo.mirror.pkgbuild.com/' $machine_path/etc/pacman.d/mirrorlist

    echo "Initializing pacman keyring..."
    # Create gnupg directory with correct permissions before keyring init
    sudo mkdir -p $machine_path/etc/pacman.d/gnupg
    sudo chmod 700 $machine_path/etc/pacman.d/gnupg

    # Initialize keyring - needs --pipe for non-interactive and error checking
    if not sudo systemd-nspawn -D $machine_path \
        --bind-ro=/etc/resolv.conf:/etc/resolv.conf \
        --pipe \
        bash -c "pacman-key --init && pacman-key --populate archlinux"
        echo "Failed to initialize pacman keyring"
        return 1
    end

    echo "Installing packages..."
    sudo systemd-nspawn -D $machine_path \
        --bind-ro=/etc/resolv.conf:/etc/resolv.conf \
        --bind-ro=$scripts_dir:/scripts \
        /scripts/local-backup-restore/fresh-install/bootstrap-steps/01-install-base-packages.bash

    if test $status -ne 0
        echo "Failed to create Arch machine"
        return 1
    end

    echo "root:test" | sudo chroot $machine_path chpasswd
    sudo chroot $machine_path useradd -m -s /usr/bin/fish tester
    echo "tester:test" | sudo chroot $machine_path chpasswd
    echo "tester ALL=(ALL) NOPASSWD:ALL" | sudo tee $machine_path/etc/sudoers.d/tester > /dev/null

    setup_user_configs $machine_path

    echo "$machine_name created"

    # Save as zygote (golden image)
    save_zygote arch
end

# ============================================================================
# Logging Helpers
# ============================================================================

function setup_distro_with_logging -a distro
    set log_path (get_setup_log_path $distro)
    echo "=== Setup $distro started at "(date)" ===" > $log_path
    echo "Log file: $log_path"

    # Run the create function and tee output to log
    switch $distro
        case ubuntu
            create_ubuntu 2>&1 | tee -a $log_path
        case fedora
            create_fedora 2>&1 | tee -a $log_path
        case arch
            create_arch 2>&1 | tee -a $log_path
    end
    set result $pipestatus[1]

    echo "" >> $log_path
    echo "=== Setup $distro completed at "(date)" ===" >> $log_path
    echo "Exit status: $result" >> $log_path

    return $result
end

# ============================================================================
# Main
# ============================================================================

set target $argv[1]
if test -z "$target"
    set target "all"
end

# Check prerequisites
if test "$target" != "-h" -a "$target" != "--help"
    if not ensure_host_prereqs
        exit 1
    end
end

switch $target
    case ubuntu
        setup_distro_with_logging ubuntu
    case fedora
        setup_distro_with_logging fedora
    case arch
        setup_distro_with_logging arch
    case all
        setup_distro_with_logging ubuntu
        echo ""
        setup_distro_with_logging fedora
        echo ""
        setup_distro_with_logging arch
    case status
        echo "=== Machines ($MACHINES_DIR/) ==="
        for distro in $ALL_DISTROS
            set machine_path (get_machine_path $distro)
            if sudo test -d $machine_path
                set size (get_dir_size $machine_path)
                echo "  $machine_path ($size)"
            else
                echo "  $machine_path (not found)"
            end
        end
        echo ""
        show_zygote_status
        exit 0
    case -h --help
        echo "Usage: ./setup.fish [ubuntu|fedora|arch|all|status]"
        echo ""
        echo "Creates systemd-nspawn machines for testing."
        echo "Machines are created in $MACHINES_DIR/"
        echo "Zygotes (golden images) are saved in $ZYGOTES_DIR/"
        echo ""
        echo "Commands:"
        echo "  all      Create all machines (default)"
        echo "  ubuntu   Create ubuntu only"
        echo "  fedora   Create fedora only"
        echo "  arch     Create arch only"
        echo "  status   Show machine and zygote status (directory sizes)"
        echo ""
        echo "Log files: /tmp/setup-<distro>.log"
        exit 0
    case '*'
        echo "Unknown: $target"
        echo "Usage: ./setup.fish [ubuntu|fedora|arch|all]"
        exit 1
end

echo ""
echo "Next: ./run.fish"
