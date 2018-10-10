#!/bin/bash
# param
# $1 long_term_key_location

# remove previous cluster directory
rm -rf $1/config

tendermint init --home=$1
rm $1/config/config.toml
rm $1/config/genesis.json
rm -rf $1/data
