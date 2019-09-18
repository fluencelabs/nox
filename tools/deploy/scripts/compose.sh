#!/bin/bash

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

set -eo pipefail

# include functions from scripts
source ./functions/asserts.sh

# updates all needed containers
function container_update()
{
    printf 'Updating all containers.'
    docker pull ipfs/go-ipfs:latest >/dev/null
    printf '.'
    docker pull fluencelabs/node:$IMAGE_TAG >/dev/null
    printf '.'
    docker pull fluencelabs/worker:$IMAGE_TAG >/dev/null
    printf '.\n'
    echo 'Containers are updated.'
}

# main function to deploy Fluence
function deploy()
{
    echo "Deploying for $CHAIN chain."

    # disables docker-compose warnings about orphan services
    export COMPOSE_IGNORE_ORPHANS=True

    export FLUENCE_STORAGE="$HOME/.fluence/"
    export ETHEREUM_ADDRESS="http://$ETHEREUM_IP:8545"

    # exports initial arguments to global scope for `docker-compose` files
#    export_arguments

#    check_envs

    container_update

    # starting node container
    echo "Removing workers & tendermints"
    docker ps -a | grep -E 'tendermint|worker' | awk '{ print $1 }' | xargs docker rm -f &> /dev/null || true
    echo "Restarting node container"
    docker-compose --compatibility -f node.yml up -d --timeout 30 --force-recreate || true &>/dev/null
    echo "Disconnecting old networks"
    docker network ls | grep fluence | awk '{print $1}' | xargs -I{} docker network disconnect {} fluence-node-1 || true &> /dev/null
    echo "Removing old networks"
    docker network prune -f &> /dev/null

    echo 'Node container is started.'
}

deploy

