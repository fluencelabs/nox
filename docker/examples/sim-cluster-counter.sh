#!/bin/bash -e
pushd .
cd ..
./sim-cluster.sh counternet "$PWD/examples/vmcode-counter" 25057 long-term-keys
popd
