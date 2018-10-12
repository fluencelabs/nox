#!/bin/bash
# param
# $1 cluster_name
# $2 long_term_key_all_locations

if [ "$#" -ne 2 ]; then
    echo "Illegal number of parameters: 2 required"
    exit 1
fi

./contract-combine-nodes.sh "$1" \
    $(./master-show-node-keys.sh "$2/node0") \
    $(./master-show-node-keys.sh "$2/node1") \
    $(./master-show-node-keys.sh "$2/node2") \
    $(./master-show-node-keys.sh "$2/node3")
