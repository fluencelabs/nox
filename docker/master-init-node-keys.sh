#!/bin/bash -e
# param
# $1 long_term_key_location

# remove previous configuration files
rm -rf "$1/config"

# generate keys
tendermint init --home="$1"

# remove unused data: we only need public/private key files
rm "$1/config/config.toml"
rm "$1/config/genesis.json"
rm -rf "$1/data"
