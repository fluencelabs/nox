#!/bin/bash
# param
# $1 docker_network_name / cluster_name
# $2 vm_code_directory
# $3 docker_network_subnet
# $4 host_rpc_port
# $5 long_term_key_all_locations

long_term_dir="long-term-keys"

# remove/kill the previous containers and network
docker kill $(docker ps -a -q -f name=$1_node)
docker rm $(docker ps -a -q -f name=$1_node)
docker network rm $1

# create Docker network
docker network create -d bridge --subnet $3 $1

rm -rf nodes/$1/node*
mkdir -p nodes/$1
./sim-combine-cluster.sh $1 $5 > nodes/$1/cluster_info.json

# run 4 nodes
./master-run-node.sh $1 $2 0 $(($4+  0)) $long_term_dir/node0 nodes/$1/cluster_info.json
./master-run-node.sh $1 $2 1 $(($4+100)) $long_term_dir/node1 nodes/$1/cluster_info.json
./master-run-node.sh $1 $2 2 $(($4+200)) $long_term_dir/node2 nodes/$1/cluster_info.json
./master-run-node.sh $1 $2 3 $(($4+300)) $long_term_dir/node3 nodes/$1/cluster_info.json
