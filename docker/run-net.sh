#!/bin/bash
docker kill $(docker ps -a -q)
docker rm $(docker ps -a -q)
docker network rm mynet
docker network create -d bridge --subnet 172.25.0.0/16 mynet
docker run -idt -p 26057:26657 -p 26060:26660 -v $PWD/statemachine:/statemachine -v $PWD/mytestnet/node0:/tendermint --name node0 --network mynet statemachine/statemachine:latest
docker run -idt -p 26157:26657 -p 26160:26660 -v $PWD/statemachine:/statemachine -v $PWD/mytestnet/node1:/tendermint --name node1 --network mynet statemachine/statemachine:latest
docker run -idt -p 26257:26657 -p 26260:26660 -v $PWD/statemachine:/statemachine -v $PWD/mytestnet/node2:/tendermint --name node2 --network mynet statemachine/statemachine:latest
docker run -idt -p 26357:26657 -p 26360:26660 -v $PWD/statemachine:/statemachine -v $PWD/mytestnet/node3:/tendermint --name node3 --network mynet statemachine/statemachine:latest
