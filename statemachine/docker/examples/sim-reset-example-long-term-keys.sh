#!/bin/bash -e
pushd .
cd ..
./sim-reset-long-term-keys.sh $HOME/.fluence/long-term-keys
popd
