#!/bin/bash -e
# param
# $1 cluster_name
# $2 vm_code_directory
# $3 node_index
# $4 long_term_key_location
# $5 cluster_info_json_file
# $6 host_p2p_port
# $7 host_rpc_port
# $8 tm_prometheus_port
# $9 sm_prometheus_port

if [ "$#" -ne 9 ]; then
    echo "Illegal number of parameters: 9 required"
    exit 1
fi

# initialize Tendermint home dir, put public/private keys there
tm_home=$HOME/.fluence/nodes/$1/node$3
mkdir -p "$tm_home/config"
cp -R "$4/config/"* "$tm_home/config"

# copy cluster info with attached node index to config volume
node_info="{\"cluster\":$(cat $5),\"node_index\":\"$3\"}"
echo "$node_info" > "$tm_home/config/node_info.json"

node_name=$1_node$3

# run Fluence solver node image with Tendermint and State machine
docker run -idt \
    --user $(id -u):$(id -g) \
    -p "$6:26656" -p "$7:26657" -p "$8:26660" -p "$9:26661" \
    -v "$PWD/statemachine:/statemachine" -v "$2:/vmcode" -v "$tm_home:/tendermint" \
    --name "$node_name" \
    fluencelabs/solver:latest
