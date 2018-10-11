#!/bin/bash -e
pushd .
cd ..
./sim-cluster.sh sqldbnet "$PWD/examples/vmcode-sqldb" 27057 long-term-keys
popd
