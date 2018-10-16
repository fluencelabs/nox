#!/bin/bash -e
pushd .
cd ..
./sim-cluster.sh counternet "$PWD/examples/vmcode-counter" 25057 $HOME/.fluence/long-term-keys
popd
docker attach counternet_node3
