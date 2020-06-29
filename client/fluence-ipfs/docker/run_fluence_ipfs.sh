#!/usr/bin/env bash
# This script is intented to be used as an ENTRYPOINT in Docker container

set -euo pipefail
set -m # enable Bash job management to pass signals to children

BASH=$(command -v bash)
IPFS=$(command -v ipfs)

ipfs init --profile server >/dev/null || true
ipfs config --json Addresses.Announce \[\"/ip4/"$HOST"/tcp/4001\"\]

# Run Server & IPFS in parallel, fail if any of the processes fails
$BASH wait.sh \
  "./fluence-server $*" \
  "$IPFS daemon"
