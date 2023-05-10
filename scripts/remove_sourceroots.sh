#!/usr/bash

################################################################################
# remove_sourceroots.sh
#
# remove sourceroot fields from JSON since these are absolute paths

sed -i "" '/"sourceroot":/d' "$1"
