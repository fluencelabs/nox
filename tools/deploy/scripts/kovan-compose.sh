#!/bin/bash

# Uses to start Fluence node with Swarm and Ethereum node on Kovan chain.

set -e

USAGE="Usage: ./kovan-compose.sh <external-ip> <owner-address> <private-key>"

if [ ! $1 = '--help' -a ! $1 = '-h' ]; then
    if [ $# -eq 3 ]; then
        export PROD_DEPLOY='true'
        export CHAIN='kovan'
        export NAME='fluence-node'
        export PORTS='25000:25010'
        export CONTRACT_ADDRESS=$(cat contract.txt)
        export HOST_IP=$1
        export OWNER_ADDRESS=$2
        export PRIVATE_KEY=$3

        ./compose.sh
    else
        echo "Error: Not enough arguments."
        echo $USAGE
        exit 125
    fi
else
    echo $USAGE
fi
