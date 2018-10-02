#!/bin/bash
docker kill $(docker ps -a -q)
docker rm $(docker ps -a -q)
docker network rm $1

./init-network.sh $1

docker network create -d bridge --subnet $3 $1
#docker run -idt -p 26057:26657 -p 26060:26660 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node0:/tendermint --name $1_node0 --network $1 statemachine/statemachine:latest
#docker run -idt -p 26157:26657 -p 26160:26660 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node1:/tendermint --name $1_node1 --network $1 statemachine/statemachine:latest
#docker run -idt -p 26257:26657 -p 26260:26660 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node2:/tendermint --name $1_node2 --network $1 statemachine/statemachine:latest
#docker run -idt -p 26357:26657 -p 26360:26660 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node3:/tendermint --name $1_node3 --network $1 statemachine/statemachine:latest

docker run -idt -p $4:26657 -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node0:/tendermint --name $1_node0 --network $1 statemachine/statemachine:latest
docker run -idt -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node1:/tendermint --name $1_node1 --network $1 statemachine/statemachine:latest
docker run -idt -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node2:/tendermint --name $1_node2 --network $1 statemachine/statemachine:latest
docker run -idt -v $PWD/statemachine:/statemachine -v $2:/vmcode -v $PWD/$1/node3:/tendermint --name $1_node3 --network $1 statemachine/statemachine:latest
