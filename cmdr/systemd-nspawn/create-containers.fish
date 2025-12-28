#!/usr/bin/env fish

# Create systemd-nspawn containers for testing r3bl-cmdr installation
#
# Usage: ./create-containers.fish [ubuntu|fedora|arch|all]
#
# Containers are created in /var/lib/machines/

set target $argv[1]
if test -z "$target"
    set target "all"
end

# Container names
set UBUNTU_CONTAINER "cmdr-ubuntu"
set FEDORA_CONTAINER "cmdr-fedora"
set ARCH_CONTAINER "cmdr-arch"

function ensure_dependencies
    # Check for systemd-nspawn
    if not command -q systemd-nspawn
        echo "Installing systemd-container..."
        sudo apt install -y systemd-container
    end

    # Check for debootstrap (Ubuntu)
    if not command -q debootstrap
        echo "Installing debootstrap..."
        sudo apt install -y debootstrap
    end
end

function create_ubuntu_container
    set container_path "/var/lib/machines/$UBUNTU_CONTAINER"

    if test -d $container_path
        echo "Ubuntu container already exists at $container_path"
        echo "   To recreate: sudo rm -rf $container_path"
        return 0
    end

    echo "=== Creating Ubuntu 24.04 Container ==="
    echo "This takes 2-5 minutes..."
    echo ""

    sudo debootstrap \
        --include=curl,gcc,make,ca-certificates \
        noble \
        $container_path \
        http://archive.ubuntu.com/ubuntu

    if test $status -ne 0
        echo "Failed to create Ubuntu container"
        return 1
    end

    echo "Ubuntu container created at $container_path"
end

function create_fedora_container
    set container_path "/var/lib/machines/$FEDORA_CONTAINER"

    if test -d $container_path
        echo "Fedora container already exists at $container_path"
        echo "   To recreate: sudo rm -rf $container_path"
        return 0
    end

    echo "=== Creating Fedora 41 Container ==="
    echo "Downloading Fedora container image..."
    echo ""

    # Download Fedora container base image
    set fedora_version 41
    set fedora_image "Fedora-Container-Base-Generic-$fedora_version-1.4.x86_64.tar.xz"
    set fedora_url "https://download.fedoraproject.org/pub/fedora/linux/releases/$fedora_version/Container/x86_64/images/$fedora_image"

    if not test -f /tmp/$fedora_image
        echo "Downloading $fedora_url..."
        wget -q --show-progress -O /tmp/$fedora_image "$fedora_url"
        if test $status -ne 0
            echo "Failed to download Fedora image"
            echo "Check if URL is valid: $fedora_url"
            return 1
        end
    end

    echo "Extracting Fedora image..."
    sudo mkdir -p $container_path

    # Fedora container images have a layer structure, extract the layer
    set tmpdir (mktemp -d)
    tar -xf /tmp/$fedora_image -C $tmpdir
    # Find and extract the layer
    set layer (find $tmpdir -name "layer.tar" | head -1)
    if test -n "$layer"
        sudo tar -xf $layer -C $container_path
    else
        # Fallback: try direct extraction
        sudo tar -xf /tmp/$fedora_image -C $container_path --strip-components=1
    end
    rm -rf $tmpdir

    if test $status -ne 0
        echo "Failed to create Fedora container"
        return 1
    end

    # Install additional packages
    echo "Installing build dependencies in Fedora container..."
    sudo systemd-nspawn -D $container_path \
        dnf install -y curl gcc gcc-c++ make ca-certificates

    echo "Fedora container created at $container_path"
end

function create_arch_container
    set container_path "/var/lib/machines/$ARCH_CONTAINER"

    if test -d $container_path
        echo "Arch container already exists at $container_path"
        echo "   To recreate: sudo rm -rf $container_path"
        return 0
    end

    echo "=== Creating Arch Linux Container ==="
    echo "Downloading Arch bootstrap image..."
    echo ""

    # Download Arch bootstrap tarball
    set arch_mirror "https://geo.mirror.pkgbuild.com"
    set arch_image "archlinux-bootstrap-x86_64.tar.zst"
    set arch_url "$arch_mirror/iso/latest/$arch_image"

    if not test -f /tmp/$arch_image
        echo "Downloading $arch_url..."
        wget -q --show-progress -O /tmp/$arch_image "$arch_url"
        if test $status -ne 0
            echo "Failed to download Arch image"
            return 1
        end
    end

    echo "Extracting Arch image..."
    sudo mkdir -p $container_path

    # Arch bootstrap has root.x86_64/ prefix
    sudo tar -xf /tmp/$arch_image -C $container_path --strip-components=1

    if test $status -ne 0
        echo "Failed to extract Arch image"
        return 1
    end

    # Initialize pacman keyring and install base packages
    echo "Initializing Arch container..."

    # Enable a mirror
    echo 'Server = https://geo.mirror.pkgbuild.com/$repo/os/$arch' | sudo tee $container_path/etc/pacman.d/mirrorlist

    sudo systemd-nspawn -D $container_path \
        /bin/bash -c "pacman-key --init && pacman-key --populate archlinux && pacman -Sy --noconfirm curl gcc make ca-certificates"

    if test $status -ne 0
        echo "Failed to initialize Arch container"
        return 1
    end

    echo "Arch container created at $container_path"
end

function show_usage
    echo "Usage: ./create-containers.fish [ubuntu|fedora|arch|all]"
    echo ""
    echo "Creates systemd-nspawn containers for testing r3bl-cmdr installation"
    echo "on multiple Linux distributions."
    echo ""
    echo "Options:"
    echo "  ubuntu  - Create Ubuntu 24.04 (noble) container"
    echo "  fedora  - Create Fedora 41 container"
    echo "  arch    - Create Arch Linux container"
    echo "  all     - Create all containers (default)"
    echo ""
    echo "Container locations:"
    echo "  /var/lib/machines/$UBUNTU_CONTAINER"
    echo "  /var/lib/machines/$FEDORA_CONTAINER"
    echo "  /var/lib/machines/$ARCH_CONTAINER"
end

# Main logic
switch $target
    case ubuntu
        ensure_dependencies
        create_ubuntu_container
    case fedora
        ensure_dependencies
        create_fedora_container
    case arch
        ensure_dependencies
        create_arch_container
    case all
        ensure_dependencies
        create_ubuntu_container
        echo ""
        create_fedora_container
        echo ""
        create_arch_container
    case -h --help help
        show_usage
    case '*'
        echo "Unknown option: $target"
        show_usage
        exit 1
end

echo ""
echo "Next steps:"
echo "  Run tests: ./run-test.fish all"
echo "  Clean up:  ./cleanup.fish"
