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

#!/usr/bin/env bash -e

###
# This script is an entrypoint of the Master docker container.
# It first checks that $TENDERMINT_IP and $PORTS are defined,
# if /var/run/docker.sock is passed as a volume, and
# then if running master-node.jar, generate application.conf,
# and run whatever command is passed as arguments (usually it's CMD from Dockerfile).
###

# set to fail fast
set -e

if [ -z "$TENDERMINT_IP" ]; then
  cat >&2 <<EOF
error: \`-e "TENDERMINT_IP=your_external_ip"\` was not specified.
It is required so Tendermint instance can advertise its address to cluster participants
EOF
  exit 1
fi

if [ -z "$PORTS" ]; then
  cat >&2 <<EOF
error: \`-e "PORTS=start:stop"\` was not specified.
TODO: add more helpful explanation
EOF
  exit 1
fi

if [ ! -S /var/run/docker.sock ]; then
    cat >&2 <<EOF
error: '/var/run/docker.sock' not found in container or is not a socket.
Please, pass it as a volume when running the container: \`-v /var/run/docker.sock:/var/run/docker.sock\`
EOF
exit 1
fi

if [ -z "$ETHEREUM_IP" ]; then
    ETHEREUM_IP=$TENDERMINT_IP
fi

SWARM_ENABLED="true"
if [ -z "$SWARM_IP" ]; then
    SWARM_ENABLED="false"
fi

# Running master-node.jar, that means running default CMD
if [ "$3" = "/master-node.jar" ]; then
    CONTAINER_ID=$(cat /proc/1/cpuset)
    cat > "/master/application.conf" <<EOF
endpoints {
  ip = "$TENDERMINT_IP"
  min-port = ${PORTS%:*}
  max-port = ${PORTS#*:}
}
ethereum {
  host = "$ETHEREUM_IP"
}
swarm {
  host = "$SWARM_IP"
  enabled = "$SWARM_ENABLED"
}
tendermint-path = "/master"
master-container-id = "${CONTAINER_ID#"/docker/"}"
EOF
fi

# Execute whatever command is passed as arguments. Usually it's CMD from Dockerfile.
exec "$@"
