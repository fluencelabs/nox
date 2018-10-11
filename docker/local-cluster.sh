#!/bin/bash
# param
# $1 docker_network_name
# $2 vm_code_directory
# $3 docker_network_subnet
# $4 host_rpc_port

if [ "$#" -ne 4 ]; then
    echo "Illegal number of parameters: 4 required"
    exit 1
fi

# remove/kill the previous containers and network
docker kill $(docker ps -a -q -f name="$1_node")
docker rm $(docker ps -a -q -f name="$1_node")
docker network rm "$1"

# init Tendermint cluster
./local-init-cluster.sh "$1"

# create Docker network
docker network create -d bridge --subnet "$3" "$1"

# run 4 containers
# the only communication point is Tendermint RPC on node0
docker run -idt -p $4:26657 -v "$PWD/statemachine":/statemachine -v "$2":/vmcode -v "$PWD/nodes/$1/node0":/tendermint --name "$1_node0" --network "$1" fluencelabs/statemachine:latest
docker run -idt             -v "$PWD/statemachine":/statemachine -v "$2":/vmcode -v "$PWD/nodes/$1/node1":/tendermint --name "$1_node1" --network "$1" fluencelabs/statemachine:latest
docker run -idt             -v "$PWD/statemachine":/statemachine -v "$2":/vmcode -v "$PWD/nodes/$1/node2":/tendermint --name "$1_node2" --network "$1" fluencelabs/statemachine:latest
docker run -idt             -v "$PWD/statemachine":/statemachine -v "$2":/vmcode -v "$PWD/nodes/$1/node3":/tendermint --name "$1_node3" --network "$1" fluencelabs/statemachine:latest
