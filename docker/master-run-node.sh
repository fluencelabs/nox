#!/bin/bash
# param
# $1 cluster_name
# $2 vm_code_directory
# $3 node_index
# $4 long_term_key_location
# $5 cluster_info_json_file
# $6 host_p2p_port
# $7 host_rpc_port

# initialize Tendermint home dir, put public/private keys there
tm_home=$PWD/nodes/$1/node$3
mkdir -p $tm_home
cp -R $4/* $tm_home

# configure genesis and peer discovery
cat $5 | jq .genesis > $tm_home/config/genesis.json
cat $5 | jq -r .persistent_peers > $tm_home/config/persistent_peers.txt
cat $5 | jq -r ".external_addrs|.[$3]" > $tm_home/config/external_addr.txt

# run Fluence node image with Tendermint and State machine
docker run -idt \
    -p $6:26656 -p $7:26657 \
    -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $tm_home:/tendermint \
    --name $1_node$3 \
    --network $1 \
    fluencelabs/statemachine:latest
