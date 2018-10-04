#!/bin/bash

# uncomment line below if attaching to container is needed before node running
#sleep 8

# copy predefined Tendermint config
cp -f $2/config.toml $1/config/config.toml

# run Tendermint with disabled output
tendermint node --home=$1 --moniker=$HOSTNAME --p2p.persistent_peers=$(cat $1/config/persistent_peers.txt) > /dev/null 2> /dev/null &

# run State machine
java -Dconfig.file=$2/sm.config -jar $3
