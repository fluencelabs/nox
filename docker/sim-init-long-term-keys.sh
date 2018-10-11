#!/bin/bash -e
# param
# $1 long_term_key_all_locations

if [ "$#" -ne 1 ]; then
    echo "Illegal number of parameters: 1 required"
    exit 1
fi

./master-init-node-keys.sh "$1/node0"
./master-init-node-keys.sh "$1/node1"
./master-init-node-keys.sh "$1/node2"
./master-init-node-keys.sh "$1/node3"
