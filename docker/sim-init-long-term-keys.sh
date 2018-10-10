#!/bin/bash
# param
# $1 long_term_key_all_locations

./master-init-node-keys.sh $1/node0
./master-init-node-keys.sh $1/node1
./master-init-node-keys.sh $1/node2
./master-init-node-keys.sh $1/node3
