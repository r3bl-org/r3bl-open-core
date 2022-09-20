#!/usr/bin/env fish
clear
rm log.txt
touch log.txt
tail -f log.txt | lolcat
