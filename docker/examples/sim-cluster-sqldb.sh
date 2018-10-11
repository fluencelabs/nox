#!/bin/bash
pushd .
cd ..
./sim-cluster.sh sqldbnet $PWD/examples/vmcode-sqldb 172.27.0.0/16 27057 long-term-keys
popd
