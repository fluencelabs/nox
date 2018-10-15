#!/bin/bash -e
# param
# $1 long_term_key_all_locations

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters: 1 required"
    exit 1
fi

./master-reset-node-keys.sh "$1/node0"
./master-reset-node-keys.sh "$1/node1"
./master-reset-node-keys.sh "$1/node2"
./master-reset-node-keys.sh "$1/node3"
