#!/bin/bash
tendermint unsafe_reset_all --home=$1
tendermint init --home=$1
tendermint node --home=$1 --consensus.create_empty_blocks=false > /dev/null 2> /dev/null &
java -Dconfig.file=$2/sm.config -jar $3
