#!/bin/bash

# param
long_term_dir="long-term-keys"

./master-init-node-keys.sh $long_term_dir/node0
./master-init-node-keys.sh $long_term_dir/node1
./master-init-node-keys.sh $long_term_dir/node2
./master-init-node-keys.sh $long_term_dir/node3
