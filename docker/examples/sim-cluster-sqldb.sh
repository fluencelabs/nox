#!/bin/bash -e
pushd .
cd ..
./sim-cluster.sh sqldbnet "$PWD/examples/vmcode-sqldb" 27057 $HOME/.fluence/long-term-keys
popd
