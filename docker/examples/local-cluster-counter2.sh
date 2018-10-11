#!/bin/bash
pushd .
cd ..
./local-cluster.sh counter2net $PWD/examples/vmcode-counter2 172.26.0.0/16 26057
popd
