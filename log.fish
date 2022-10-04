#!/usr/bin/env fish
clear
pushd tui
rm log.txt
touch log.txt
tail -f log.txt | lolcat
popd
