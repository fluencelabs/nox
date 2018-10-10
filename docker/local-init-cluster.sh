#!/bin/bash
# param
# $1 docker_network_name used also as a directory for test cluster

# remove previous cluster node directories
rm -rf nodes/$1/node*

# generate testnet
tendermint testnet --o nodes/$1 --hostname-prefix $1_node

# copy persistent_peers from Tendermint config to a dedicated file
# Tendermint config would be replaced with a predefined config
echo $(cat nodes/$1/node0/config/config.toml | grep persistent_peers | awk '{print $3}') > nodes/$1/node0/config/persistent_peers.txt
echo $(cat nodes/$1/node1/config/config.toml | grep persistent_peers | awk '{print $3}') > nodes/$1/node1/config/persistent_peers.txt
echo $(cat nodes/$1/node2/config/config.toml | grep persistent_peers | awk '{print $3}') > nodes/$1/node2/config/persistent_peers.txt
echo $(cat nodes/$1/node3/config/config.toml | grep persistent_peers | awk '{print $3}') > nodes/$1/node3/config/persistent_peers.txt
