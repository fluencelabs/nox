# Copyright 2018 Fluence Labs Limited
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

#!/bin/bash

set -e

# The script uses for deploying Parity, Swarm, and Fluence containers.
# If `PROD_DEPLOY` is set in env, the script will also expect the following env variables: `NAME`, `PORTS`, `OWNER_ADDRESS`, `PRIVATE_KEY`
# Without `PROD_DEPLOY` exported flag the script will use default arguments
# If first arg is `multiple`, script will start 4 fluence node along with Swarm & Parity nodes

# `PROD_DEPLOY` variable is assigned in `fabfile.py`, so if run `compose.sh` directly,
#  the network will be started in development mode locally
if [ -z "$PROD_DEPLOY" ]; then
    echo "Deploying locally with default arguments."
    export NAME='node1'
    # open 10 ports, so it's possible to create 10 workers
    export PORTS='25000:25010'
    # eth address in `dev` mode Parity with eth
    export OWNER_ADDRESS=0x00a329c0648769a73afac7f9381e08fb43dbea72
    export PRIVATE_KEY=4d5db4107d237df6a3d58ee5f70ae63d73d7658d4026f2eefd2f204c81682cb7
    export PARITY_ARGS='--config dev-insecure --jsonrpc-apis=all --jsonrpc-hosts=all --jsonrpc-cors="*" --unsafe-expose'
else
    echo "Deploying for $CHAIN chain."
    export PARITY_ARGS='--light --chain '$CHAIN' --jsonrpc-apis=all --jsonrpc-hosts=all --jsonrpc-cors="*" --unsafe-expose'

fi

# getting docker ip address
case "$(uname -s)" in
   Darwin)
     export DOCKER_IP=host.docker.internal
     ;;

   Linux)
     export DOCKER_IP=$(ifconfig docker0 | grep 'inet ' | awk '{print $2}' | grep -Po "[0-9\.]+")
     ;;
esac

# use exported external ip address or get it from OS
# todo rewrite this
if [ -z "$PROD_DEPLOY" ]; then
    EXTERNAL_HOST_IP="127.0.0.1"
    case "$(uname -s)" in
       Darwin)
         export HOST_IP=host.docker.internal
         ;;

       Linux)
         export HOST_IP=$(ip route get 8.8.8.8 | grep -Po "(?<=src )[0-9\.]+")
         ;;
    esac
else
    EXTERNAL_HOST_IP=$HOST_IP
fi

# running parity and swarm containers
docker-compose -f parity.yml up -d
docker-compose -f swarm.yml up -d

echo 'Parity and Swarm containers are started.'

# waiting that API of parity start working
# todo get rid of all `sleep`
sleep 30

# deploy contract if there is new dev ethereum node
if [ -z "$PROD_DEPLOY" ]; then
    if [ ! -d "node_modules" ]; then
        npm install
    fi
    RESULT=$(npm run deploy)
    # get last word from script output
    export CONTRACT_ADDRESS=`echo ${RESULT} | awk '{print $NF}'`
    sleep 1
fi

# check all variables exists
echo "CONTRACT_ADDRESS="$CONTRACT_ADDRESS
echo "NAME="$NAME
echo "PORTS="$PORTS
echo "HOST_IP="$HOST_IP
echo "EXTERNAL_HOST_IP="$EXTERNAL_HOST_IP
echo "OWNER_ADDRESS="$OWNER_ADDRESS
echo "CONTRACT_ADDRESS="$CONTRACT_ADDRESS
echo "PRIVATE_KEY="$PRIVATE_KEY

START_PORT=${PORTS%:*}
LAST_PORT=${PORTS#*:}
export STATUS_PORT=$((LAST_PORT+400))

# port for status API
echo "STATUS_PORT="$STATUS_PORT

COUNTER=1

# starting node container
# if there was `multiple` flag on the running script, will be created 4 nodes, otherwise one node
if [ "$1" = "multiple" ]; then
    docker-compose -f multiple-node.yml up -d --force-recreate
    NUMBER_OF_NODES=4
else
    docker-compose -f node.yml up -d --force-recreate
    NUMBER_OF_NODES=1
fi

echo 'Node container is started.'

while [ $COUNTER -le $NUMBER_OF_NODES ]; do
        # get tendermint key from node logs
        # todo get this from `status` API by CLI
        while [ -z "$TENDERMINT_KEY" ]; do
            # TODO: parse for 'Node ID' instead of 'PubKey'
            TENDERMINT_KEY=$(docker logs node$COUNTER 2>&1 | awk 'match($0, /PubKey: /) { print substr($0, RSTART + RLENGTH) }')
            sleep 3
        done

    echo "CURRENT NODE = "$COUNTER
    echo "TENDERMINT_KEY="$TENDERMINT_KEY

    # use hardcoded ports for multiple nodes
    if [ "$1" = "multiple" ]; then
        START_PORT="25"$COUNTER"00"
        LAST_PORT="25"$COUNTER"10"
    fi

    echo "START_PORT="$START_PORT
    echo "LAST_PORT="$LAST_PORT

    # check if node is already registered
    # todo build fluence CLI in fly, use cargo from cli directory, or run from target cli directory?
    ./fluence register $EXTERNAL_HOST_IP $TENDERMINT_KEY $CONTRACT_ADDRESS $OWNER_ADDRESS -s $PRIVATE_KEY --wait_syncing --start_port $START_PORT --last_port $LAST_PORT --base64_tendermint_key

    COUNTER=$[$COUNTER+1]
    TENDERMINT_KEY=""
done
