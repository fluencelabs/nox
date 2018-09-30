#!/bin/bash
tendermint node --home=$1 --consensus.create_empty_blocks=false > /dev/null 2> /dev/null &
java -Dconfig.file=/container_data/sm.config -jar $2