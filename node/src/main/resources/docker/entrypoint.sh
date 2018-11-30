#!/usr/bin/env bash -e
set -e

if [ -z "$TENDERMINT_IP" ]; then
  cat >&2 <<EOF
error: \`-e "TENDERMINT_IP=your_external_ip"\` was not specified.
It is required so Tendermint instance can advertise its address to cluster participants
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

if [ "$3" = "/master-node.jar" ]; then
    CONTAINER_ID=$(cat /proc/1/cpuset)
    cat > "/master/application.conf" <<EOF
endpoints {
  ip = $TENDERMINT_IP
}
ethereum {
  protocol = http
  host = $TENDERMINT_IP
  port = 8545
}
tendermint-path=/master

master-container-id = ${CONTAINER_ID#"/docker/"}
EOF
fi
#/docker/68c8283d3b1396fb2dd7f9412fe0e474a19fe7506a197721071c644bf130c804

exec "$@"