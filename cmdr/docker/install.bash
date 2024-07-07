#!/bin/bash

apt-get update -y
apt-get upgrade -y
apt-get install -y curl gcc build-essential libssl-dev pkg-config

# More info: https://rust-lang.github.io/rustup/installation/index.html
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y

. "$HOME/.cargo/env"
cargo install r3bl-cmdr

edi --version
