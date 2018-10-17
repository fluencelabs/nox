#!/bin/bash -e
# param
# $1 docker_network_name / cluster_name
# $2 vm_code_directory
# $3 host_base_rpc_port
# $4 long_term_key_all_locations

# Currently 'host_base_rpc_port' is Tendermint RPC port for 0-th node.
# RPC ports of other nodes are obtained by adding 100, 200, ... to 'host_base_rpc_port'.
# P2P ports of all nodes are obtained as RPC ports minus 1.
# TODO: Fix this logic to choosing ports from predefined ranges during implementing cluster formation via contracts.

if [ "$#" -ne 4 ]; then
    echo "Illegal number of parameters: 4 required"
    exit 1
fi

# remove/kill the previous containers and network
echo "Clearing previous docker containers"
docker kill $(docker ps -a -q -f name="$1_node") 2> /dev/null || true
docker rm $(docker ps -a -q -f name="$1_node") 2> /dev/null || true

# prepare node directories
echo "Preparing node directories"
network_dir=$HOME/.fluence/nodes/$1
for ((i = 0; i <= 3; i++)); do
    mkdir -p "$network_dir/node$i"
    ./master-run-tm-utility.sh tm-reset "$network_dir/node$i"
    rm -rf "$network_dir/node$i/config"
done

# initializing nodes' keys, if not initialized yet
echo "Initializing node keys"
for ((i = 0; i <= 3; i++)); do
    mkdir -p "$4/node$i"
    ./master-init-node-keys.sh "$4/node$i"
done

# combine genesis and persistent peers and put them to a file
echo "Combining cluster genesis and discovery"
./sim-combine-cluster.sh "$1" "$3" "$4" > "$network_dir/cluster_info.json"

# run 4 nodes
echo "Running nodes in docker containers"
for ((i = 0; i <= 3; i++)); do
    p2p_port=$(($3 + $i * 100 - 1))
    rpc_port=$(($3 + $i * 100))
    pmt_port=$(($3 + $i * 100 + 3))
    ./master-run-node.sh "$1" "$2" $i "$4/node$i" "$network_dir/cluster_info.json" $p2p_port $rpc_port $pmt_port
done
