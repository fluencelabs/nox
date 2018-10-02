#!/bin/bash
#tendermint unsafe_reset_all --home=$1
#cp $2/config.toml $1/config/config.toml
#tendermint init --home=$1
tendermint node --home=$1 --moniker=$HOSTNAME > /dev/null 2> /dev/null &
java -Dconfig.file=$2/sm.config -jar $3
