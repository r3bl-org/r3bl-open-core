#!/usr/bin/env fish

# Run install.bash in systemd-nspawn containers to test r3bl-cmdr installation
#
# Usage: ./run-test.fish [ubuntu|fedora|arch|all]
#
# Options:
#   --ephemeral  Discard changes after test (clean-room test)
#   --shell      Open interactive shell instead of running install

set -l options 'ephemeral' 'shell'
argparse $options -- $argv
or exit 1

set target $argv[1]
if test -z "$target"
    set target "all"
end

# Container names (must match create-containers.fish)
set UBUNTU_CONTAINER "cmdr-ubuntu"
set FEDORA_CONTAINER "cmdr-fedora"
set ARCH_CONTAINER "cmdr-arch"

# Get the directory containing this script
set script_dir (dirname (realpath (status filename)))

function check_container_exists
    set distro $argv[1]
    set container_path "/var/lib/machines/$distro"

    if not test -d $container_path
        echo "Container not found: $container_path"
        echo "Create it first: ./create-containers.fish"
        return 1
    end
    return 0
end

function run_in_container
    set container_name $argv[1]
    set container_path "/var/lib/machines/$container_name"

    if not check_container_exists $container_name
        return 1
    end

    # Build nspawn command
    set nspawn_cmd sudo systemd-nspawn -D $container_path

    # Add ephemeral flag if requested (changes discarded after exit)
    if set -q _flag_ephemeral
        set nspawn_cmd $nspawn_cmd --ephemeral
        echo "=== Testing $container_name (ephemeral - changes will be discarded) ==="
    else
        echo "=== Testing $container_name ==="
    end

    # Bind mount the script directory
    set nspawn_cmd $nspawn_cmd --bind-ro=$script_dir:/app

    if set -q _flag_shell
        # Interactive shell
        echo "Entering $container_name container..."
        echo "Install script available at: /app/install.bash"
        echo ""
        $nspawn_cmd /bin/bash
    else
        # Run the install script
        echo "Running install.bash..."
        echo ""

        $nspawn_cmd /bin/bash /app/install.bash
        set exit_status $status

        echo ""
        if test $exit_status -eq 0
            echo "$container_name: PASSED"
        else
            echo "$container_name: FAILED (exit code: $exit_status)"
        end

        return $exit_status
    end
end

function run_all_tests
    set all_passed true
    set -l results

    for container in $UBUNTU_CONTAINER $FEDORA_CONTAINER $ARCH_CONTAINER
        echo ""
        echo (string repeat -n 60 "=")

        if not check_container_exists $container
            echo "Skipping $container (not found)"
            set results $results "$container: SKIPPED"
            continue
        end

        run_in_container $container
        if test $status -eq 0
            set results $results "$container: PASSED"
        else
            set results $results "$container: FAILED"
            set all_passed false
        end
    end

    echo ""
    echo (string repeat -n 60 "=")
    echo "Summary"
    echo (string repeat -n 60 "=")
    for result in $results
        echo "  $result"
    end

    if test "$all_passed" = true
        echo ""
        echo "All tests passed!"
        return 0
    else
        echo ""
        echo "Some tests failed!"
        return 1
    end
end

function show_usage
    echo "Usage: ./run-test.fish [options] [ubuntu|fedora|arch|all]"
    echo ""
    echo "Runs install.bash in systemd-nspawn containers to verify"
    echo "r3bl-cmdr installation works on multiple distributions."
    echo ""
    echo "Distros:"
    echo "  ubuntu  - Test on Ubuntu 24.04"
    echo "  fedora  - Test on Fedora 41"
    echo "  arch    - Test on Arch Linux"
    echo "  all     - Test on all distros (default)"
    echo ""
    echo "Options:"
    echo "  --ephemeral  Discard changes after test (clean-room)"
    echo "  --shell      Open interactive shell instead of running test"
    echo ""
    echo "Examples:"
    echo "  ./run-test.fish ubuntu              # Test on Ubuntu (persistent)"
    echo "  ./run-test.fish --ephemeral all     # Test all, discard changes"
    echo "  ./run-test.fish --shell fedora      # Debug in Fedora shell"
end

# Main logic
switch $target
    case ubuntu
        run_in_container $UBUNTU_CONTAINER
    case fedora
        run_in_container $FEDORA_CONTAINER
    case arch
        run_in_container $ARCH_CONTAINER
    case all
        if set -q _flag_shell
            echo "Cannot use --shell with 'all'. Specify a single distro."
            exit 1
        end
        run_all_tests
    case -h --help help
        show_usage
    case '*'
        echo "Unknown option: $target"
        show_usage
        exit 1
end
