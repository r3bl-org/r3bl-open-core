#!/usr/bin/env fish
clear

pushd tui
tail -f -s 5 log.txt | lolcat
rm log.txt
touch log.txt
popd
