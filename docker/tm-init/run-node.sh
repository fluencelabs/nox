#!/bin/bash -e

tendermint init --home="/tendermint"
rm -rf "/tendermint/config/config.toml"
rm -rf "/tendermint/config/genesis.json"
rm -rf "/tendermint/data"
