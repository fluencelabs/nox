#!/bin/bash
cd ..
tendermint init --home=$PWD/nodes/single_node
echo "" > $PWD/nodes/single_node/config/persistent_peers.txt
docker run -idt -p 26657:26657 -p 26660:26660 -v $PWD/statemachine:/statemachine -v $PWD/examples/vmcode-counter:/vmcode -v $PWD/nodes/single_node:/tendermint fluencelabs/statemachine:latest
cd examples
