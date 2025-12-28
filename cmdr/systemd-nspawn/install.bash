#!/bin/bash

#
#   Copyright (c) 2025 R3BL LLC
#   All rights reserved.
#
#   Licensed under the Apache License, Version 2.0 (the "License");
#   you may not use this file except in compliance with the License.
#   You may obtain a copy of the License at
#
#   http://www.apache.org/licenses/LICENSE-2.0
#
#   Unless required by applicable law or agreed to in writing, software
#   distributed under the License is distributed on an "AS IS" BASIS,
#   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
#   See the License for the specific language governing permissions and
#   limitations under the License.
#

# Detect package manager and install dependencies
if command -v apt-get &> /dev/null; then
    # Debian/Ubuntu
    apt-get update -y
    apt-get upgrade -y
    apt-get install -y curl gcc build-essential
elif command -v dnf &> /dev/null; then
    # Fedora/RHEL
    dnf update -y
    dnf install -y curl gcc gcc-c++ make
elif command -v pacman &> /dev/null; then
    # Arch Linux
    pacman -Syu --noconfirm
    pacman -S --noconfirm curl gcc make
elif command -v zypper &> /dev/null; then
    # openSUSE
    zypper refresh
    zypper update -y
    zypper install -y curl gcc gcc-c++ make
else
    echo "Unsupported package manager. Please install curl, gcc, and make manually."
    exit 1
fi

# More info: https://rust-lang.github.io/rustup/installation/index.html
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

. "$HOME/.cargo/env"
cargo install r3bl-cmdr

edi --version
giti --version
rc --version
