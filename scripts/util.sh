#!/usr/bin/bash

# Utilities for regression testing

green_check="$(printf "[\033[32;1m ✔ \033[0m]")"
yellow_elipses="$(printf "[\033[33;1m...\033[0m]")"
red_x="$(printf "[\033[31;1m ✘ \033[0m]")"

export green_check
export yellow_elipses
export red_x
