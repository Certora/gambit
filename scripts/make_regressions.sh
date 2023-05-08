#!/bin/bash

################################################################################
# make_regressions.sh
#
# Overview
# ========
#
# This script iterates through all conf files in `benchmarks/config-jsons`, runs
# Gambit on them, and outputs the results to a subdirectory of
# `resources/regressions` named after the configuration json.
#
# Usage
# -----
#
# This script takes no arguments. It determines paths to all input files (i.e.,
# files in `benchmarks/config-jsons`) and output locations (i.e.,
# `resources/regressions/XXXXX`) relative to this script's parent directory.

SCRIPTS=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" &>/dev/null && pwd)
GAMBIT="$SCRIPTS/.."
CONFIGS="$GAMBIT/benchmarks/config-jsons"
REGRESSIONS="$GAMBIT"/resources/regressions
echo "scripts: $SCRIPTS"
echo "gambit: $GAMBIT"
echo "configs: $CONFIGS"
echo "regressions: $REGRESSIONS"

# Make sure gambit install is up to date!
(
    echo "Installing gambit..."
    cd "$GAMBIT" || {
        echo "Error: couldn't cd $GAMBIT"
        exit 1
    }
    cargo install --path . || {
        echo "Error: couldn't install gambit"
        exit 1
    }
    echo "Gambit installed"
)

[ -e "$REGRESSIONS" ] && {
    echo "Removing old regressions"
    rm -rf "$REGRESSIONS"
}
echo "Making regressions directory at $REGRESSIONS"
mkdir -p "$REGRESSIONS"

echo "Running conf files"
for conf_path in "$CONFIGS"/*; do
    echo
    echo
    printf "\033[1m- Conf path: %s\033[0m\n" "$conf_path"

    conf=$(basename "$conf_path")
    outdir="$REGRESSIONS"/"$conf"

    (
        cd "$GAMBIT" || {
            echo "Error: couldn't cd $GAMBIT"
            exit 1
        }
        printf "  \033[1mRunning:\033[0m %s\n" "gambit mutate --json $conf_path"
        stdout="$(gambit mutate --json "$conf_path")"
        printf "  \033[1mGambit Output:\033[0m '\033[3m%s\033[0m'\n" "$stdout"
        mv gambit_out "$outdir"
    )
    printf "  \033[1mWrote regression:\033[0m %s\n" "$outdir"

done
