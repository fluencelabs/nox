#!/bin/bash

# param
# $1 cluster_name
long_term_dir="long-term-keys"

./contract-combine-nodes.sh $1 \
    $(./master-show-node-keys.sh $long_term_dir/node0) \
    $(./master-show-node-keys.sh $long_term_dir/node1) \
    $(./master-show-node-keys.sh $long_term_dir/node2) \
    $(./master-show-node-keys.sh $long_term_dir/node3)
