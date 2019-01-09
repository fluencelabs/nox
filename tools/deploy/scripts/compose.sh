#!/bin/bash

set -e

DEV='true'

if [ -n "$DEV" ]
then
    export OWNER_ADDRESS=0x00a329c0648769a73afac7f9381e08fb43dbea72
    export PRIVATE_KEY=4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7
    export PARITY_ARGS='--config dev --jsonrpc-apis=all --jsonrpc-hosts=all --jsonrpc-cors="*" --unsafe-expose'
else
    export PARITY_ARGS='--light --chain kovan --jsonrpc-apis=all --jsonrpc-hosts all --jsonrpc-cors "*" --unsafe-expose'

fi

# getting external ip address and docker ip address
case "$(uname -s)" in
   Darwin)
     export DOCKER_IP=host.docker.internal
     export HOST_IP=host.docker.internal
     ;;

   Linux)
     export DOCKER_IP=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}' | grep -Po "[0-9\.]+")
     export HOST_IP=$(ip route get 8.8.8.8 | grep -Po "(?<=src )[0-9\.]+")
     ;;
esac

# running parity and swarm containers
docker-compose -f parity.yml up -d
docker-compose -f swarm.yml up -d

echo 'Parity and Swarm containers are started.'

# waiting that API of parity start working
# todo get rid of all `sleep`
sleep 10

if [ -n "$DEV" ]
then
    export CONTRACT_ADDRESS=$(node deploy-contract.js)
    sleep 1
fi

echo $CONTRACT_ADDRESS

# starting node container
docker-compose -f node.yml up -d --force-recreate

echo 'Node container is started.'

# check all variables exists
echo "HOST_IP="$HOST_IP
echo "OWNER_ADDRESS="$OWNER_ADDRESS
echo "CONTRACT_ADDRESS="$CONTRACT_ADDRESS
echo "PRIVATE_KEY="$PRIVATE_KEY

# get tendermint key from node logs
# todo get this from `status` API by CLI
while [ -z "$TENDERMINT_KEY" ]
do
    sleep 3
    TENDERMINT_KEY=$(docker logs node1 2>&1 | grep PubKey | sed 's/.*value\":\"\([^ ]*\).*/\1/' | sed 's/\"},//g')
done

echo "TENDERMINT_KEY="$TENDERMINT_KEY

# check if node is already registered
# todo build fluence CLI in fly, use cargo from cli directory, or run from target cli directory?
./fluence register $HOST_IP $TENDERMINT_KEY $OWNER_ADDRESS $CONTRACT_ADDRESS -s $PRIVATE_KEY --wait_syncing --base64_tendermint_key
