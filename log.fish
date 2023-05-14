#!/usr/bin/env fish
clear

pushd tui
# tail -f -s 5 log.txt | lolcat
tail -f -s 5 log.txt
rm log.txt
touch log.txt
popd
