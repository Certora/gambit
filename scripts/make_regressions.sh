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
green_check=""
red_x=""
# shellcheck disable=SC1091
source "$SCRIPTS/util.sh"
GAMBIT="$SCRIPTS/.."
GAMBIT_EXECUTABLE="$GAMBIT/target/release/gambit"
CONFIGS="$GAMBIT/benchmarks/config-jsons"
REGRESSIONS="$GAMBIT"/resources/regressions
TMP_REGRESSIONS="$GAMBIT"/resources/tmp_regressions
EXPECTED_SOLC_VERSION_NUM="8.13"

if [ -z ${SOLC+x} ]; then
    SOLC="solc$EXPECTED_SOLC_VERSION_NUM"
fi

NUM_CONFIGS=$(ls "$CONFIGS" | wc -l | xargs)

print_vars() {
    echo "scripts: $SCRIPTS"
    echo "gambit: $GAMBIT"
    echo "configs: $CONFIGS"
    echo "regressions: $REGRESSIONS"
    echo "temporary regressions: $TMP_REGRESSIONS"
}

double_check_make_regressions() {
    printf "\033[33m[!!!] WARNING!\033[0m You are about to remake all regression tests!!\n"
    printf "      \033[41;37;1;3mThis will overwrite \`\033[0;41;37;1mresources/regressions\033[0;41;37;1;3m\`!!\033[0m\n"
    printf "      (\033[1mNote:\033[0m regressions are tracked by Git, so you can recover to a previous state)\n"
    while true; do
        printf "Do you wish to proceed? [Y/n] "
        read -n 1 user_response
        echo

        case $user_response in
        y | Y)
            printf "Continuing with make_regressions.sh...\n"
            return 0
            ;;
        n | N)
            printf "Exiting without continuing\n"
            exit 0
            ;;
        *)
            printf "Unrecognized response: '%s'\n" $user_response
            ;;
        esac
    done
}

check_solc_version() {
    if ! $SOLC --version | grep "0.""$EXPECTED_SOLC_VERSION_NUM" >/dev/null; then
        echo "Expected solc version 0.$EXPECTED_SOLC_VERSION_NUM"
        exit 1
    fi
}

build_release() {
    old_dir=$(pwd)
    cd "$GAMBIT" || exit 1
    cargo build --release

    cd "$old_dir" || exit 1
}

clean_state() {
    [ -e "$TMP_REGRESSIONS" ] && {
        echo "Removing temporary regressions directory $TMP_REGRESSIONS"
        rm -rf "$TMP_REGRESSIONS"
    }
}

setup() {
    echo "Making temporary regressions directory at $TMP_REGRESSIONS"
    mkdir -p "$TMP_REGRESSIONS"
}

make_regressions() {
    echo "Running on $NUM_CONFIGS configurations"
    starting_dir=$(pwd)
    failed_confs=()
    conf_idx=0
    failed=false
    for conf_path in "$CONFIGS"/*; do
        conf_idx=$((conf_idx + 1))
        echo
        echo
        printf "\033[1mConfiguration %s/%s:\033[0m %s\n" "$conf_idx" "$NUM_CONFIGS" "$(basename "$conf_path")"

        conf=$(basename "$conf_path")
        outdir="$TMP_REGRESSIONS"/"$conf"

        cd "$GAMBIT" || {
            echo "Error: couldn't cd $GAMBIT"
            exit 1
        }
        printf "  %s \033[1mRunning:\033[0m %s\n" "$green_check" "gambit mutate --json $conf_path --solc $SOLC"
        stdout="$("$GAMBIT_EXECUTABLE" mutate --json "$conf_path" --solc "$SOLC")"
        printf "  %s \033[1mGambit Output:\033[0m '\033[3m%s\033[0m'\n" "$green_check" "$stdout"
        exit_code=$?
        if [ $exit_code -ne 0 ]; then
            printf "  %s Failed to run config %s\n" "$red_x" "$(basename "$conf_path")"
            failed=true
            failed_confs+=("$conf_path")
        else
            printf "  %s \033[1mMoving Outdir:\033[0m to %s\n" "$green_check" "$outdir"
            mv gambit_out "$outdir"
            printf "  %s Successfully created regression test case for %s\n" "$green_check" "$(basename "$conf_path")"
        fi
        cd "$starting_dir" || exit 1

    done

}

summary() {
    printf "\n\n\033[1m                         SUMMARY OF make_regressions.sh\n\n\033[0m"

    if $failed; then

        printf "%s \033[31;1m%s/%s configurations failed to run:\033[0m\n" "$red_x" "${#failed_confs[@]}" "$NUM_CONFIGS"
        idx=0
        for conf in "${failed_confs[@]}"; do
            idx=$((idx + 1))
            echo "   ($idx) $conf"
        done

        printf "\n\nRegression tests were not updated\n"
        printf "Temporary regression tests were cleaned up"
        clean_state
        exit 101
    else
        printf "%s \033[32;1m All %s configurations ran successfully\033[0m\n" "$green_check" "$NUM_CONFIGS"
        [ -e "$REGRESSIONS" ] && {
            rm -rf "$REGRESSIONS"
            printf "%s Removed old regressions\n" "$green_check"
        }
        mv "$TMP_REGRESSIONS" "$REGRESSIONS"
        printf "%s Moved Temporary regessions to regressions location\n" "$green_check"
        printf "     %s -> %s\n" "$TMP_REGRESSIONS" "$REGRESSIONS"
        printf "%s \033[1;3mRegression tests successfully updated\033[0m\n" "$green_check"
        clean_state
    fi
}

double_check_make_regressions
print_vars
check_solc_version
build_release
clean_state
setup
make_regressions
summary
