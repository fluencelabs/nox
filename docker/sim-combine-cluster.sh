#!/bin/bash
# param
# $1 cluster_name
# $2 long_term_key_all_locations

./contract-combine-nodes.sh "$1" \
    $(./master-show-node-keys.sh "$2/node0") \
    $(./master-show-node-keys.sh "$2/node1") \
    $(./master-show-node-keys.sh "$2/node2") \
    $(./master-show-node-keys.sh "$2/node3")
