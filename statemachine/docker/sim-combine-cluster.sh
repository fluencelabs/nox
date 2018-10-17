#!/bin/bash
# param
# $1 cluster_name
# $2 host_base_rpc_port
# $3 long_term_key_all_locations

if [ "$#" -ne 3 ]; then
    echo "Illegal number of parameters: 3 required"
    exit 1
fi

./contract-combine-nodes.sh "$1" "$2" \
    $(./master-show-node-keys.sh "$3/node0") \
    $(./master-show-node-keys.sh "$3/node1") \
    $(./master-show-node-keys.sh "$3/node2") \
    $(./master-show-node-keys.sh "$3/node3")
