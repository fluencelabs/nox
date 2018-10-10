#!/bin/bash
# param
# $1 docker_network_name / cluster_name
# $2 vm_code_directory
# $3 node_index
# $4 host_rpc_port
# $5 long_term_key_location
# $6 cluster_info_json_file

# initialize Tendermint home dir, put public/private keys there
tm_home=$PWD/nodes/$1/node$3
mkdir -p $tm_home
cp -R $5/* $tm_home

# configure genesis and peer discovery
cat $6 | jq -r .persistent_peers > $tm_home/config/persistent_peers.txt
cat $6 | jq .genesis > $tm_home/config/genesis.json

# run Fluence node image with Tendermint and State machine
docker run -idt -p $4:26657 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $tm_home:/tendermint --name $1_node$3 --network $1 fluencelabs/statemachine:latest
