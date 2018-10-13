#!/bin/bash -e
pushd .
cd ..
./local-cluster.sh counternet "$PWD/examples/vmcode-counter" 172.25.0.0/16 25057
popd
