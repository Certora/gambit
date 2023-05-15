#!/usr/bash

################################################################################
# remove_sourceroots.sh
#
# remove sourceroot fields from JSON since these are absolute paths

if [[ "$(uname)" == "Linux" ]]; then
    sed -i '/"sourceroot":/d' "$1"
elif [[ "$(uname)" == "Darwin" ]]; then
    sed -i "" '/"sourceroot":/d' "$1"
else
    echo "Unknown operating system: using the GNU sed interface"
    sed -i '/"sourceroot":/d' "$1"
fi
