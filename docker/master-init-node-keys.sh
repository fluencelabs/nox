#!/bin/bash -e
# param
# $1 long_term_key_location

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters: 1 required"
    exit 1
fi

# generate keys
./master-run-tm-utility.sh tm-init "$1"

# remove unused data: we only need public/private key files
rm -rf "$1/config/config.toml"
rm -rf "$1/config/genesis.json"
rm -rf "$1/data"
