#!/bin/bash -e

tendermint unsafe_reset_all --home="/tendermint"
rm -rf /tendermint/config
