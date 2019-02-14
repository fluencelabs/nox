#!/bin/bash

# Uses to start Fluence node with Swarm and Ethereum node on Kovan chain.

set -e

USAGE="Usage: ./kovan-compose.sh <contract> <external-ip> <owner-address> <private-key> <start-port:end-port>"

if [ ! $1 = '--help' -a ! $1 = '-h' ]; then

    if [ $# -eq 5 ]; then
        export PROD_DEPLOY='true'
        export CHAIN='kovan'
        export NAME='fluence-node-1'
        export CONTRACT_ADDRESS=$1
        export HOST_IP="$2"
        export OWNER_ADDRESS="$3"
        export PRIVATE_KEY="$4"
        export PORTS="$5"
        ./compose.sh
    else
        echo "Error: Not enough arguments."
        echo $USAGE
        exit 125
    fi
else
    echo $USAGE
fi
