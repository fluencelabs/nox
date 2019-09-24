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

###
# This script is an entrypoint of the Fluence Node docker container.
# It first checks that $EXTERNAL_IP is defined,
# if /var/run/docker.sock (or other $DOCKER_HOST) is passed as a volume,
# and run whatever command is passed as arguments (usually it's CMD from Dockerfile).
###

# set to fail fast
set -eo pipefail

if [ -z "$EXTERNAL_IP" ]; then
  cat >&2 <<EOF
error: \`-e "EXTERNAL_IP=your_external_ip"\` was not specified.
It is required so Tendermint instance can advertise its address to cluster participants
EOF
  exit 1
fi

DHOST=${DOCKER_HOST:-unix:///var/run/docker.sock}
if [[ $DHOST = unix://* ]]; then
  DSOCKET=${DHOST#unix://}
  if [ ! -S "$DSOCKET" ]; then
      cat >&2 <<EOF
error: '$DSOCKET' not found in container or is not a unix socket.
Please, pass it as a volume when running the container: \`-v /var/run/docker.sock:$DSOCKET\`
EOF
  exit 1
  fi
fi

CONTAINER_ID=$(cat /proc/1/cpuset)
CONTAINER_ID="${CONTAINER_ID#"/docker/"}"

ln -sf /application.conf /master/application.conf

# Execute whatever command is passed as arguments. Usually it's CMD from Dockerfile.
CONTAINER_ID=$CONTAINER_ID exec "$@"
