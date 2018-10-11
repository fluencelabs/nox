#!/bin/bash
# param
# $1 docker_network_name / cluster_name
# $2 vm_code_directory
# $3 docker_network_subnet
# $4 host_rpc_port
# $5 long_term_key_all_locations

# remove/kill the previous containers and network
docker kill $(docker ps -a -q -f name=$1_node)
docker rm $(docker ps -a -q -f name=$1_node)
docker network rm $1

# create Docker network
docker network create -d bridge --subnet $3 $1

# prepare node directories
rm -rf nodes/$1/node*
mkdir -p nodes/$1

# combine genesis and persistent peers and put them to a file
./sim-combine-cluster.sh $1 $5 > nodes/$1/cluster_info.json

# run 4 nodes
./master-run-node.sh $1 $2 0 $5/node0 nodes/$1/cluster_info.json $(($4+  0-1)) $(($4+  0))
./master-run-node.sh $1 $2 1 $5/node1 nodes/$1/cluster_info.json $(($4+100-1)) $(($4+100))
./master-run-node.sh $1 $2 2 $5/node2 nodes/$1/cluster_info.json $(($4+200-1)) $(($4+200))
./master-run-node.sh $1 $2 3 $5/node3 nodes/$1/cluster_info.json $(($4+300-1)) $(($4+300))
