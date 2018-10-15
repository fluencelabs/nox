#!/bin/bash -e
# param
# $1 long_term_key_location

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters: 1 required"
    exit 1
fi

# remove previous configuration files
rm -rf "$1/config"

# generate keys
./master-init-node-keys.sh "$1"
