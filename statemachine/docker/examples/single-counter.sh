#!/bin/bash -e
pushd .
cd ..
tm_home=$HOME/.fluence/nodes/single_node
tendermint init --home="$tm_home"
echo "" > "$tm_home/config/persistent_peers.txt"
docker run -idt -p 26657:26657 -p 26660:26660 -v "$PWD/statemachine":/statemachine -v "$PWD/examples/vmcode-counter":/vmcode -v "$tm_home":/tendermint fluencelabs/solver:latest
popd
