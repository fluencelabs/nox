#!/bin/bash
# param
# $1 docker_network_name / cluster_name
# $2 vm_code_directory
# $3 docker_network_subnet
# $4 host_rpc_port
# $5 long_term_key_all_locations

# remove/kill the previous containers and network
docker kill $(docker ps -a -q -f name="$1_node")
docker rm $(docker ps -a -q -f name="$1_node")

# prepare node directories
rm -rf "nodes/$1/node"*
mkdir -p "nodes/$1"

# combine genesis and persistent peers and put them to a file
./sim-combine-cluster.sh "$1" "$5" > "nodes/$1/cluster_info.json"

# run 4 nodes
for ((i = 0; i <= 3; i++)); do
    p2p_port=$(($4 + $i * 100 - 1))
    rpc_port=$(($4 + $i * 100))
    ./master-run-node.sh "$1" "$2" $i "$5/node$i" "nodes/$1/cluster_info.json" $p2p_port $rpc_port
done
