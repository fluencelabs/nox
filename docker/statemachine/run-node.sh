#!/bin/bash
cp -f $2/config.toml $1/config/config.toml
tendermint node --home=$1 --moniker=$HOSTNAME --p2p.persistent_peers=$(cat $1/config/persistent_peers.txt) > /dev/null 2> /dev/null &
java -Dconfig.file=$2/sm.config -jar $3
