#!/bin/bash -e
pushd .
cd ..
./sim-cluster.sh llamadbnet "$PWD/examples/vmcode-llamadb" 29057 $HOME/.fluence/long-term-keys
popd
docker attach llamadbnet_node3
