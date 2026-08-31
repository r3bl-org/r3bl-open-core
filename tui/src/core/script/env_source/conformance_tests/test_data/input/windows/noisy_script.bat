@echo off
echo This is loud stdout output that must not pollute env-source stdout
echo This is loud stderr output 1>&2
set "NOISY_VAR=success"
set "LOUD_SETTING=1"
