#!/usr/bin/env bash
# This script is intented to be used as an ENTRYPOINT in Docker container

set -euo pipefail
set -m # enable Bash job management to pass signals to children

BASH=$(command -v bash)
IPFS=$(command -v ipfs)

ipfs init --profile server >/dev/null || true
# Announce public ip to the network, so other IPFS nodes know how to reach us
ipfs config --json Addresses.Announce \[\"/ip4/"$HOST"/tcp/4001\"\]
# Listen on all interfaces inside container (default is localhost)
ipfs config --json Addresses.API '"/ip4/0.0.0.0/tcp/5001/"'
# CORS
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Origin '["*"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Headers '["*"]'
ipfs config --json API.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "GET", "POST"]'
ipfs config --json Gateway.HTTPHeaders.Access-Control-Allow-Origin '["*"]'
ipfs config --json Gateway.HTTPHeaders.Access-Control-Allow-Headers '["*"]'
ipfs config --json Gateway.HTTPHeaders.Access-Control-Allow-Methods '["PUT", "GET", "POST"]'

# Run Server & IPFS in parallel, fail if any of the processes fails
$BASH wait.sh \
  "./particle-server $*" \
  "$IPFS daemon"
