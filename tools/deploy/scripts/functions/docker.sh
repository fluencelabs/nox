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

# updates all needed containers
function pull_new_images()
{
    printf 'Updating containers '
    docker pull ipfs/go-ipfs:latest >/dev/null
    printf '.'
    docker pull fluencelabs/node:$IMAGE_TAG >/dev/null
    printf '.'
    docker pull fluencelabs/worker:$IMAGE_TAG >/dev/null
    printf '.\n'
}

# removes all containers with tendermint or worker in their names
function remove_old_containers()
{
    echo "Removing workers & tendermints"
    docker ps -a | grep -E 'tendermint|worker' | awk '{ print $1 }' | xargs docker rm -f &> /dev/null || true
}

# restarts fluence node; node.yml assumes some environment variables are set
function restart_node()
{
    # disables docker-compose warnings about orphan services
    export COMPOSE_IGNORE_ORPHANS=True
    echo "Restarting node container"
    docker-compose --compatibility -f node.yml up -d --timeout 30 --force-recreate || true &>/dev/null
}

# disconnects containers from their networks and prunes all dangling networks
function clean_networks()
{
    echo "Cleaning networks"
    docker network ls | grep -E 'fluence|tendermint' | awk '{print $1}' | xargs -I{} docker network disconnect {} fluence-node-1 &> /dev/null || true
    docker network prune -f &> /dev/null
}
