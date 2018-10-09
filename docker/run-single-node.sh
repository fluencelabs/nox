#!/bin/bash
tendermint init --home=$PWD/single_node
echo "" > $PWD/single_node/config/persistent_peers.txt
docker run -idt -p 26657:26657 -p 26660:26660 -v $PWD/statemachine:/statemachine -v $PWD/vmcode-counter:/vmcode -v $PWD/single_node:/tendermint fluencelabs/statemachine:latest
